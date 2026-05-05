// Forwarder module — request forwarding to upstream LLM providers.
// Split into sub-modules for maintainability.

mod copilot;
mod copilot_request;
mod non_streaming;
mod passthrough;
mod protocol;
mod utf8_utils;

pub use copilot::{
    forward_request_copilot_stream, forward_request_copilot_stream_openai, get_provider_for_model,
};
pub use non_streaming::forward_request;
pub use passthrough::forward_request_stream_passthrough;

use std::time::Instant;

use axum::http::HeaderMap;
use axum::response::sse::Event;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{
    config::app_config::load_gateway_config,
    database::copilot_account_dao,
    database::model_group_dao,
    domain::provider::Provider,
    gateway::error::ForwardRequestError,
    service::routing_service,
    services::copilot_headers,
    services::metrics_collector::{self, RequestMetricInput},
    state::AppState,
};

/// Apply authentication headers based on the provider's `auth_mode`.
/// - `"anthropic_api_key"`: sends `x-api-key` header (Anthropic official API convention)
/// - `"bearer"` (default): sends `Authorization: Bearer` header
fn apply_provider_auth(
    mut req: reqwest::RequestBuilder,
    provider: &Provider,
) -> reqwest::RequestBuilder {
    if provider.auth_mode == "anthropic_api_key" {
        req = req.header("x-api-key", &provider.api_key_ref);
    } else {
        req = req.header("Authorization", format!("Bearer {}", provider.api_key_ref));
    }
    req
}

/// Check if a provider has an associated Copilot account.
pub(crate) fn has_copilot_account(state: &AppState, provider_id: &str) -> bool {
    let conn = match state.db.get() {
        Ok(c) => c,
        Err(_) => return false,
    };
    copilot_account_dao::get_by_provider(&conn, provider_id)
        .ok()
        .flatten()
        .is_some()
}

/// Resolves model binding, provider, and group name for a given client `model` string.
/// 模型名解析见 `routing_service::resolve_model_binding`：默认可用 `分组/绑定路由名`，可在网关设置中关闭。
fn resolve_binding_provider_group(
    state: &AppState,
    model_name: &str,
) -> Result<
    (
        crate::domain::model_binding::ModelBinding,
        crate::domain::provider::Provider,
        Option<String>,
    ),
    ForwardRequestError,
> {
    use crate::domain::model_binding::ModelBinding;
    use crate::domain::provider::Provider;

    let conn = state
        .db
        .get()
        .map_err(|_| ForwardRequestError::Upstream("Database connection error".to_string()))?;
    let trim = model_name.trim();
    let group_lookup_key = trim.split_once('/').map(|(a, _)| a.trim()).unwrap_or(trim);
    let group_name: Option<String> = model_group_dao::get_by_alias_ci(&conn, group_lookup_key)
        .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?
        .map(|g| g.alias);
    let gw = load_gateway_config();
    let binding: ModelBinding = routing_service::resolve_model_binding(
        &conn,
        model_name,
        gw.allow_group_member_model_path,
    )?;
    let provider: Provider = match routing_service::get_provider(&conn, &binding.provider_id)? {
        Some(p) => p,
        None => {
            return Err(ForwardRequestError::ProviderNotFound {
                provider_id: binding.provider_id.clone(),
            });
        }
    };
    if !provider.is_enabled {
        return Err(ForwardRequestError::ProviderDisabled {
            name: provider.name.clone(),
        });
    }
    Ok((binding, provider, group_name))
}

/// Re-export for `copilot` / `non_streaming` submodules.
pub(super) fn build_copilot_headers(token: &str) -> (reqwest::header::HeaderMap, String, String) {
    copilot_headers::build_copilot_headers(token)
}

