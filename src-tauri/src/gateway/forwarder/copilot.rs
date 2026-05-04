// Copilot streaming — handle Copilot-authenticated request forwarding
// with OpenAI SSE to Anthropic SSE translation and raw OpenAI passthrough.

use std::collections::HashMap;
use std::time::Instant;

use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{
    gateway::error::ForwardRequestError,
    log_codes::{STRM_DISCONNECT, STRM_DONE, STRM_EOF, STRM_ERROR, STRM_START},
    state::AppState,
};

use super::copilot_request;
use super::utf8_utils::{append_utf8_safe, flush_utf8_remainder};
use super::{
    apply_anthropic_inbound_headers, apply_openai_inbound_headers, build_copilot_headers,
    extract_usage_from_sse, find_sse_message_boundary, record_stream_metrics,
    resolve_binding_provider_group, rx_to_sse_stream, value_to_i64, StreamMetricsInfo,
};

pub(super) struct CopilotStreamState {
    pub(super) message_start_sent: bool,
    pub(super) thinking_block_open: bool,
    pub(super) content_block_open: bool,
    pub(super) content_block_index: usize,
    pub(super) tool_calls: HashMap<usize, ToolCallInfo>,
    /// 上游可能分多片发送同一 `index` 的 `id` / `name` / `arguments`（顺序与 OpenAI 官方示例不一致），在此合并后再开 `tool_use` 块。
    pub(super) pending_tool_streams: HashMap<usize, PendingToolStream>,
}

#[derive(Default)]
pub(super) struct PendingToolStream {
    pub(super) id: Option<String>,
    pub(super) name: Option<String>,
    pub(super) args_buffer: String,
    pub(super) started: bool,
    pub(super) anthropic_block_index: Option<usize>,
}

pub(super) struct ToolCallInfo {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    anthropic_block_index: usize,
}

pub(super) fn map_stop_reason(reason: &str) -> &'static str {
    match reason {
        "stop" => "end_turn",
        "length" => "max_tokens",
        "tool_calls" | "tool_call" => "tool_use",
        "content_filter" => "end_turn",
        _ => "end_turn",
    }
}

