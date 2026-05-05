// Passthrough streaming — forward raw SSE events with metrics collection

use std::collections::HashMap;
use std::time::Instant;

use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::gateway::error::ForwardRequestError;
use crate::log_codes::{STRM_DISCONNECT, STRM_DONE, STRM_EOF, STRM_ERROR, STRM_START};
use crate::state::AppState;

use super::copilot::{
    translate_openai_chunk_to_anthropic_events, AnthropicStreamState, CopilotStreamState,
};
use super::protocol::{
    convert_anthropic_to_openai, convert_anthropic_to_openai_responses, AnthropicToOpenAiOptions,
};
use super::utf8_utils::{append_utf8_safe, flush_utf8_remainder};
use super::{
    apply_anthropic_inbound_headers, apply_openai_inbound_headers, apply_provider_auth,
    deduplicate_url_path, estimate_input_tokens, extract_upstream_error_message,
    extract_usage_from_sse, find_sse_message_boundary, infer_event_type_from_data,
    is_reasoning_content_provider, record_stream_metrics, resolve_binding_provider_group,
    rx_to_sse_stream, sanitize_upstream_payload, summarize_payload,
    StreamMetricsInfo,
};

pub async fn forward_request_stream_passthrough(
    state: &AppState,
    model_name: &str,
    payload: Value,
    path: &str,
    inbound_headers: Option<&HeaderMap>,
) -> Result<
    Sse<
        std::pin::Pin<
            Box<dyn futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send>,
        >,
    >,
    ForwardRequestError,