/// Core metrics recording: acquires pool connections and writes to DB + in-memory aggregator.
/// Silently returns on connection errors to avoid disrupting the response path.
fn do_record_metric(state: &AppState, input: RequestMetricInput) {
    let conn = match state.db.get() {
        Ok(c) => c,
        Err(_) => {
            log::error!(
                "[{}] failed to acquire db connection for metric recording",
                crate::log_codes::DB_LOCK_SKIP
            );
            return;
        }
    };
    let mut metrics = match state.metrics.lock() {
        Ok(m) => m,
        Err(_) => {
            log::error!(
                "[{}] failed to acquire metrics lock",
                crate::log_codes::MET_LOCK_SKIP
            );
            return;
        }
    };
    let _ = metrics_collector::record_request_metric(&conn, &mut metrics, input);
}

/// Bundle of token counts for a single request.
struct MetricTokens {
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
}

/// Record a request metric to the database and metrics aggregator.
fn record_request_metrics(
    state: &AppState,
    model_name: &str,
    group_name: &Option<String>,
    provider_id: &str,
    status_code: u16,
    latency_ms: i64,
    tokens: &MetricTokens,
) {
    do_record_metric(
        state,
        RequestMetricInput {
            model_name: model_name.to_string(),
            group_name: group_name.clone(),
            provider_id: provider_id.to_string(),
            status_code: status_code as i64,
            latency_ms,
            input_tokens: tokens.input_tokens,
            output_tokens: tokens.output_tokens,
            cache_creation_input_tokens: tokens.cache_creation_tokens,
            cache_read_input_tokens: tokens.cache_read_tokens,
        },
    );
}