fn openai_tool_arguments_fragment(func: Option<&Value>) -> String {
    let Some(f) = func else {
        return String::new();
    };
    let Some(args) = f.get("arguments") else {
        return String::new();
    };
    match args {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// 关闭当前文本/thinking 块后，开启 Anthropic `tool_use` content block（并登记 `tool_calls` 映射表）。
fn emit_tool_use_content_block_start(
    state: &mut CopilotStreamState,
    events: &mut Vec<Event>,
    tc_index: usize,
    id: &str,
    name: &str,
) -> usize {
    if state.content_block_open {
        events.push(
            Event::default().event("content_block_stop").data(
                serde_json::to_string(&serde_json::json!({
                    "type": "content_block_stop",
                    "index": state.content_block_index,
                }))
                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
            ),
        );
        state.content_block_index += 1;
        state.content_block_open = false;
    }
    let anthropic_block_index = state.content_block_index;
    state.tool_calls.insert(
        tc_index,
        ToolCallInfo {
            id: id.to_string(),
            name: name.to_string(),
            anthropic_block_index,
        },
    );
    events.push(
        Event::default().event("content_block_start").data(
            serde_json::to_string(&serde_json::json!({
                "type": "content_block_start",
                "index": anthropic_block_index,
                "content_block": {
                    "type": "tool_use",
                    "id": id,
                    "name": name,
                    "input": {}
                }
            }))
            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
        ),
    );
    state.content_block_open = true;
    anthropic_block_index
}

/// 流结束前仍有「有函数名但未起块」的 tool 片段时补齐（部分网关最后才给 `id` 或始终省略）。
fn force_start_pending_tool_streams_on_tool_finish(
    state: &mut CopilotStreamState,
    events: &mut Vec<Event>,
    finish_reason: &str,
) {
    if finish_reason != "tool_calls" && finish_reason != "tool_call" {
        return;
    }
    close_thinking_block_if_open(state, events);
    let keys: Vec<usize> = state.pending_tool_streams.keys().copied().collect();
    for idx in keys {
        let mut pending = match state.pending_tool_streams.remove(&idx) {
            Some(p) => p,
            None => continue,
        };
        if pending.started {
            state.pending_tool_streams.insert(idx, pending);
            continue;
        }
        let Some(name) = pending.name.clone().filter(|n| !n.is_empty()) else {
            state.pending_tool_streams.insert(idx, pending);
            continue;
        };
        if pending.id.as_deref().unwrap_or("").is_empty() {
            pending.id = Some(format!("call_{idx}"));
        }
        let id = pending.id.clone().unwrap_or_else(|| format!("call_{idx}"));
        let anth_idx = emit_tool_use_content_block_start(state, events, idx, &id, &name);
        pending.started = true;
        pending.anthropic_block_index = Some(anth_idx);
        let buf = std::mem::take(&mut pending.args_buffer);
        if !buf.is_empty() {
            events.push(
                Event::default().event("content_block_delta").data(
                    serde_json::to_string(&serde_json::json!({
                        "type": "content_block_delta",
                        "index": anth_idx,
                        "delta": { "type": "input_json_delta", "partial_json": buf }
                    }))
                    .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                ),
            );
        }
        state.pending_tool_streams.insert(idx, pending);
    }
}

pub(super) fn close_thinking_block_if_open(
    state: &mut CopilotStreamState,
    events: &mut Vec<Event>,
) {
    if state.thinking_block_open {
        events.push(
            Event::default().event("content_block_delta").data(
                serde_json::to_string(&serde_json::json!({
                    "type": "content_block_delta",
                    "index": state.content_block_index,
                    "delta": { "type": "signature_delta", "signature": "" }
                }))
                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
            ),
        );
        events.push(
            Event::default().event("content_block_stop").data(
                serde_json::to_string(&serde_json::json!({
                    "type": "content_block_stop",
                    "index": state.content_block_index,
                }))
                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
            ),
        );
        state.content_block_index += 1;
        state.thinking_block_open = false;
    }
}

pub(super) fn is_tool_block_open(state: &CopilotStreamState) -> bool {
    if !state.content_block_open {
        return false;
    }
    state
        .tool_calls
        .values()
        .any(|tc| tc.anthropic_block_index == state.content_block_index)
}

pub(super) fn translate_openai_chunk_to_anthropic_events(
    chunk: &Value,
    state: &mut CopilotStreamState,
) -> Vec<Event> {
    let mut events: Vec<Event> = Vec::new();

    let choices = match chunk.get("choices").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return events,
    };

    for choice in choices {
        let delta = choice.get("delta");

        // ── message_start ──
        if !state.message_start_sent {
            let id = chunk.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let model = chunk.get("model").and_then(|m| m.as_str()).unwrap_or("");

            let usage_obj = chunk.get("usage");
            let prompt_tokens = usage_obj
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(value_to_i64)
                .unwrap_or(0);
            let cached_tokens = usage_obj
                .and_then(|u| u.get("prompt_tokens_details"))
                .and_then(|d| d.get("cached_tokens"))
                .and_then(value_to_i64);
            let input_tokens = (prompt_tokens - cached_tokens.unwrap_or(0)).max(0);

            let mut usage = serde_json::json!({
                "input_tokens": input_tokens,
                "output_tokens": 0,
            });
            if let Some(ct) = cached_tokens {
                usage["cache_read_input_tokens"] = serde_json::json!(ct);
            }

            events.push(
                Event::default().event("message_start").data(
                    serde_json::to_string(&serde_json::json!({
                        "type": "message_start",
                        "message": {
                            "id": id,
                            "type": "message",
                            "role": "assistant",
                            "content": [],
                            "model": model,
                            "stop_reason": null,
                            "stop_sequence": null,
                            "usage": usage,
                        }
                    }))
                    .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                ),
            );
            state.message_start_sent = true;
        }

        if let Some(d) = delta {
            // ── reasoning_text → thinking block ──
            if let Some(reasoning) = d.get("reasoning_text").and_then(|r| r.as_str()) {
                if !reasoning.is_empty() {
                    if !state.thinking_block_open {
                        events.push(
                            Event::default().event("content_block_start").data(
                                serde_json::to_string(&serde_json::json!({
                                    "type": "content_block_start",
                                    "index": state.content_block_index,
                                    "content_block": { "type": "thinking", "thinking": "" }
                                }))
                                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                            ),
                        );
                        state.thinking_block_open = true;
                    }
                    events.push(
                        Event::default().event("content_block_delta").data(
                            serde_json::to_string(&serde_json::json!({
                                "type": "content_block_delta",
                                "index": state.content_block_index,
                                "delta": { "type": "thinking_delta", "thinking": reasoning }
                            }))
                            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                        ),
                    );
                }
            }

            // ── reasoning (OpenAI o-series plain string) → thinking block ──
            if let Some(reasoning) = d.get("reasoning").and_then(|r| r.as_str()) {
                if !reasoning.is_empty() {
                    if !state.thinking_block_open {
                        events.push(
                            Event::default().event("content_block_start").data(
                                serde_json::to_string(&serde_json::json!({
                                    "type": "content_block_start",
                                    "index": state.content_block_index,
                                    "content_block": { "type": "thinking", "thinking": "" }
                                }))
                                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                            ),
                        );
                        state.thinking_block_open = true;
                    }
                    events.push(
                        Event::default().event("content_block_delta").data(
                            serde_json::to_string(&serde_json::json!({
                                "type": "content_block_delta",
                                "index": state.content_block_index,
                                "delta": { "type": "thinking_delta", "thinking": reasoning }
                            }))
                            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                        ),
                    );
                }
            }

            // ── reasoning_opaque → thinking with signature ──
            if let Some(opaque) = d.get("reasoning_opaque").and_then(|r| r.as_str()) {
                if !opaque.is_empty() {
                    if !state.thinking_block_open {
                        events.push(
                            Event::default().event("content_block_start").data(
                                serde_json::to_string(&serde_json::json!({
                                    "type": "content_block_start",
                                    "index": state.content_block_index,
                                    "content_block": { "type": "thinking", "thinking": "" }
                                }))
                                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                            ),
                        );
                        state.thinking_block_open = true;
                    }
                    events.push(
                        Event::default().event("content_block_delta").data(
                            serde_json::to_string(&serde_json::json!({
                                "type": "content_block_delta",
                                "index": state.content_block_index,
                                "delta": { "type": "signature_delta", "signature": opaque }
                            }))
                            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                        ),
                    );
                }
            }

            // ── content → text block ──
            if let Some(content) = d.get("content").and_then(|c| c.as_str()) {
                if !content.is_empty() {
                    close_thinking_block_if_open(state, &mut events);
                    if is_tool_block_open(state) {
                        events.push(
                            Event::default().event("content_block_stop").data(
                                serde_json::to_string(&serde_json::json!({
                                    "type": "content_block_stop",
                                    "index": state.content_block_index,
                                }))
                                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                            ),
                        );
                        state.content_block_index += 1;
                        state.content_block_open = false;
                    }
                    if !state.content_block_open {
                        events.push(
                            Event::default().event("content_block_start").data(
                                serde_json::to_string(&serde_json::json!({
                                    "type": "content_block_start",
                                    "index": state.content_block_index,
                                    "content_block": { "type": "text", "text": "" }
                                }))
                                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                            ),
                        );
                        state.content_block_open = true;
                    }
                    events.push(
                        Event::default().event("content_block_delta").data(
                            serde_json::to_string(&serde_json::json!({
                                "type": "content_block_delta",
                                "index": state.content_block_index,
                                "delta": { "type": "text_delta", "text": content }
                            }))
                            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                        ),
                    );
                }
            }

            // ── tool_calls → tool_use blocks（合并分片：id/name/arguments 到达顺序因上游而异）──
            if let Some(tool_calls) = d.get("tool_calls").and_then(|t| t.as_array()) {
                close_thinking_block_if_open(state, &mut events);
                for tc in tool_calls {
                    let tc_index = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                    let tc_id = tc.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    let tc_name = tc
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("");
                    let tc_args = openai_tool_arguments_fragment(tc.get("function"));

                    let mut pending = state
                        .pending_tool_streams
                        .remove(&tc_index)
                        .unwrap_or_default();
                    if !tc_id.is_empty() {
                        pending.id = Some(tc_id.to_string());
                    }
                    if !tc_name.is_empty() {
                        pending.name = Some(tc_name.to_string());
                    }
                    if !tc_args.is_empty() {
                        pending.args_buffer.push_str(&tc_args);
                    }

                    let id_ready = !pending.id.as_deref().unwrap_or("").is_empty();
                    let name_ready = !pending.name.as_deref().unwrap_or("").is_empty();
                    if id_ready && name_ready && !pending.started {
                        let id = pending.id.as_deref().unwrap_or("");
                        let name = pending.name.as_deref().unwrap_or("");
                        let anth_idx = emit_tool_use_content_block_start(
                            state,
                            &mut events,
                            tc_index,
                            id,
                            name,
                        );
                        pending.started = true;
                        pending.anthropic_block_index = Some(anth_idx);
                        let buf = std::mem::take(&mut pending.args_buffer);
                        if !buf.is_empty() {
                            events.push(
                                Event::default().event("content_block_delta").data(
                                    serde_json::to_string(&serde_json::json!({
                                        "type": "content_block_delta",
                                        "index": anth_idx,
                                        "delta": { "type": "input_json_delta", "partial_json": buf }
                                    }))
                                    .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                                ),
                            );
                        }
                    } else if pending.started && !tc_args.is_empty() {
                        if let Some(bi) = pending.anthropic_block_index {
                            events.push(
                                Event::default()
                                    .event("content_block_delta")
                                    .data(
                                        serde_json::to_string(&serde_json::json!({
                                            "type": "content_block_delta",
                                            "index": bi,
                                            "delta": { "type": "input_json_delta", "partial_json": tc_args }
                                        }))
                                        .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                                    ),
                            );
                        }
                    }
                    state.pending_tool_streams.insert(tc_index, pending);
                }
            }
        }

        // ── finish_reason → close blocks + message_delta + message_stop ──
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|f| f.as_str())
            .unwrap_or("");
        if !finish_reason.is_empty() {
            force_start_pending_tool_streams_on_tool_finish(state, &mut events, finish_reason);
            if state.content_block_open {
                events.push(
                    Event::default().event("content_block_stop").data(
                        serde_json::to_string(&serde_json::json!({
                            "type": "content_block_stop",
                            "index": state.content_block_index,
                        }))
                        .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                    ),
                );
                state.content_block_index += 1;
                state.content_block_open = false;
            } else if state.thinking_block_open {
                events.push(
                    Event::default().event("content_block_delta").data(
                        serde_json::to_string(&serde_json::json!({
                            "type": "content_block_delta",
                            "index": state.content_block_index,
                            "delta": { "type": "signature_delta", "signature": "" }
                        }))
                        .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                    ),
                );
                events.push(
                    Event::default().event("content_block_stop").data(
                        serde_json::to_string(&serde_json::json!({
                            "type": "content_block_stop",
                            "index": state.content_block_index,
                        }))
                        .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                    ),
                );
                state.content_block_index += 1;
                state.thinking_block_open = false;
            }

            let output_tokens = chunk
                .get("usage")
                .and_then(|u| u.get("completion_tokens"))
                .and_then(value_to_i64)
                .unwrap_or(0);

            events.push(
                Event::default().event("message_delta").data(
                    serde_json::to_string(&serde_json::json!({
                        "type": "message_delta",
                        "delta": {
                            "stop_reason": map_stop_reason(finish_reason),
                            "stop_sequence": null,
                        },
                        "usage": { "output_tokens": output_tokens }
                    }))
                    .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                ),
            );
            events.push(
                Event::default().event("message_stop").data(
                    serde_json::to_string(&serde_json::json!({
                        "type": "message_stop"
                    }))
                    .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                ),
            );
        }
    }

    events
}

