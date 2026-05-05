// Non-streaming request forwarding

use std::time::{Duration, Instant};

use axum::http::HeaderMap;
use serde_json::Value;
use tokio::time::sleep;

use crate::{
    gateway::error::ForwardRequestError,
    log_codes::{FWD_DONE, FWD_RETRY, FWD_RETRY_EXH, FWD_START},
    state::AppState,
};

use super::copilot_request;
use super::protocol::{
    convert_anthropic_to_openai, convert_anthropic_to_openai_responses,
    convert_openai_responses_json_to_chat_completion, convert_openai_responses_to_anthropic,
    convert_openai_to_anthropic, AnthropicToOpenAiOptions,
};
use super::{
    apply_anthropic_inbound_headers, apply_openai_inbound_headers, apply_provider_auth,
    build_copilot_headers, deduplicate_url_path, estimate_input_tokens,
    extract_upstream_error_message, has_copilot_account, is_reasoning_content_provider,
    parse_tokens_from_upstream_usage, record_request_metrics, resolve_binding_provider_group,
    sanitize_upstream_payload, status_is_retryable,
    summarize_payload,
};

pub async fn forward_request(
    state: &AppState,
    model_name: &str,
    payload: Value,
    path: &str,
    inbound_headers: Option<&HeaderMap>,
) -> Result<(u16, Value), ForwardRequestError> {
    let (binding, provider, group_name) = resolve_binding_provider_group(state, model_name)?;

    {
        let breaker = state
            .breaker
            .lock()
            .map_err(|_| ForwardRequestError::Upstream("Circuit breaker lock error".to_string()))?;
        if breaker.is_open(&provider.id) {
            log::warn!(
                "[{}] request rejected: provider '{}' circuit open",
                crate::log_codes::CB_OPEN,
                provider.name
            );
            return Err(ForwardRequestError::Upstream(format!(
                "Provider '{}' circuit breaker is open, please retry later",
                provider.name
            )));
        }
    }

    let path_normalized = path.trim().to_lowercase();

    // ── Copilot path ──
    if has_copilot_account(state, &provider.id) {
        let ctx = copilot_request::prepare_copilot_request_for_provider(
            state,
            &provider,
            &binding,
            group_name.clone(),
            path,
            inbound_headers,
        )
        .await?;

        let copilot_path = if ctx.use_responses_upstream {
            "/v1/responses"
        } else {
            "/chat/completions"
        };
        let target_url = format!("{}/{}", ctx.base_url, copilot_path.trim_start_matches('/'));

        let payload = copilot_request::transform_copilot_payload(
            payload,
            &ctx.binding.upstream_model_name,
            ctx.anthropic_inbound,
            ctx.openai_chat_inbound,
            ctx.use_responses_upstream,
        );

        // Estimate tokens BEFORE conversion to avoid cloning the payload.
        let input_estimate = estimate_input_tokens(&payload);

        let started = Instant::now();
        let client = state.http_client.clone();

        let (copilot_headers, _request_id, editor_device_id) =
            build_copilot_headers(&ctx.copilot_token);
        let mut req = client.post(&target_url).json(&payload);
        for (name, value) in &copilot_headers {
            req = req.header(name, value);
        }
        req = req.header("Accept", "application/json");
        req = req.header("Editor-Device-Id", &editor_device_id);

        if ctx.anthropic_inbound {
            req = apply_anthropic_inbound_headers(req, inbound_headers, "2023-06-01");
        } else {
            req = apply_openai_inbound_headers(req, inbound_headers);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?;
        let status = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?;
        let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(
            |_| serde_json::json!({"raw": String::from_utf8_lossy(&bytes).to_string()}),
        );

        let final_body = if ctx.anthropic_inbound {
            if ctx.use_responses_upstream {
                convert_openai_responses_to_anthropic(&body)
            } else {
                convert_openai_to_anthropic(&body)
            }
        } else if ctx.openai_chat_inbound && ctx.use_responses_upstream {
            convert_openai_responses_json_to_chat_completion(&body)
        } else {
            body.clone()
        };

        let usage_tokens = match parse_tokens_from_upstream_usage(&body) {
            Some(tokens) => tokens,
            None => super::UsageTokens {
                input_tokens: input_estimate,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
            },
        };
        record_request_metrics(
            state,
            &ctx.binding.model_name,
            &ctx.group_name,
            &ctx.provider.id,
            status,
            started.elapsed().as_millis() as i64,
            &super::MetricTokens {
                input_tokens: usage_tokens.input_tokens,
                output_tokens: usage_tokens.output_tokens,
                cache_creation_tokens: usage_tokens.cache_creation_tokens,
                cache_read_tokens: usage_tokens.cache_read_tokens,
                input_price_per_1m: ctx.binding.input_price_per_1m,
                output_price_per_1m: ctx.binding.output_price_per_1m,
            },
        );

        return Ok((status, final_body));
    }

    // ── Standard (non-Copilot) path ──
    let api_format = provider.api_format.as_deref().unwrap_or("anthropic");
    let needs_transform = api_format == "openai_chat" || api_format == "openai_responses";

    // Determine effective endpoint based on api_format
    let effective_path = if needs_transform {
        if path_normalized.contains("/v1/messages") {
            if api_format == "openai_responses" {
                "/v1/responses"
            } else {
                "/v1/chat/completions"
            }
        } else {
            path
        }
    } else {
        path
    };

    let target_url = deduplicate_url_path(&format!(
        "{}/{}",
        provider.base_url.trim_end_matches('/'),
        effective_path.trim_start_matches('/')
    ));

    let mut payload = payload;
    if path_normalized.contains("/v1/messages")
        || path_normalized.contains("/v1/chat/completions")
        || path_normalized.contains("/v1/responses")
    {
        payload["model"] = Value::String(binding.upstream_model_name.clone());
    }

    // Transform request body if api_format requires it
    if needs_transform && path_normalized.contains("/v1/messages") {
        if api_format == "openai_chat" {
            let preserve_rc = is_reasoning_content_provider(&provider.base_url, &binding.upstream_model_name);
            payload = convert_anthropic_to_openai(&payload, AnthropicToOpenAiOptions {
                preserve_reasoning_content: preserve_rc,
                ..Default::default()
            });
        } else if api_format == "openai_responses" {
            payload = convert_anthropic_to_openai_responses(&payload);
        }
    }
    sanitize_upstream_payload(&provider, path, &mut payload);

    let started = Instant::now();
    // TODO(future): Add non-streaming timeout following cc-switch's auto_failover model.
    //   See cc-switch: proxy/forwarder.rs non_streaming_timeout
    let client = state.http_client.clone();

    let max_attempts = (provider.max_retries.max(0) as usize)
        .saturating_add(1)
        .max(1);

    let mut final_status_body: Option<(u16, Value)> = None;
    let mut final_transport_err: Option<String> = None;

    for attempt in 0..max_attempts {
        if attempt > 0 {
            sleep(Duration::from_millis(200 + 150 * attempt as u64)).await;
        }

        log::debug!(
            "[{}] forwarding model={} attempt {}/{}",
            FWD_START,
            binding.model_name,
            attempt + 1,
            max_attempts
        );

        let mut req = client.post(&target_url).json(&payload);
        req = apply_provider_auth(req, &provider);
        if api_format == "anthropic" {
            req = apply_anthropic_inbound_headers(req, inbound_headers, "2023-06-01");
        } else if needs_transform {
            req = apply_openai_inbound_headers(req, inbound_headers);
        }

        match req.send().await {
            Ok(r) => {
                let status = r.status().as_u16();
                let bytes = match r.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        final_transport_err = Some(e.to_string());
                        if attempt + 1 < max_attempts {
                            log::warn!("[{}] transport error, retrying: {}", FWD_RETRY, e);
                            continue;
                        }
                        break;
                    }
                };

                let body = match serde_json::from_slice::<Value>(&bytes) {
                    Ok(v) => v,
                    Err(_) => {
                        if status_is_retryable(status) && attempt + 1 < max_attempts {
                            log::warn!("[{}] retryable status {status}", FWD_RETRY);
                            final_transport_err = Some(format!(
                                "upstream returned non-json body with retryable status {status}"
                            ));
                            continue;
                        }
                        serde_json::json!({"raw": String::from_utf8_lossy(&bytes).to_string()})
                    }
                };
                if status_is_retryable(status) && attempt + 1 < max_attempts {
                    log::warn!("[{}] retryable status {status}", FWD_RETRY);
                    continue;
                }
                if status >= 400 {
                    log::warn!(
                        target: "octoswitch::gateway::forwarder::non_streaming",
                        "upstream rejected provider={} url={} status={} message={} payload={}",
                        provider.name,
                        target_url,
                        status,
                        extract_upstream_error_message(&body, status),
                        summarize_payload(&payload)
                    );
                }
                // Transform response back to Anthropic format if api_format requires it
                let final_body = if needs_transform {
                    if api_format == "openai_chat" {
                        convert_openai_to_anthropic(&body)
                    } else if api_format == "openai_responses" {
                        convert_openai_responses_to_anthropic(&body)
                    } else {
                        body
                    }
                } else {
                    body
                };
                final_status_body = Some((status, final_body));
                break;
            }
            Err(e) => {
                if attempt + 1 < max_attempts {
                    log::warn!("[{}] transport error, retrying: {}", FWD_RETRY, e);
                    final_transport_err = Some(e.to_string());
                    continue;
                }
                final_transport_err = Some(e.to_string());
                break;
            }
        }
    }

    let (status, body) = match final_status_body {
        Some(pair) => pair,
        None => {
            log::error!("[{}] all {max_attempts} attempts failed", FWD_RETRY_EXH);
            let msg = final_transport_err.unwrap_or_else(|| "Upstream request failed".to_string());
            let mut breaker = state.breaker.lock().map_err(|_| {
                ForwardRequestError::Upstream("Circuit breaker lock error".to_string())
            })?;
            breaker.mark_failure(&provider.id);
            return Err(ForwardRequestError::Upstream(msg));
        }
    };

    {
        let mut breaker = state
            .breaker
            .lock()
            .map_err(|_| ForwardRequestError::Upstream("Circuit breaker lock error".to_string()))?;
        if status >= 400 {
            breaker.mark_failure(&provider.id);
        } else {
            breaker.mark_success(&provider.id);
        }
    }

    let usage_tokens = match parse_tokens_from_upstream_usage(&body) {
        Some(tokens) => tokens,
        None => super::UsageTokens {
            input_tokens: estimate_input_tokens(&payload),
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        },
    };
    record_request_metrics(
        state,
        &binding.model_name,
        &group_name,
        &provider.id,
        status,
        started.elapsed().as_millis() as i64,
        &super::MetricTokens {
            input_tokens: usage_tokens.input_tokens,
            output_tokens: usage_tokens.output_tokens,
            cache_creation_tokens: usage_tokens.cache_creation_tokens,
            cache_read_tokens: usage_tokens.cache_read_tokens,
            input_price_per_1m: binding.input_price_per_1m,
            output_price_per_1m: binding.output_price_per_1m,
        },
    );

    log::info!(
        "[{}] model={} status={} latency={}ms",
        FWD_DONE,
        binding.model_name,
        status,
        started.elapsed().as_millis()
    );

    Ok((status, body))
}