fn status_is_retryable(status: u16) -> bool {
    matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

fn value_to_i64(v: &Value) -> Option<i64> {
    if let Some(i) = v.as_i64() {
        return Some(i);
    }
    if let Some(u) = v.as_u64() {
        // Cap u64 values that exceed i64::MAX instead of returning None.
        // This can happen with JSON numbers larger than 2^63-1.
        return Some(i64::try_from(u).unwrap_or(i64::MAX));
    }
    if let Some(f) = v.as_f64() {
        // Note: f64 → i64 rounds to nearest integer.
        // Values outside the i64 range are clamped rather than discarded.
        if f.is_finite() {
            return Some(f.round() as i64);
        }
    }
    if let Some(s) = v.as_str() {
        if let Ok(i) = s.parse::<i64>() {
            return Some(i);
        }
    }
    None
}

fn read_usage_value(usage: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|k| usage.get(*k).and_then(value_to_i64))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UsageTokens {
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
}

fn parse_usage_tokens(usage: &Value) -> Option<UsageTokens> {
    let anthropic_input_tokens = read_usage_value(usage, &["input_tokens"]);
    let openai_prompt_tokens = read_usage_value(usage, &["prompt_tokens"]);
    let other_input_tokens = read_usage_value(usage, &["promptTokenCount", "inputTokenCount"]);
    let raw_input_tokens = anthropic_input_tokens
        .or(openai_prompt_tokens)
        .or(other_input_tokens);
    let cache_creation_tokens = read_usage_value(
        usage,
        &["cache_creation_input_tokens", "cache_creation_tokens"],
    )
    .unwrap_or(0)
    .max(0);
    let cache_read_tokens =
        read_usage_value(usage, &["cache_read_input_tokens", "cache_read_tokens"])
            .or_else(|| {
                usage
                    .get("prompt_tokens_details")
                    .and_then(|d| d.get("cached_tokens"))
                    .and_then(value_to_i64)
            })
            .unwrap_or(0)
            .max(0);

    let mut output_tokens = read_usage_value(
        usage,
        &[
            "output_tokens",
            "completion_tokens",
            "candidatesTokenCount",
            "outputTokenCount",
        ],
    );

    if output_tokens.is_none() {
        let total = read_usage_value(usage, &["totalTokenCount", "total_tokens"]);
        if let (Some(total), Some(raw_input)) = (total, raw_input_tokens) {
            output_tokens = Some((total - raw_input).max(0));
        }
    }

    if raw_input_tokens.is_none()
        && output_tokens.is_none()
        && cache_creation_tokens == 0
        && cache_read_tokens == 0
    {
        return None;
    }

    Some(UsageTokens {
        input_tokens: if anthropic_input_tokens.is_some() || other_input_tokens.is_some() {
            raw_input_tokens.unwrap_or(0).max(0)
        } else if openai_prompt_tokens.is_some() {
            (raw_input_tokens.unwrap_or(0) - cache_read_tokens).max(0)
        } else {
            raw_input_tokens.unwrap_or(0).max(0)
        },
        output_tokens: output_tokens.unwrap_or(0).max(0),
        cache_creation_tokens,
        cache_read_tokens,
    })
}

fn parse_tokens_from_upstream_usage(body: &Value) -> Option<UsageTokens> {
    let usage = body.get("usage").or_else(|| body.get("usageMetadata"))?;
    parse_usage_tokens(usage)
}

fn merge_usage_tokens(
    parsed: UsageTokens,
    input_tokens: &mut i64,
    output_tokens: &mut i64,
    cache_creation_tokens: &mut i64,
    cache_read_tokens: &mut i64,
) {
    *input_tokens = (*input_tokens).max(parsed.input_tokens);
    *output_tokens = (*output_tokens).max(parsed.output_tokens);
    *cache_creation_tokens = (*cache_creation_tokens).max(parsed.cache_creation_tokens);
    *cache_read_tokens = (*cache_read_tokens).max(parsed.cache_read_tokens);
}

fn estimate_input_tokens(payload: &Value) -> i64 {
    fn estimate_from_char_count(chars: i64) -> i64 {
        // 粗估：英文约 4 chars/token，中文更密；这里仅作为 usage 缺失时兜底。
        ((chars.max(0) as f64) / 4.0).ceil() as i64
    }

    fn collect_text_len(v: &Value) -> i64 {
        match v {
            Value::String(s) => s.chars().count() as i64,
            Value::Array(arr) => arr.iter().map(collect_text_len).sum(),
            Value::Object(map) => {
                let mut sum = 0_i64;
                if let Some(text) = map.get("text") {
                    sum += collect_text_len(text);
                }
                if let Some(content) = map.get("content") {
                    sum += collect_text_len(content);
                }
                sum
            }
            _ => 0,
        }
    }

    if let Some(messages) = payload.get("messages").and_then(|m| m.as_array()) {
        let chars: i64 = messages.iter().map(collect_text_len).sum();
        let msg_overhead = messages.len() as i64 * 3;
        return estimate_from_char_count(chars).max(0) + msg_overhead;
    }

    if let Some(prompt) = payload.get("prompt") {
        return estimate_from_char_count(collect_text_len(prompt)).max(0);
    }

    0
}

/// 将客户端（如 Claude Code）的 Anthropic 相关头透传到上游；部分云厂商会依此识别允许的 Coding Agent 客户端。
fn apply_anthropic_inbound_headers(
    mut req: reqwest::RequestBuilder,
    inbound_headers: Option<&HeaderMap>,
    default_version: &str,
) -> reqwest::RequestBuilder {
    let version = inbound_headers
        .and_then(|h| h.get("anthropic-version"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_version);
    req = req.header("anthropic-version", version);

    // 注入 claude-code-20250219 beta 标记（参考 cc-switch 行为）
    // DashScope 等上游依此识别 Claude Code 客户端，若请求已有该标记则去重合并
    const CLAUDE_CODE_BETA: &str = "claude-code-20250219";
    let existing_beta = inbound_headers
        .and_then(|h| h.get("anthropic-beta"))
        .and_then(|v| v.to_str().ok());
    let beta_value = match existing_beta {
        Some(beta_str) if beta_str.contains(CLAUDE_CODE_BETA) => beta_str.to_string(),
        Some(beta_str) => format!("{CLAUDE_CODE_BETA},{beta_str}"),
        None => CLAUDE_CODE_BETA.to_string(),
    };
    req = req.header("anthropic-beta", &beta_value);

    if let Some(hmap) = inbound_headers {
        for name in ["user-agent", "anthropic-dangerous-direct-browser-access"] {
            if let Some(v) = hmap.get(name) {
                if let Ok(s) = v.to_str() {
                    req = req.header(name, s);
                }
            }
        }
        for (name, value) in hmap.iter() {
            let n = name.as_str();
            if n.starts_with("x-stainless-") {
                if let Ok(s) = value.to_str() {
                    req = req.header(n, s);
                }
            }
        }
    }
    req
}

/// 入站为 OpenAI/Codex（`/v1/chat/completions` 等）时透传客户端指纹头。
/// 不向 GitHub Copilot 注入 `anthropic-*`，否则上游可能不按 SSE 流式返回。
pub(super) fn apply_openai_inbound_headers(
    mut req: reqwest::RequestBuilder,
    inbound_headers: Option<&HeaderMap>,
) -> reqwest::RequestBuilder {
    if let Some(hmap) = inbound_headers {
        if let Some(ua) = hmap.get("user-agent").and_then(|v| v.to_str().ok()) {
            if !ua.trim().is_empty() {
                req = req.header("user-agent", ua);
            }
        }
        for (name, value) in hmap.iter() {
            let n = name.as_str();
            let nl = n.to_ascii_lowercase();
            if nl.starts_with("x-stainless-") || nl == "openai-beta" {
                if let Ok(s) = value.to_str() {
                    req = req.header(n, s);
                }
            }
        }
    }
    req
}

pub(super) fn summarize_payload(payload: &Value) -> String {
    let keys = payload
        .as_object()
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
        .unwrap_or_default();
    let model = payload.get("model").and_then(|v| v.as_str()).unwrap_or("");
    let stream = payload
        .get("stream")
        .and_then(|v| v.as_bool())
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let temperature = payload
        .get("temperature")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let max_tokens = payload
        .get("max_tokens")
        .or_else(|| payload.get("max_output_tokens"))
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let tools = payload
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|v| v.len().to_string())
        .unwrap_or_else(|| "-".to_string());
    let tool_choice = payload
        .get("tool_choice")
        .and_then(|v| {
            v.get("type")
                .and_then(|t| t.as_str())
                .map(str::to_string)
                .or_else(|| v.as_str().map(str::to_string))
        })
        .unwrap_or_else(|| "-".to_string());
    let thinking = payload
        .get("thinking")
        .and_then(|v| v.get("type").and_then(|t| t.as_str()))
        .unwrap_or("-");

    format!(
        "keys=[{keys}] model={model} stream={stream} max_tokens={max_tokens} temperature={temperature} tools={tools} tool_choice={tool_choice} thinking={thinking}"
    )
}

pub(super) fn sanitize_upstream_payload(provider: &Provider, path: &str, payload: &mut Value) {
    let path_normalized = path.trim().to_ascii_lowercase();
    let provider_name = provider.name.to_ascii_lowercase();
    let provider_base = provider.base_url.to_ascii_lowercase();

    let is_minimax_anthropic = provider_base.contains("api.minimaxi.com")
        && provider_base.contains("/anthropic")
        && provider.api_format.as_deref().unwrap_or("anthropic") == "anthropic"
        && path_normalized.contains("/v1/messages");

    if !is_minimax_anthropic {
        return;
    }

    if let Some(obj) = payload.as_object_mut() {
        // MiniMax 的 Anthropic 兼容口会拒绝 Claude Code 带上的扩展字段。
        obj.remove("output_config");

        if obj
            .get("tools")
            .and_then(|v| v.as_array())
            .is_some_and(|v| v.is_empty())
        {
            obj.remove("tools");
        }

        if obj
            .get("metadata")
            .and_then(|v| v.as_object())
            .is_some_and(|v| v.is_empty())
        {
            obj.remove("metadata");
        }
    }

    log::debug!(
        target: "octoswitch::gateway::forwarder",
        "sanitized MiniMax anthropic payload provider={} payload={}",
        if provider_name.is_empty() { &provider.name } else { &provider.name },
        summarize_payload(payload)
    );
}

/// Check whether the provider or model requires `reasoning_content` preservation
/// (DeepSeek / Moonshot / Kimi). These providers require `reasoning_content` in
/// assistant messages for tool-call round-trips.
pub(super) fn is_reasoning_content_provider(base_url: &str, model: &str) -> bool {
    let value = format!("{base_url} {model}").to_ascii_lowercase();
    value.contains("deepseek") || value.contains("moonshot") || value.contains("kimi")
}

/// Normalize a constructed URL by removing consecutive duplicate path segments.
/// e.g. `https://host/v1/v1/chat/completions` → `https://host/v1/chat/completions`
pub(super) fn deduplicate_url_path(url: &str) -> String {
    if let Some((scheme_rest, path)) = url.split_once("://") {
        let path_start = path.find('/').unwrap_or(path.len());
        let host = &path[..path_start];
        let path_part = &path[path_start..];
        let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
        let mut deduped: Vec<&str> = Vec::new();
        for seg in segments {
            if deduped.last() != Some(&seg) {
                deduped.push(seg);
            }
        }
        if deduped.is_empty() {
            format!("{}://{}", scheme_rest, host)
        } else {
            format!("{}://{}/{}", scheme_rest, host, deduped.join("/"))
        }
    } else {
        url.to_string()
    }
}

pub(super) fn extract_upstream_error_message(body: &Value, status: u16) -> String {
    body.get("error")
        .and_then(|e| {
            e.as_str()
                .map(String::from)
                .or_else(|| e.get("message").and_then(|m| m.as_str()).map(String::from))
        })
        .or_else(|| {
            body.get("message")
                .and_then(|m| m.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| format!("Upstream returned status {status}"))
}

// ── Streaming helpers ──

struct StreamMetricsInfo {
    model_name: String,
    group_name: Option<String>,
    provider_id: String,
    input_estimate: i64,
    started: Instant,
}

/// Extract usage tokens from an SSE data payload (handles both OpenAI and Anthropic formats).
fn extract_usage_from_sse(
    data: &Value,
    input_tokens: &mut i64,
    output_tokens: &mut i64,
    cache_creation_tokens: &mut i64,
    cache_read_tokens: &mut i64,
) {
    // OpenAI format: {"usage":{"prompt_tokens":N,"completion_tokens":M}}
    if let Some(usage) = data.get("usage") {
        if let Some(usage) = parse_usage_tokens(usage) {
            merge_usage_tokens(
                usage,
                input_tokens,
                output_tokens,
                cache_creation_tokens,
                cache_read_tokens,
            );
        }
    }
    // Anthropic format: message_start → message.usage.input_tokens
    if data.get("type").and_then(|t| t.as_str()) == Some("message_start") {
        if let Some(msg_usage) = data.get("message").and_then(|m| m.get("usage")) {
            if let Some(usage) = parse_usage_tokens(msg_usage) {
                merge_usage_tokens(
                    usage,
                    input_tokens,
                    output_tokens,
                    cache_creation_tokens,
                    cache_read_tokens,
                );
            }
        }
    }
    // Anthropic format: message_delta → usage.output_tokens
    if data.get("type").and_then(|t| t.as_str()) == Some("message_delta") {
        if let Some(ot) = data
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(value_to_i64)
        {
            *output_tokens = (*output_tokens).max(ot);
        }
    }
    // OpenAI Responses API stream: `response.completed` (GitHub Copilot / Codex)
    if data.get("type").and_then(|t| t.as_str()) == Some("response.completed") {
        let r = data.get("response").unwrap_or(data);
        if let Some(usage) = r.get("usage") {
            if let Some(usage) = parse_usage_tokens(usage) {
                merge_usage_tokens(
                    usage,
                    input_tokens,
                    output_tokens,
                    cache_creation_tokens,
                    cache_read_tokens,
                );
            }
        }
    }
}

/// Record streaming metrics to the database and metrics aggregator.
fn record_stream_metrics(
    state: &AppState,
    info: &StreamMetricsInfo,
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
) {
    // Fallback: if upstream didn't report input_tokens, use the pre-stream estimate
    let input_tokens = if input_tokens == 0 {
        info.input_estimate
    } else {
        input_tokens.max(0)
    };
    let output_tokens = output_tokens.max(0);
    do_record_metric(
        state,
        RequestMetricInput {
            model_name: info.model_name.clone(),
            group_name: info.group_name.clone(),
            provider_id: info.provider_id.clone(),
            status_code: 200,
            latency_ms: info.started.elapsed().as_millis() as i64,
            input_tokens,
            output_tokens,
            cache_creation_input_tokens: cache_creation_tokens,
            cache_read_input_tokens: cache_read_tokens,
        },
    );
}

/// Build a ReceiverStream from a channel, compatible with axum SSE.
fn rx_to_sse_stream(
    rx: mpsc::Receiver<Result<Event, std::convert::Infallible>>,
) -> impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>> {
    futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|event| (event, rx))
    })
}