/// Split one SSE message into `(event_name, data_line)` pairs (Copilot `/v1/responses` uses `event:` lines).
fn sse_message_event_data_pairs(message: &str) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    let mut pending_event: Option<String> = None;
    for raw in message.lines() {
        let line = raw.trim_end_matches('\r');
        let t = line.trim();
        if t.is_empty() || t.starts_with(':') {
            continue;
        }
        if let Some(rest) = t.strip_prefix("event:") {
            pending_event = Some(rest.trim().to_string());
        } else if let Some(rest) = t.strip_prefix("data:") {
            out.push((pending_event.take(), rest.trim().to_string()));
        }
    }
    out
}

fn extract_responses_stream_text_delta(data: &Value) -> String {
    if let Some(s) = data.get("delta").and_then(|d| d.as_str()) {
        return s.to_string();
    }
    if let Some(s) = data
        .get("delta")
        .and_then(|d| d.get("text"))
        .and_then(|t| t.as_str())
    {
        return s.to_string();
    }
    data.get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string()
}

fn push_anthropic_message_start_if_needed(
    state: &mut CopilotStreamState,
    events: &mut Vec<Event>,
    message_id: &str,
    model: &str,
) {
    if state.message_start_sent {
        return;
    }
    let usage = serde_json::json!({
        "input_tokens": 0,
        "output_tokens": 0,
    });
    events.push(
        Event::default().event("message_start").data(
            serde_json::to_string(&serde_json::json!({
                "type": "message_start",
                "message": {
                    "id": message_id,
                    "type": "message",
                    "role": "assistant",
                    "content": [],
                    "model": model,
                    "stop_reason": null,
                    "stop_sequence": null,
                    "usage": usage,
                }
            }))
            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
        ),
    );
    state.message_start_sent = true;
}