> {
    let (binding, provider, group_name) = resolve_binding_provider_group(state, model_name)?;

    // Check circuit breaker before forwarding
    {
        let breaker = state
            .breaker
            .lock()
            .map_err(|_| ForwardRequestError::Upstream("circuit breaker lock error".to_string()))?;
        if breaker.is_open(&provider.id) {
            return Err(ForwardRequestError::Upstream(format!(
                "provider '{}' circuit is open, please try again later",
                provider.name
            )));
        }
    }

    let input_estimate = estimate_input_tokens(&payload);

    let metrics_info = StreamMetricsInfo {
        model_name: binding.model_name.clone(),
        group_name,
        provider_id: provider.id.clone(),
        input_price_per_1m: binding.input_price_per_1m,
        output_price_per_1m: binding.output_price_per_1m,
        input_estimate,
        started: Instant::now(),
    };

    let api_format = provider.api_format.as_deref().unwrap_or("anthropic");
    let needs_transform = api_format == "openai_chat" || api_format == "openai_responses";
    let path_normalized = path.trim().to_lowercase();

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

    // TODO(future): Add fine-grained streaming timeouts (first-byte, idle)
    //   following cc-switch's auto_failover model with StreamingTimeoutConfig.
    //   See cc-switch: proxy/handler_context.rs StreamingTimeoutConfig
    let client = state.http_client.clone();

    let mut req = client.post(&target_url).json(&payload);
    req = apply_provider_auth(req, &provider);
    req = req
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Accept-Encoding", "identity");
    if api_format == "anthropic" {
        req = apply_anthropic_inbound_headers(req, inbound_headers, "2023-06-01");
    } else if needs_transform {
        // OpenAI Chat / Responses 上游：透传 UA 与 x-stainless-*（与 copilot 转发一致，利于兼容代理）
        req = apply_openai_inbound_headers(req, inbound_headers);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?;

    let status = resp.status().as_u16();
    if status != 200 {
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?;
        let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(
            |_| serde_json::json!({"raw": String::from_utf8_lossy(&bytes).to_string()}),
        );
        let message = extract_upstream_error_message(&body, status);
        log::warn!(
            target: "octoswitch::gateway::forwarder::passthrough",
            "stream upstream rejected provider={} url={} status={} message={} payload={}",
            provider.name,
            target_url,
            status,
            message,
            summarize_payload(&payload)
        );
        return Err(ForwardRequestError::Upstream(message));
    }

    if needs_transform {
        // Transform OpenAI SSE chunks back to Anthropic SSE events
        let stream_state = CopilotStreamState {
            message_start_sent: false,
            thinking_block_open: false,
            content_block_open: false,
            content_block_index: 0,
            tool_calls: HashMap::new(),
            pending_tool_streams: HashMap::new(),
        };

        let stream = futures_util::stream::unfold(
            AnthropicStreamState {
                byte_stream: Box::pin(
                    resp.bytes_stream()
                        .map(|r| r.map(|b| b.to_vec()).map_err(|e| e.to_string())),
                ),
                sstate: stream_state,
                buffer: String::new(),
                utf8_remainder: Vec::new(),
                pending: std::collections::VecDeque::new(),
                done: false,
                metrics_info: Some(metrics_info),
                app_state: Some(state.clone()),
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
                chunk_count: 0,
                upstream_responses_api: false,
                copilot_response_id: String::new(),
                copilot_response_model: String::new(),
            },
            |mut state| async move {
                loop {
                    if let Some(event) = state.pending.pop_front() {
                        return Some((Ok(event), state));
                    }
                    if state.done {
                        if let (Some(info), Some(st)) =
                            (state.metrics_info.take(), state.app_state.take())
                        {
                            record_stream_metrics(
                                &st,
                                &info,
                                state.input_tokens,
                                state.output_tokens,
                                state.cache_creation_tokens,
                                state.cache_read_tokens,
                            );
                        }
                        return None;
                    }

                    match state.byte_stream.as_mut().next().await {
                        Some(Ok(data)) => {
                            state.chunk_count += 1;
                            append_utf8_safe(&mut state.buffer, &mut state.utf8_remainder, &data);

                            while let Some((pos, sep_len)) =
                                find_sse_message_boundary(&state.buffer)
                            {
                                let message = state.buffer[..pos].to_string();
                                state.buffer = state.buffer[pos + sep_len..].to_string();

                                for line in message.lines() {
                                    let data = line
                                        .strip_prefix("data: ")
                                        .or_else(|| line.strip_prefix("data:"));
                                    let Some(data) = data else { continue };
                                    let data = data.trim();

                                    if data == "[DONE]" {
                                        if !state.sstate.message_start_sent {
                                            state.pending.push_back(
                                                Event::default()
                                                    .event("message_start")
                                                    .data(
                                                        serde_json::to_string(&serde_json::json!({
                                                            "type": "message_start",
                                                            "message": {
                                                                "id": "",
                                                                "type": "message",
                                                                "role": "assistant",
                                                                "content": [],
                                                                "model": "",
                                                                "stop_reason": null,
                                                                "stop_sequence": null,
                                                                "usage": { "input_tokens": 0, "output_tokens": 0 },
                                                            }
                                                        }))
                                                        .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                                                    ),
                                            );
                                            state.sstate.message_start_sent = true;
                                        }
                                        state.pending.push_back(
                                            Event::default().event("message_stop").data(
                                                serde_json::to_string(&serde_json::json!({
                                                    "type": "message_stop"
                                                }))
                                                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                                            ),
                                        );
                                        state.done = true;
                                        break;
                                    }

                                    let Ok(chunk) = serde_json::from_str::<Value>(data) else {
                                        continue;
                                    };

                                    extract_usage_from_sse(
                                        &chunk,
                                        &mut state.input_tokens,
                                        &mut state.output_tokens,
                                        &mut state.cache_creation_tokens,
                                        &mut state.cache_read_tokens,
                                    );

                                    let events = translate_openai_chunk_to_anthropic_events(
                                        &chunk,
                                        &mut state.sstate,
                                    );
                                    state.pending.extend(events);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::error!(
                                "[{STRM_ERROR}] transform stream error after {} chunks: {e}",
                                state.chunk_count
                            );
                            flush_utf8_remainder(&mut state.buffer, &mut state.utf8_remainder);
                            state.done = true;
                            if state.pending.is_empty() {
                                return None;
                            }
                        }
                        None => {
                            log::debug!(
                                "[{STRM_EOF}] transform stream EOF after {} chunks",
                                state.chunk_count
                            );
                            flush_utf8_remainder(&mut state.buffer, &mut state.utf8_remainder);
                            state.done = true;
                            if state.pending.is_empty() {
                                return None;
                            }
                        }
                    }
                }
            },
        );

        return Ok(Sse::new(Box::pin(stream)));
    }

    // Passthrough mode: forward raw SSE events as-is
    let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(32);
    let state_clone = state.clone();
    let provider_id = metrics_info.provider_id.clone();

    // Spawned task self-terminates: when the SSE stream ends, errors, or the client
    // disconnects (rx dropped → tx.send().is_err()), the task returns naturally.
    // A global cancel token would be needed for full app-shutdown abort.
    let _handle = tokio::spawn(async move {
        let mut byte_stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut utf8_remainder: Vec<u8> = Vec::new();
        let mut input_tokens: i64 = 0;
        let mut output_tokens: i64 = 0;
        let mut cache_creation_tokens: i64 = 0;
        let mut cache_read_tokens: i64 = 0;
        let mut chunk_count: u64 = 0;

        log::debug!(
            "[{STRM_START}] passthrough stream started model={} provider={}",
            metrics_info.model_name,
            provider_id
        );

        loop {
            match byte_stream.next().await {
                Some(Ok(bytes)) => {
                    chunk_count += 1;
                    append_utf8_safe(&mut buffer, &mut utf8_remainder, &bytes);

                    while let Some((pos, sep_len)) = find_sse_message_boundary(&buffer) {
                        let message = buffer[..pos].to_string();
                        buffer = buffer[pos + sep_len..].to_string();

                        let mut event_type: Option<String> = None;
                        let mut data_lines: Vec<String> = Vec::new();

                        for line in message.lines() {
                            if let Some(evt) = line.strip_prefix("event: ") {
                                event_type = Some(evt.trim().to_string());
                            } else if let Some(data) = line
                                .strip_prefix("data: ")
                                .or_else(|| line.strip_prefix("data:"))
                            {
                                let data = data.trim();
                                if data == "[DONE]" {
                                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                    log::debug!("[{STRM_DONE}] passthrough stream completed chunks={chunk_count} in={input_tokens} out={output_tokens} cc={cache_creation_tokens} cr={cache_read_tokens}");
                                    record_stream_metrics(
                                        &state_clone,
                                        &metrics_info,
                                        input_tokens,
                                        output_tokens,
                                        cache_creation_tokens,
                                        cache_read_tokens,
                                    );
                                    {
                                        if let Ok(mut breaker) = state_clone.breaker.lock() {
                                            breaker.mark_success(&provider_id);
                                        }
                                    }
                                    return;
                                }
                                data_lines.push(data.to_string());
                            }
                        }

                        if !data_lines.is_empty() {
                            for data in &data_lines {
                                if let Ok(v) = serde_json::from_str::<Value>(data) {
                                    extract_usage_from_sse(
                                        &v,
                                        &mut input_tokens,
                                        &mut output_tokens,
                                        &mut cache_creation_tokens,
                                        &mut cache_read_tokens,
                                    );
                                }
                            }

                            if event_type.is_none() {
                                for data in &data_lines {
                                    if let Some(inferred) = infer_event_type_from_data(data) {
                                        event_type = Some(inferred);
                                        break;
                                    }
                                }
                            }

                            let mut event = Event::default();
                            if let Some(et) = &event_type {
                                event = event.event(et);
                            }
                            event = event.data(data_lines.join("\n"));
                            if tx.send(Ok(event)).await.is_err() {
                                log::info!("[{STRM_DISCONNECT}] passthrough client disconnected after {chunk_count} chunks");
                                record_stream_metrics(
                                    &state_clone,
                                    &metrics_info,
                                    input_tokens,
                                    output_tokens,
                                    cache_creation_tokens,
                                    cache_read_tokens,
                                );
                                {
                                    if let Ok(mut breaker) = state_clone.breaker.lock() {
                                        breaker.mark_success(&provider_id);
                                    }
                                }
                                return;
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                    log::error!(
                        "[{STRM_ERROR}] passthrough stream error after {chunk_count} chunks: {e}"
                    );
                    record_stream_metrics(
                        &state_clone,
                        &metrics_info,
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens,
                    );
                    {
                        if let Ok(mut breaker) = state_clone.breaker.lock() {
                            breaker.mark_failure(&provider_id);
                        }
                    }
                    return;
                }
                None => {
                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                    log::debug!("[{STRM_EOF}] passthrough stream EOF after {chunk_count} chunks in={input_tokens} out={output_tokens} cc={cache_creation_tokens} cr={cache_read_tokens}");
                    record_stream_metrics(
                        &state_clone,
                        &metrics_info,
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens,
                    );
                    {
                        if let Ok(mut breaker) = state_clone.breaker.lock() {
                            breaker.mark_success(&provider_id);
                        }
                    }
                    return;
                }
            }
        }
    });

    Ok(Sse::new(Box::pin(rx_to_sse_stream(rx))))
}