/// Find the next SSE message boundary and return (start_index_of_boundary, boundary_len).
/// Supports both LF and CRLF separators used by different upstream providers.
fn find_sse_message_boundary(buffer: &str) -> Option<(usize, usize)> {
    let candidates = [
        buffer.find("\r\n\r\n").map(|i| (i, 4)),
        buffer.find("\n\n").map(|i| (i, 2)),
        buffer.find("\r\r").map(|i| (i, 2)),
    ];

    candidates.into_iter().flatten().min_by_key(|(idx, _)| *idx)
}

/// Some upstreams emit Anthropic-style JSON in `data:` but omit `event:` lines.
/// Infer event name from `data.type` so downstream clients can render incrementally.
fn infer_event_type_from_data(data: &str) -> Option<String> {
    let v: Value = serde_json::from_str(data).ok()?;
    v.get("type")
        .and_then(|t| t.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};
    use serde_json::json;

    use super::{
        apply_anthropic_inbound_headers, deduplicate_url_path, estimate_input_tokens,
        parse_tokens_from_upstream_usage,
        UsageTokens,
    };

    #[test]
    fn applies_anthropic_inbound_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            HeaderValue::from_static("anthropic-sdk-typescript/1.14.0"),
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_static("token-counts-2026-01-09"),
        );
        headers.insert("x-stainless-lang", HeaderValue::from_static("js"));
        headers.insert("x-stainless-os", HeaderValue::from_static("MacOS"));

        let client = reqwest::Client::new();
        let req = client.post("http://localhost/test").json(&json!({}));
        let req = apply_anthropic_inbound_headers(req, Some(&headers), "2023-06-01");
        let built = req.build().unwrap();

        assert_eq!(
            built.headers().get("user-agent").unwrap(),
            "anthropic-sdk-typescript/1.14.0"
        );
        assert_eq!(
            built.headers().get("anthropic-version").unwrap(),
            "2023-06-01"
        );
        // claude-code-20250219 should be prepended to existing beta value
        assert_eq!(
            built.headers().get("anthropic-beta").unwrap(),
            "claude-code-20250219,token-counts-2026-01-09"
        );
        assert_eq!(built.headers().get("x-stainless-lang").unwrap(), "js");
        assert_eq!(built.headers().get("x-stainless-os").unwrap(), "MacOS");
    }

    #[test]
    fn applies_default_version_when_missing() {
        let client = reqwest::Client::new();
        let req = client.post("http://localhost/test").json(&json!({}));
        let req = apply_anthropic_inbound_headers(req, None, "2023-06-01");
        let built = req.build().unwrap();

        assert_eq!(
            built.headers().get("anthropic-version").unwrap(),
            "2023-06-01"
        );
        // claude-code-20250219 should always be injected
        assert_eq!(
            built.headers().get("anthropic-beta").unwrap(),
            "claude-code-20250219"
        );
        // user-agent should NOT be set when no inbound headers
        assert_eq!(built.headers().get("user-agent"), None);
    }

    #[test]
    fn does_not_duplicate_claude_code_beta() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_static("claude-code-20250219,messages-2024-04-01"),
        );

        let client = reqwest::Client::new();
        let req = client.post("http://localhost/test").json(&json!({}));
        let req = apply_anthropic_inbound_headers(req, Some(&headers), "2023-06-01");
        let built = req.build().unwrap();

        // Should pass through unchanged since it already contains the marker
        assert_eq!(
            built.headers().get("anthropic-beta").unwrap(),
            "claude-code-20250219,messages-2024-04-01"
        );
    }

    #[test]
    fn parses_openai_usage_tokens() {
        let body = json!({
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 30
            }
        });
        assert_eq!(
            parse_tokens_from_upstream_usage(&body),
            Some(UsageTokens {
                input_tokens: 120,
                output_tokens: 30,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
            })
        );
    }

    #[test]
    fn parses_anthropic_usage_tokens() {
        let body = json!({
            "usage": {
                "input_tokens": 88,
                "output_tokens": 22
            }
        });
        assert_eq!(
            parse_tokens_from_upstream_usage(&body),
            Some(UsageTokens {
                input_tokens: 88,
                output_tokens: 22,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
            })
        );
    }

    #[test]
    fn parses_gemini_usage_tokens() {
        let body = json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "totalTokenCount": 180
            }
        });
        assert_eq!(
            parse_tokens_from_upstream_usage(&body),
            Some(UsageTokens {
                input_tokens: 100,
                output_tokens: 80,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
            })
        );
    }

    #[test]
    fn parses_cached_prompt_tokens_without_double_counting_input() {
        let body = json!({
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 30,
                "prompt_tokens_details": {
                    "cached_tokens": 40
                }
            }
        });
        assert_eq!(
            parse_tokens_from_upstream_usage(&body),
            Some(UsageTokens {
                input_tokens: 80,
                output_tokens: 30,
                cache_creation_tokens: 0,
                cache_read_tokens: 40,
            })
        );
    }

    #[test]
    fn preserves_anthropic_input_tokens_when_cache_read_is_reported_separately() {
        let body = json!({
            "usage": {
                "input_tokens": 8,
                "output_tokens": 27,
                "cache_read_input_tokens": 5410
            }
        });
        assert_eq!(
            parse_tokens_from_upstream_usage(&body),
            Some(UsageTokens {
                input_tokens: 8,
                output_tokens: 27,
                cache_creation_tokens: 0,
                cache_read_tokens: 5410,
            })
        );
    }

    #[test]
    fn estimates_input_when_usage_missing() {
        let payload = json!({
            "messages": [
                {"role":"user", "content":"hello world"},
                {"role":"assistant", "content":"ok"}
            ]
        });
        assert!(estimate_input_tokens(&payload) > 0);
    }

    #[test]
    fn detects_deepseek_in_base_url() {
        assert!(super::is_reasoning_content_provider(
            "https://api.deepseek.com/v1",
            "claude-sonnet"
        ));
    }

    #[test]
    fn detects_deepseek_in_model_name() {
        // OpenCodeGo scenario: base_url doesn't contain deepseek, but model does
        assert!(super::is_reasoning_content_provider(
            "https://opencode.ai/zen/go/v1",
            "deepseek-v4-pro"
        ));
    }

    #[test]
    fn rejects_non_reasoning_content_provider() {
        assert!(!super::is_reasoning_content_provider(
            "https://api.openai.com/v1",
            "gpt-4o"
        ));
    }

    #[test]
    fn detects_moonshot() {
        assert!(super::is_reasoning_content_provider(
            "https://api.moonshot.cn/v1",
            "moonshot-v1"
        ));
    }

    #[test]
    fn dedup_double_v1_path() {
        assert_eq!(
            deduplicate_url_path("https://opencode.ai/zen/go/v1/v1/chat/completions"),
            "https://opencode.ai/zen/go/v1/chat/completions"
        );
    }

    #[test]
    fn dedup_no_duplicate_unchanged() {
        assert_eq!(
            deduplicate_url_path("https://api.openai.com/v1/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn dedup_consecutive_identical_segments() {
        assert_eq!(
            deduplicate_url_path("https://host.com/v1/v1/v1/messages"),
            "https://host.com/v1/messages"
        );
    }
}