fn finish_anthropic_from_responses_completed(
    state: &mut CopilotStreamState,
    events: &mut Vec<Event>,
    envelope: &Value,
) {
    close_thinking_block_if_open(state, events);
    if state.content_block_open {
        events.push(
            Event::default().event("content_block_stop").data(
                serde_json::to_string(&serde_json::json!({
                    "type": "content_block_stop",
                    "index": state.content_block_index,
                }))
                .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
            ),
        );
        state.content_block_index += 1;
        state.content_block_open = false;
    }

    let r = envelope.get("response").unwrap_or(envelope);
    let output_tokens = r
        .get("usage")
        .and_then(|u| u.get("output_tokens"))
        .and_then(value_to_i64)
        .unwrap_or(0);

    events.push(
        Event::default().event("message_delta").data(
            serde_json::to_string(&serde_json::json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": "end_turn",
                    "stop_sequence": null,
                },
                "usage": { "output_tokens": output_tokens }
            }))
            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
        ),
    );
    events.push(Event::default().event("message_stop").data(
        serde_json::to_string(&serde_json::json!({ "type": "message_stop" })).unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
    ));
}

/// Map one Copilot `/v1/responses` SSE `data:` JSON (with optional `event:`) to Anthropic SSE events.
fn translate_copilot_responses_sse_to_anthropic(
    event_type: Option<&str>,
    data: &Value,
    state: &mut CopilotStreamState,
    response_id: &mut String,
    response_model: &mut String,
) -> (Vec<Event>, bool) {
    let mut events: Vec<Event> = Vec::new();
    let mut terminal = false;

    let type_in_json = data.get("type").and_then(|t| t.as_str());
    let effective = event_type.or(type_in_json);

    match effective {
        Some("response.created") => {
            if let Some(r) = data.get("response") {
                if let Some(id) = r.get("id").and_then(|i| i.as_str()) {
                    *response_id = id.to_string();
                }
                if let Some(m) = r.get("model").and_then(|m| m.as_str()) {
                    *response_model = m.to_string();
                }
            }
        }
        Some("response.output_text.delta") => {
            let delta = extract_responses_stream_text_delta(data);
            if !delta.is_empty() {
                push_anthropic_message_start_if_needed(
                    state,
                    &mut events,
                    response_id.as_str(),
                    response_model.as_str(),
                );
                close_thinking_block_if_open(state, &mut events);
                if is_tool_block_open(state) {
                    events.push(
                        Event::default().event("content_block_stop").data(
                            serde_json::to_string(&serde_json::json!({
                                "type": "content_block_stop",
                                "index": state.content_block_index,
                            }))
                            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                        ),
                    );
                    state.content_block_index += 1;
                    state.content_block_open = false;
                }
                if !state.content_block_open {
                    events.push(
                        Event::default().event("content_block_start").data(
                            serde_json::to_string(&serde_json::json!({
                                "type": "content_block_start",
                                "index": state.content_block_index,
                                "content_block": { "type": "text", "text": "" }
                            }))
                            .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                        ),
                    );
                    state.content_block_open = true;
                }
                events.push(
                    Event::default().event("content_block_delta").data(
                        serde_json::to_string(&serde_json::json!({
                            "type": "content_block_delta",
                            "index": state.content_block_index,
                            "delta": { "type": "text_delta", "text": delta }
                        }))
                        .unwrap_or_else(|e| { log::warn!("[FWD_SERIALIZE] JSON serialize failure: {e}"); String::new() }),
                    ),
                );
            }
        }
        Some("response.completed") => {
            if !state.message_start_sent {
                push_anthropic_message_start_if_needed(
                    state,
                    &mut events,
                    response_id.as_str(),
                    response_model.as_str(),
                );
            }
            finish_anthropic_from_responses_completed(state, &mut events, data);
            terminal = true;
        }
        _ => {}
    }

    (events, terminal)
}

fn openai_chat_completion_chunk_json(
    id: &str,
    model: &str,
    content_delta: Option<&str>,
    finish_reason: Option<&str>,
) -> String {
    let delta = match content_delta {
        Some(c) if !c.is_empty() => serde_json::json!({ "content": c }),
        _ => serde_json::json!({}),
    };
    let fin = finish_reason
        .map(|s| Value::String(s.to_string()))
        .unwrap_or(Value::Null);
    serde_json::json!({
        "id": id,
        "object": "chat.completion.chunk",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": fin,
        }]
    })
    .to_string()
}

/// Build the common copilot request builder (shared between streaming and non-streaming).
/// Returns (request_builder, target_url, copilot_token, binding_info).
struct CopilotRequestInfo {
    req: reqwest::RequestBuilder,
    #[allow(dead_code)]
    target_url: String,
    #[allow(dead_code)]
    payload: Value,
    binding_model_name: String,
    /// Upstream model id sent to Copilot (e.g. `gpt-5.3-codex`).
    upstream_model_name: String,
    provider_id: String,
    #[allow(dead_code)]
    provider_name: String,
    group_name: Option<String>,
    started: Instant,
    /// `POST .../v1/responses` when vendor is OpenAI-style (Codex) or Anthropic→responses.
    use_responses_upstream: bool,
    /// Client called `/v1/messages` (Anthropic).
    anthropic_inbound: bool,
    /// Client called `/v1/chat/completions` (OpenAI).
    openai_chat_inbound: bool,
}

async fn build_copilot_request(
    state: &AppState,
    model_name: &str,
    payload: Value,
    path: &str,
    inbound_headers: Option<&HeaderMap>,
) -> Result<CopilotRequestInfo, ForwardRequestError> {
    let ctx = copilot_request::prepare_copilot_request(state, model_name, path, inbound_headers)
        .await?;

    let payload = copilot_request::transform_copilot_payload(
        payload,
        &ctx.binding.upstream_model_name,
        ctx.anthropic_inbound,
        ctx.openai_chat_inbound,
        ctx.use_responses_upstream,
    );

    let copilot_path = if ctx.use_responses_upstream {
        "/v1/responses"
    } else {
        "/chat/completions"
    };
    let target_url = format!(
        "{}/{}",
        ctx.base_url,
        copilot_path.trim_start_matches('/')
    );

    let started = Instant::now();
    // TODO(future): Add fine-grained streaming timeouts (first-byte, idle)
    //   following cc-switch's auto_failover model with StreamingTimeoutConfig.
    //   See cc-switch: proxy/handler_context.rs StreamingTimeoutConfig
    let client = state.http_client.clone();

    let (copilot_headers, _request_id, editor_device_id) =
        build_copilot_headers(&ctx.copilot_token);
    let mut req = client.post(&target_url).json(&payload);
    for (name, value) in &copilot_headers {
        req = req.header(name, value);
    }
    req = req.header("Accept", "text/event-stream");
    req = req.header("Cache-Control", "no-cache");
    req = req.header("Connection", "keep-alive");
    // 避免 gzip 包裹 SSE 导致中间层或客户端侧整包缓冲
    req = req.header("Accept-Encoding", "identity");
    req = req.header("Editor-Device-Id", &editor_device_id);

    if ctx.anthropic_inbound {
        req = apply_anthropic_inbound_headers(req, inbound_headers, "2023-06-01");
    } else {
        req = apply_openai_inbound_headers(req, inbound_headers);
    }

    Ok(CopilotRequestInfo {
        req,
        target_url,
        payload,
        binding_model_name: ctx.binding.model_name,
        upstream_model_name: ctx.binding.upstream_model_name,
        provider_id: ctx.provider.id,
        provider_name: ctx.provider.name,
        group_name: ctx.group_name,
        started,
        use_responses_upstream: ctx.use_responses_upstream,
        anthropic_inbound: ctx.anthropic_inbound,
        openai_chat_inbound: ctx.openai_chat_inbound,
    })
}

/// Get the provider for a given model name (used by router for streaming detection)
pub fn get_provider_for_model(
    state: &AppState,
    model_name: &str,
) -> Result<crate::domain::provider::Provider, ForwardRequestError> {
    let (_binding, provider, _group_name) = resolve_binding_provider_group(state, model_name)?;
    Ok(provider)
}

/// Stream a copilot request and translate OpenAI SSE chunks to Anthropic SSE events.
pub async fn forward_request_copilot_stream(
    state: &AppState,
    model_name: &str,
    payload: Value,
    path: &str,
    inbound_headers: Option<&HeaderMap>,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>>,
    ForwardRequestError,
> {
    let info = build_copilot_request(state, model_name, payload, path, inbound_headers).await?;

    {
        let breaker = state
            .breaker
            .lock()
            .map_err(|_| ForwardRequestError::Upstream("circuit breaker lock error".to_string()))?;
        if breaker.is_open(&info.provider_id) {
            log::warn!(
                "[{}] copilot stream request rejected: provider '{}' circuit open",
                crate::log_codes::CB_OPEN,
                info.provider_name
            );
            return Err(ForwardRequestError::Upstream(format!(
                "provider '{}' circuit is open, please try again later",
                info.provider_name
            )));
        }
    }

    let resp = info
        .req
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
        return Err(ForwardRequestError::Upstream(
            body.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(&format!("Upstream returned status {status}"))
                .to_string(),
        ));
    }

    let anthropic_responses_stream = info.use_responses_upstream && info.anthropic_inbound;

    let stream_state = CopilotStreamState {
        message_start_sent: false,
        thinking_block_open: false,
        content_block_open: false,
        content_block_index: 0,
        tool_calls: HashMap::new(),
        pending_tool_streams: HashMap::new(),
    };

    let metrics_info = StreamMetricsInfo {
        model_name: info.binding_model_name.clone(),
        group_name: info.group_name.clone(),
        provider_id: info.provider_id.clone(),
        input_price_per_1m: 0.0, // Copilot is free
        output_price_per_1m: 0.0,
        input_estimate: 0,
        started: info.started,
    };
    let app_state = state.clone();

    let raw_byte_stream = resp.bytes_stream();
    let byte_stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<Vec<u8>, String>> + Send + Unpin>,
    > = Box::pin(
        raw_byte_stream.map(|result| result.map(|b| b.to_vec()).map_err(|e| e.to_string())),
    );

    // Use unfold with a pending event queue (VecDeque) to handle multiple events per chunk.
    let stream = futures_util::stream::unfold(
        AnthropicStreamState {
            byte_stream,
            sstate: stream_state,
            buffer: String::new(),
            utf8_remainder: Vec::new(),
            pending: std::collections::VecDeque::new(),
            done: false,
            metrics_info: Some(metrics_info),
            app_state: Some(app_state),
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
            chunk_count: 0,
            upstream_responses_api: anthropic_responses_stream,
            copilot_response_id: String::new(),
            copilot_response_model: info.upstream_model_name.clone(),
        },
        |mut state| async move {
            loop {
                // Drain pending events first
                if let Some(event) = state.pending.pop_front() {
                    return Some((Ok(event), state));
                }
                if state.done {
                    // Record metrics before ending the stream
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

                        // Process complete SSE messages with LF/CRLF compatibility.
                        while let Some((pos, sep_len)) = find_sse_message_boundary(&state.buffer) {
                            let message = state.buffer[..pos].to_string();
                            state.buffer = state.buffer[pos + sep_len..].to_string();

                            if state.upstream_responses_api {
                                for (event_name, data_line) in
                                    sse_message_event_data_pairs(&message)
                                {
                                    if data_line == "[DONE]" {
                                        if !state.sstate.message_start_sent {
                                            let mut start_events = Vec::new();
                                            push_anthropic_message_start_if_needed(
                                                &mut state.sstate,
                                                &mut start_events,
                                                state.copilot_response_id.as_str(),
                                                state.copilot_response_model.as_str(),
                                            );
                                            for e in start_events {
                                                state.pending.push_back(e);
                                            }
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

                                    let Ok(chunk) = serde_json::from_str::<Value>(&data_line)
                                    else {
                                        continue;
                                    };

                                    extract_usage_from_sse(
                                        &chunk,
                                        &mut state.input_tokens,
                                        &mut state.output_tokens,
                                        &mut state.cache_creation_tokens,
                                        &mut state.cache_read_tokens,
                                    );

                                    let (events, terminal) =
                                        translate_copilot_responses_sse_to_anthropic(
                                            event_name.as_deref(),
                                            &chunk,
                                            &mut state.sstate,
                                            &mut state.copilot_response_id,
                                            &mut state.copilot_response_model,
                                        );
                                    state.pending.extend(events);
                                    if terminal {
                                        state.done = true;
                                        break;
                                    }
                                }
                            } else {
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
                            if state.done {
                                break;
                            }
                        }
                        // Loop back to drain pending events
                    }
                    Some(Err(e)) => {
                        log::error!(
                            "[{STRM_ERROR}] copilot-transform stream error after {} chunks: {e}",
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
                            "[{STRM_EOF}] copilot-transform stream EOF after {} chunks",
                            state.chunk_count
                        );
                        flush_utf8_remainder(&mut state.buffer, &mut state.utf8_remainder);
                        state.done = true;
                        // If there are no pending events, end the stream.
                        if state.pending.is_empty() {
                            return None;
                        }
                        // Otherwise loop back to drain remaining pending events.
                    }
                }
            }
        },
    );

    Ok(Sse::new(stream))
}
pub(super) struct AnthropicStreamState {
    pub(super) byte_stream:
        std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Vec<u8>, String>> + Send + Unpin>>,
    pub(super) sstate: CopilotStreamState,
    pub(super) buffer: String,
    pub(super) utf8_remainder: Vec<u8>,
    pub(super) pending: std::collections::VecDeque<Event>,
    pub(super) done: bool,
    // Metrics tracking
    pub(super) metrics_info: Option<StreamMetricsInfo>,
    pub(super) app_state: Option<AppState>,
    pub(super) input_tokens: i64,
    pub(super) output_tokens: i64,
    pub(super) cache_creation_tokens: i64,
    pub(super) cache_read_tokens: i64,
    pub(super) chunk_count: u64,
    pub(super) upstream_responses_api: bool,
    pub(super) copilot_response_id: String,
    pub(super) copilot_response_model: String,
}

/// Stream a copilot request and pass through OpenAI SSE chunks directly (no translation).
/// Metrics are recorded when the stream ends.
pub async fn forward_request_copilot_stream_openai(
    state: &AppState,
    model_name: &str,
    payload: Value,
    path: &str,
    inbound_headers: Option<&HeaderMap>,
) -> Result<
    Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>>>,
    ForwardRequestError,
> {
    let info = build_copilot_request(state, model_name, payload, path, inbound_headers).await?;

    {
        let breaker = state
            .breaker
            .lock()
            .map_err(|_| ForwardRequestError::Upstream("circuit breaker lock error".to_string()))?;
        if breaker.is_open(&info.provider_id) {
            log::warn!(
                "[{}] copilot-openai stream request rejected: provider '{}' circuit open",
                crate::log_codes::CB_OPEN,
                info.provider_name
            );
            return Err(ForwardRequestError::Upstream(format!(
                "provider '{}' circuit is open, please try again later",
                info.provider_name
            )));
        }
    }

    let resp = info
        .req
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
        return Err(ForwardRequestError::Upstream(
            body.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(&format!("Upstream returned status {status}"))
                .to_string(),
        ));
    }

    let metrics_info = StreamMetricsInfo {
        model_name: info.binding_model_name.clone(),
        group_name: info.group_name.clone(),
        provider_id: info.provider_id.clone(),
        input_price_per_1m: 0.0,
        output_price_per_1m: 0.0,
        input_estimate: 0,
        started: info.started,
    };

    let translate_responses_sse = info.use_responses_upstream && info.openai_chat_inbound;
    let stream_model = info.upstream_model_name.clone();

    let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(32);
    let state_clone = state.clone();

    // Self-terminates: when stream ends, errors, or client disconnects (tx dropped).
    let _handle = tokio::spawn(async move {
        let mut byte_stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut utf8_remainder: Vec<u8> = Vec::new();
        let mut input_tokens: i64 = 0;
        let mut output_tokens: i64 = 0;
        let mut cache_creation_tokens: i64 = 0;
        let mut cache_read_tokens: i64 = 0;
        let mut chunk_count: u64 = 0;
        let mut stream_chunk_id = String::from("chatcmpl-copilot");
        let mut model_label = stream_model;
        let mut saw_output_delta = false;

        log::debug!(
            "[{STRM_START}] copilot-openai stream started model={} responses_sse={}",
            metrics_info.model_name,
            translate_responses_sse
        );

        let finish_metrics = |input_tokens: i64,
                              output_tokens: i64,
                              cache_creation_tokens: i64,
                              cache_read_tokens: i64| {
            record_stream_metrics(
                &state_clone,
                &metrics_info,
                input_tokens,
                output_tokens,
                cache_creation_tokens,
                cache_read_tokens,
            );
        };

        loop {
            match byte_stream.next().await {
                Some(Ok(bytes)) => {
                    chunk_count += 1;
                    append_utf8_safe(&mut buffer, &mut utf8_remainder, &bytes);

                    while let Some((pos, sep_len)) = find_sse_message_boundary(&buffer) {
                        let message = buffer[..pos].to_string();
                        buffer = buffer[pos + sep_len..].to_string();

                        if translate_responses_sse {
                            for (event_name, data_line) in sse_message_event_data_pairs(&message) {
                                if data_line == "[DONE]" {
                                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                    log::debug!("[{STRM_DONE}] copilot-openai stream completed chunks={chunk_count}");
                                    finish_metrics(
                                        input_tokens,
                                        output_tokens,
                                        cache_creation_tokens,
                                        cache_read_tokens,
                                    );
                                    return;
                                }

                                let Ok(v) = serde_json::from_str::<Value>(&data_line) else {
                                    continue;
                                };
                                extract_usage_from_sse(
                                    &v,
                                    &mut input_tokens,
                                    &mut output_tokens,
                                    &mut cache_creation_tokens,
                                    &mut cache_read_tokens,
                                );

                                let eff = event_name
                                    .as_deref()
                                    .or_else(|| v.get("type").and_then(|t| t.as_str()));

                                match eff {
                                    Some("response.created") => {
                                        if let Some(r) = v.get("response") {
                                            if let Some(id) = r.get("id").and_then(|i| i.as_str()) {
                                                stream_chunk_id = id.to_string();
                                            }
                                            if let Some(m) = r.get("model").and_then(|m| m.as_str())
                                            {
                                                model_label = m.to_string();
                                            }
                                        }
                                    }
                                    Some("response.output_text.delta") => {
                                        let piece = extract_responses_stream_text_delta(&v);
                                        if !piece.is_empty() {
                                            saw_output_delta = true;
                                            let chunk = openai_chat_completion_chunk_json(
                                                &stream_chunk_id,
                                                &model_label,
                                                Some(piece.as_str()),
                                                None,
                                            );
                                            if tx
                                                .send(Ok(Event::default().data(chunk)))
                                                .await
                                                .is_err()
                                            {
                                                flush_utf8_remainder(
                                                    &mut buffer,
                                                    &mut utf8_remainder,
                                                );
                                                log::info!("[{STRM_DISCONNECT}] copilot-openai client disconnected after {chunk_count} chunks");
                                                finish_metrics(
                                                    input_tokens,
                                                    output_tokens,
                                                    cache_creation_tokens,
                                                    cache_read_tokens,
                                                );
                                                return;
                                            }
                                        }
                                    }
                                    Some("response.completed") => {
                                        let fin = openai_chat_completion_chunk_json(
                                            &stream_chunk_id,
                                            &model_label,
                                            None,
                                            Some("stop"),
                                        );
                                        if tx.send(Ok(Event::default().data(fin))).await.is_err() {
                                            flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                            finish_metrics(
                                                input_tokens,
                                                output_tokens,
                                                cache_creation_tokens,
                                                cache_read_tokens,
                                            );
                                            return;
                                        }
                                        if tx
                                            .send(Ok(Event::default().data("[DONE]")))
                                            .await
                                            .is_err()
                                        {
                                            flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                            finish_metrics(
                                                input_tokens,
                                                output_tokens,
                                                cache_creation_tokens,
                                                cache_read_tokens,
                                            );
                                            return;
                                        }
                                        flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                        log::debug!("[{STRM_DONE}] copilot-openai responses stream completed chunks={chunk_count}");
                                        finish_metrics(
                                            input_tokens,
                                            output_tokens,
                                            cache_creation_tokens,
                                            cache_read_tokens,
                                        );
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            let mut event_type: Option<String> = None;
                            let mut data_line: Option<String> = None;

                            for line in message.lines() {
                                if let Some(evt) = line.strip_prefix("event: ") {
                                    event_type = Some(evt.trim().to_string());
                                } else if let Some(data) = line
                                    .strip_prefix("data: ")
                                    .or_else(|| line.strip_prefix("data:"))
                                {
                                    data_line = Some(data.trim().to_string());
                                }
                            }

                            if let Some(data) = data_line {
                                if data == "[DONE]" {
                                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                    log::debug!("[{STRM_DONE}] copilot-openai stream completed chunks={chunk_count}");
                                    finish_metrics(
                                        input_tokens,
                                        output_tokens,
                                        cache_creation_tokens,
                                        cache_read_tokens,
                                    );
                                    return;
                                }
                                if let Ok(v) = serde_json::from_str::<Value>(&data) {
                                    extract_usage_from_sse(
                                        &v,
                                        &mut input_tokens,
                                        &mut output_tokens,
                                        &mut cache_creation_tokens,
                                        &mut cache_read_tokens,
                                    );
                                }
                                let mut event = Event::default().data(data);
                                if let Some(et) = &event_type {
                                    event = event.event(et);
                                }
                                if tx.send(Ok(event)).await.is_err() {
                                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                                    log::info!("[{STRM_DISCONNECT}] copilot-openai client disconnected after {chunk_count} chunks");
                                    finish_metrics(
                                        input_tokens,
                                        output_tokens,
                                        cache_creation_tokens,
                                        cache_read_tokens,
                                    );
                                    return;
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                    log::error!("[{STRM_ERROR}] copilot-openai stream error after {chunk_count} chunks: {e}");
                    finish_metrics(
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens,
                    );
                    return;
                }
                None => {
                    flush_utf8_remainder(&mut buffer, &mut utf8_remainder);
                    if translate_responses_sse && saw_output_delta {
                        let fin = openai_chat_completion_chunk_json(
                            &stream_chunk_id,
                            &model_label,
                            None,
                            Some("stop"),
                        );
                        let _ = tx.send(Ok(Event::default().data(fin))).await;
                        let _ = tx.send(Ok(Event::default().data("[DONE]"))).await;
                    }
                    log::debug!(
                        "[{STRM_EOF}] copilot-openai stream EOF after {chunk_count} chunks"
                    );
                    finish_metrics(
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens,
                    );
                    return;
                }
            }
        }
    });

    Ok(Sse::new(rx_to_sse_stream(rx)))
}

#[cfg(test)]
mod openai_tool_stream_translation_tests {
    use serde_json::json;
    use std::collections::HashMap;

    use super::{translate_openai_chunk_to_anthropic_events, CopilotStreamState};

    fn base_state() -> CopilotStreamState {
        CopilotStreamState {
            message_start_sent: true,
            thinking_block_open: false,
            content_block_open: false,
            content_block_index: 0,
            tool_calls: HashMap::new(),
            pending_tool_streams: HashMap::new(),
        }
    }

    #[test]
    fn merges_tool_stream_when_name_chunk_arrives_before_id() {
        let mut state = base_state();
        let c1 = json!({
            "id": "resp1",
            "model": "m",
            "choices": [{
                "index": 0,
                "delta": { "tool_calls": [{
                    "index": 0,
                    "function": { "name": "fn1" }
                }]}
            }]
        });
        let e1 = translate_openai_chunk_to_anthropic_events(&c1, &mut state);
        let e1_dump: String = e1.iter().map(|ev| format!("{ev:?}")).collect();
        assert!(!e1_dump.contains("tool_use"));

        let c2 = json!({
            "choices": [{
                "index": 0,
                "delta": { "tool_calls": [{
                    "index": 0,
                    "id": "call_real_1",
                    "function": { "arguments": "{}" }
                }]}
            }]
        });
        let e2 = translate_openai_chunk_to_anthropic_events(&c2, &mut state);
        let e2_dump: String = e2.iter().map(|ev| format!("{ev:?}")).collect();
        assert!(e2_dump.contains("tool_use") && e2_dump.contains("call_real_1"));

        let c3 = json!({
            "choices": [{ "index": 0, "finish_reason": "tool_calls" }]
        });
        let e3 = translate_openai_chunk_to_anthropic_events(&c3, &mut state);
        let e3_dump: String = e3.iter().map(|ev| format!("{ev:?}")).collect();
        assert!(e3_dump.contains("message_stop"));
    }

    #[test]
    fn finish_tool_call_emits_tool_use_with_synthetic_id_when_upstream_omits_id() {
        let mut state = base_state();
        let c1 = json!({
            "id": "resp1",
            "model": "m",
            "choices": [{
                "index": 0,
                "delta": { "tool_calls": [{
                    "index": 0,
                    "function": { "name": "my_tool" }
                }]}
            }]
        });
        let _ = translate_openai_chunk_to_anthropic_events(&c1, &mut state);
        let c2 = json!({
            "choices": [{ "index": 0, "finish_reason": "tool_call" }]
        });
        let e2 = translate_openai_chunk_to_anthropic_events(&c2, &mut state);
        let e2_dump: String = e2.iter().map(|ev| format!("{ev:?}")).collect();
        assert!(e2_dump.contains("tool_use") && e2_dump.contains("call_0"));
    }
}
