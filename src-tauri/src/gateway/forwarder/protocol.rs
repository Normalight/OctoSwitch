// Protocol conversion: Anthropic ↔ OpenAI format translation
// Contains all conversion functions and their helper utilities.

use serde_json::Value;

use super::value_to_i64;

fn normalize_tool_schema(schema: &Value) -> Value {
    let mut s = schema.clone();
    if s.get("type").and_then(|t| t.as_str()) == Some("object") && s.get("properties").is_none() {
        s["properties"] = serde_json::json!({});
    }
    s
}

/// Map Anthropic content blocks to OpenAI content parts.
/// Returns a string when all blocks are text-only; otherwise returns an array of parts.
fn map_anthropic_content_to_openai(content: &Value) -> Value {
    match content {
        Value::String(s) => Value::String(s.clone()),
        Value::Array(blocks) => {
            let mut has_non_text = false;
            let mut parts: Vec<Value> = Vec::new();
            for block in blocks {
                let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match block_type {
                    "text" => {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            parts.push(serde_json::json!({
                                "type": "text",
                                "text": text
                            }));
                        }
                    }
                    "image" => {
                        has_non_text = true;
                        let media_type = block
                            .get("source")
                            .and_then(|s| s.get("media_type"))
                            .and_then(|m| m.as_str())
                            .unwrap_or("image/png");
                        let data = block
                            .get("source")
                            .and_then(|s| s.get("data"))
                            .and_then(|d| d.as_str())
                            .unwrap_or("");
                        parts.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{media_type};base64,{data}")
                            }
                        }));
                    }
                    "tool_reference" => {
                        let tool_name = block
                            .get("tool_name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        parts.push(serde_json::json!({
                            "type": "text",
                            "text": format!("Tool {tool_name} loaded")
                        }));
                    }
                    _ => {
                        // Skip unrecognized block types (tool_use, tool_result, thinking, etc.)
                    }
                }
            }
            // When only text blocks are present, collapse to a single string for OpenAI.
            if !has_non_text {
                let text: String = parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                Value::String(text)
            } else {
                Value::Array(parts)
            }
        }
        _ => Value::String(String::new()),
    }
}

/// Map a subset of content blocks (filtering by allowed types) to OpenAI content.
fn map_content_subset<'a>(
    blocks: impl Iterator<Item = &'a Value>,
    allowed_types: &[&str],
) -> Value {
    let filtered: Vec<&Value> = blocks
        .filter(|b| {
            let bt = b.get("type").and_then(|t| t.as_str()).unwrap_or("");
            allowed_types.contains(&bt)
        })
        .collect();
    let arr = Value::Array(filtered.into_iter().cloned().collect());
    map_anthropic_content_to_openai(&arr)
}

/// Convert Anthropic tool_choice to OpenAI tool_choice.
fn translate_anthropic_tool_choice_to_openai(tool_choice: &Value) -> Option<Value> {
    match tool_choice {
        Value::String(s) => match s.as_str() {
            "auto" => Some(serde_json::json!("auto")),
            "any" | "required" => Some(serde_json::json!("required")),
            "none" => Some(serde_json::json!("none")),
            _ => None,
        },
        Value::Object(obj) => {
            let tc_type = obj
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("auto");
            match tc_type {
                "auto" => Some(serde_json::json!("auto")),
                "any" => Some(serde_json::json!("required")),
                "none" => Some(serde_json::json!("none")),
                "tool" => {
                    let name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.is_empty() {
                        None
                    } else {
                        Some(serde_json::json!({
                            "type": "function",
                            "function": { "name": name }
                        }))
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Handle a user message: split tool_result blocks into tool messages,
/// and remaining content into a user message.
fn handle_user_message(msg: &Value) -> Vec<Value> {
    let mut result: Vec<Value> = Vec::new();
    let content = msg.get("content");

    match content {
        Some(Value::Array(blocks)) => {
            // Separate tool_result blocks from other blocks
            let tool_result_blocks: Vec<&Value> = blocks
                .iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result"))
                .collect();
            let other_blocks: Vec<&Value> = blocks
                .iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) != Some("tool_result"))
                .collect();

            // Tool results first (tool_use → tool_result protocol order)
            for block in &tool_result_blocks {
                let tool_use_id = match block
                    .get("tool_use_id")
                    .and_then(|id| id.as_str())
                {
                    Some(id) if !id.is_empty() => id,
                    _ => continue, // skip malformed tool_result without valid ID
                };
                let tool_content = block
                    .get("content")
                    .map(map_anthropic_content_to_openai)
                    .unwrap_or(Value::String(String::new()));
                result.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": tool_use_id,
                    "content": tool_content,
                }));
            }

            // Remaining user content
            if !other_blocks.is_empty() {
                let other_content = map_content_subset(other_blocks.into_iter(), &["text", "image", "tool_reference"]);
                result.push(serde_json::json!({
                    "role": "user",
                    "content": other_content,
                }));
            }
        }
        _ => {
            let content_value = content
                .map(map_anthropic_content_to_openai)
                .unwrap_or(Value::String(String::new()));
            result.push(serde_json::json!({
                "role": "user",
                "content": content_value,
            }));
        }
    }

    result
}

/// Handle an assistant message: extract thinking blocks, tool_use blocks,
/// and text content into the OpenAI assistant message format.
fn handle_assistant_message(msg: &Value, model_id: &str, emit_reasoning_extensions: bool) -> Vec<Value> {
    let content = msg.get("content");

    // Non-array content: simple text
    let Some(blocks) = content.and_then(|c| c.as_array()) else {
        let content_value = content
            .map(map_anthropic_content_to_openai)
            .unwrap_or(Value::String(String::new()));
        let mut one = serde_json::json!({
            "role": "assistant",
            "content": content_value,
        });
        normalize_openai_assistant_content_for_tool_calls(&mut one);
        return vec![one];
    };

    let tool_use_blocks: Vec<&Value> = blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
        .collect();
    let mut thinking_blocks: Vec<&Value> = blocks
        .iter()
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("thinking"))
        .collect();

    // For claude models, filter out invalid thinking blocks
    if model_id.starts_with("claude") {
        thinking_blocks.retain(|b| {
            let thinking = b.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
            let signature = b.get("signature").and_then(|s| s.as_str()).unwrap_or("");
            !thinking.is_empty()
                && thinking != "Thinking..."
                && !signature.is_empty()
                && !signature.contains('@')
        });
    }

    // Join valid thinking content
    let thinking_contents: Vec<&str> = thinking_blocks
        .iter()
        .filter_map(|b| {
            let t = b.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
            if !t.is_empty() && t != "Thinking..." {
                Some(t)
            } else {
                None
            }
        })
        .collect();
    let all_thinking_content = if thinking_contents.is_empty() {
        None
    } else {
        Some(thinking_contents.join("\n\n"))
    };
    let signature = thinking_blocks
        .iter()
        .find_map(|b| b.get("signature").and_then(|s| s.as_str()))
        .map(String::from);

    // Map non-tool-use, non-thinking content blocks to OpenAI format
    let text_content = map_content_subset(
        blocks.iter(),
        &["text", "image", "tool_reference"],
    );

    // Build the assistant message
    let mut assistant_msg = serde_json::json!({
        "role": "assistant",
        "content": text_content,
    });
    if emit_reasoning_extensions {
        if let Some(ref rt) = all_thinking_content {
            assistant_msg["reasoning_text"] = Value::String(rt.clone());
        }
        if let Some(ref sig) = signature {
            assistant_msg["reasoning_opaque"] = Value::String(sig.clone());
        }
    }

    // Add tool_calls if present
    if !tool_use_blocks.is_empty() {
        let tool_calls: Vec<Value> = tool_use_blocks
            .iter()
            .filter_map(|tu| {
                let id = match tu.get("id").and_then(|i| i.as_str()) {
                    Some(id) if !id.is_empty() => id,
                    _ => return None, // skip tool_use without valid ID
                };
                let name = tu.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let input = tu.get("input").cloned().unwrap_or(serde_json::json!({}));
                let arguments = input.to_string();
                Some(serde_json::json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": arguments,
                    }
                }))
            })
            .collect();
        assistant_msg["tool_calls"] = Value::Array(tool_calls);
    }

    normalize_openai_assistant_content_for_tool_calls(&mut assistant_msg);

    vec![assistant_msg]
}

// ── Anthropic → OpenAI Chat Completions ──

/// Options for [`convert_anthropic_to_openai`].
#[derive(Debug, Clone, Copy)]
pub(super) struct AnthropicToOpenAiOptions {
    /// When true (e.g. GitHub Copilot OpenAI upstream), include `reasoning_text` / `reasoning_opaque`
    /// on assistant messages and map Anthropic `thinking.budget_tokens` → top-level `reasoning`.
    /// Strict OpenAI-compatible APIs (MiniMax, many proxies) reject unknown fields → keep false.
    pub emit_reasoning_extensions: bool,
}

impl Default for AnthropicToOpenAiOptions {
    fn default() -> Self {
        Self {
            emit_reasoning_extensions: false,
        }
    }
}

/// OpenAI Chat Completions expects `content: null` when the assistant turn is tool-only; an empty
/// string breaks templates and strict validators (common with function-calling round-trips).
fn normalize_openai_assistant_content_for_tool_calls(msg: &mut Value) {
    let has_tool_calls = msg
        .get("tool_calls")
        .and_then(|t| t.as_array())
        .is_some_and(|a| !a.is_empty());
    if !has_tool_calls {
        return;
    }
    match msg.get("content") {
        Some(Value::String(s)) if s.is_empty() => {
            msg["content"] = Value::Null;
        }
        Some(Value::Array(a)) if a.is_empty() => {
            msg["content"] = Value::Null;
        }
        _ => {}
    }
}

/// Convert an Anthropic /v1/messages payload to OpenAI /chat/completions format.
pub(super) fn convert_anthropic_to_openai(payload: &Value, opts: AnthropicToOpenAiOptions) -> Value {
    let model = payload
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gpt-4o");
    let max_tokens = payload
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(4096);
    let stream = payload
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    // ── System message ──
    let mut messages: Vec<Value> = Vec::new();
    if let Some(system) = payload.get("system") {
        let system_text = match system {
            Value::String(s) => s.clone(),
            Value::Array(arr) => arr
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        };
        if !system_text.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system_text
            }));
        }
    }

    // ── Messages ──
    if let Some(msgs) = payload.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");
            match role {
                "user" => {
                    let expanded = handle_user_message(msg);
                    messages.extend(expanded);
                }
                "assistant" => {
                    let expanded =
                        handle_assistant_message(msg, model, opts.emit_reasoning_extensions);
                    messages.extend(expanded);
                }
                _ => {
                    let content = map_anthropic_content_to_openai(
                        msg.get("content").cloned().as_ref().unwrap_or(&Value::Null),
                    );
                    messages.push(serde_json::json!({
                        "role": role,
                        "content": content
                    }));
                }
            }
        }
    }

    // ── Build OpenAI payload ──
    let mut openai = serde_json::json!({
        "model": model,
        "messages": messages,
        "max_tokens": max_tokens,
        "stream": stream,
    });

    if let Some(temp) = payload.get("temperature") {
        openai["temperature"] = temp.clone();
    }
    if let Some(top_p) = payload.get("top_p") {
        openai["top_p"] = top_p.clone();
    }

    if opts.emit_reasoning_extensions {
        if let Some(thinking) = payload.get("thinking") {
            if let Some(budget) = thinking.get("budget_tokens").and_then(|b| b.as_i64()) {
                openai["reasoning"] = serde_json::json!({
                    "effort": "high",
                    "max_tokens": budget
                });
            }
        }
    }

    if let Some(stop) = payload.get("stop_sequences").and_then(|s| s.as_array()) {
        let stop_vals: Vec<Value> = stop
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .map(Value::String)
            .collect();
        if !stop_vals.is_empty() {
            openai["stop"] = Value::Array(stop_vals);
        }
    }

    if let Some(user) = payload
        .get("metadata")
        .and_then(|m| m.get("user_id"))
        .and_then(|u| u.as_str())
    {
        openai["user"] = Value::String(user.to_string());
    }

    if let Some(tools) = payload.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                let name = tool
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("");
                let description = tool
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let parameters = normalize_tool_schema(
                    &tool
                        .get("input_schema")
                        .cloned()
                        .unwrap_or(serde_json::json!({"type":"object","properties":{}})),
                );
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": name,
                        "description": description,
                        "parameters": parameters,
                    }
                })
            })
            .collect();
        openai["tools"] = Value::Array(openai_tools);
    }

    if let Some(tc) = payload.get("tool_choice") {
        if let Some(openai_tc) = translate_anthropic_tool_choice_to_openai(tc) {
            openai["tool_choice"] = openai_tc;
        }
    }

    openai
}

// ── OpenAI → Anthropic response translation ──

/// Convert an OpenAI chat completion response to Anthropic message format.
pub(super) fn convert_openai_to_anthropic(response: &Value) -> Value {
    let choices = response.get("choices").and_then(|c| c.as_array());
    let first_choice = choices.and_then(|c| c.first());

    let mut assistant_content_blocks: Vec<Value> = Vec::new();
    let mut stop_reason: Option<&str> = first_choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|f| f.as_str());

    if let Some(choices_arr) = choices {
        for choice in choices_arr {
            let message = choice.get("message");

            if let Some(msg) = message {
                let reasoning_text = msg
                    .get("reasoning_text")
                    .and_then(|r| r.as_str())
                    .unwrap_or("");
                let reasoning_opaque = msg
                    .get("reasoning_opaque")
                    .and_then(|r| r.as_str())
                    .unwrap_or("");

                if !reasoning_text.is_empty() {
                    assistant_content_blocks.push(serde_json::json!({
                        "type": "thinking",
                        "thinking": reasoning_text,
                        "signature": if reasoning_opaque.is_empty() { "" } else { reasoning_opaque },
                    }));
                } else if !reasoning_opaque.is_empty() {
                    assistant_content_blocks.push(serde_json::json!({
                        "type": "thinking",
                        "thinking": "Thinking...",
                        "signature": reasoning_opaque,
                    }));
                }
            }

            if let Some(msg) = message {
                let content = msg.get("content");
                match content {
                    Some(Value::String(s)) if !s.is_empty() => {
                        assistant_content_blocks
                            .push(serde_json::json!({ "type": "text", "text": s }));
                    }
                    Some(Value::Array(parts)) => {
                        for part in parts {
                            match part.get("type").and_then(|t| t.as_str()) {
                                Some("text") => {
                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        assistant_content_blocks.push(
                                            serde_json::json!({ "type": "text", "text": text }),
                                        );
                                    }
                                }
                                Some("thinking") => {
                                    let thinking = part.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
                                    let signature = part.get("signature").and_then(|s| s.as_str()).unwrap_or("");
                                    if !thinking.is_empty() {
                                        assistant_content_blocks.push(serde_json::json!({
                                            "type": "thinking",
                                            "thinking": thinking,
                                            "signature": signature,
                                        }));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }

                if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
                    for tool_call in tool_calls {
                        let id = tool_call
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("");
                        let name = tool_call
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("");
                        let arguments_str = tool_call
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .map(|a| match a {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            })
                            .unwrap_or_else(|| "{}".to_string());
                        let input: Value =
                            serde_json::from_str(&arguments_str).unwrap_or(serde_json::json!({}));
                        assistant_content_blocks.push(serde_json::json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input,
                        }));
                    }
                }
            }

            let choice_finish = choice
                .get("finish_reason")
                .and_then(|f| f.as_str())
                .unwrap_or("");
            if choice_finish == "tool_calls" || choice_finish == "tool_call" {
                stop_reason = Some("tool_calls");
            } else if stop_reason != Some("tool_calls") && !choice_finish.is_empty() {
                stop_reason = Some(choice_finish);
            }
        }
    }

    let anthropic_stop_reason: Option<&str> = match stop_reason {
        Some("stop") => Some("end_turn"),
        Some("length") => Some("max_tokens"),
        Some("tool_calls") => Some("tool_use"),
        Some("content_filter") => Some("end_turn"),
        other => other,
    };

    let usage = response.get("usage");
    let prompt_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(value_to_i64)
        .unwrap_or(0);
    let cached_tokens = usage
        .and_then(|u| u.get("prompt_tokens_details"))
        .and_then(|d| d.get("cached_tokens"))
        .and_then(value_to_i64);
    let input_tokens = (prompt_tokens - cached_tokens.unwrap_or(0)).max(0);
    let output_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(value_to_i64)
        .unwrap_or(0);

    let mut anthropic_usage = serde_json::json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
    });
    if let Some(ct) = cached_tokens {
        anthropic_usage["cache_read_input_tokens"] = serde_json::json!(ct);
    }

    let response_id = response
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("");
    let response_model = response
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    serde_json::json!({
        "id": response_id,
        "type": "message",
        "role": "assistant",
        "model": response_model,
        "content": assistant_content_blocks,
        "stop_reason": anthropic_stop_reason,
        "stop_sequence": Value::Null,
        "usage": anthropic_usage,
    })
}

// ── OpenAI Responses API ↔ Anthropic translation ──

/// Convert an Anthropic /v1/messages payload to OpenAI Responses API format.
pub(super) fn convert_anthropic_to_openai_responses(payload: &Value) -> Value {
    let model = payload
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gpt-4o");

    let mut input: Vec<Value> = Vec::new();

    if let Some(system) = payload.get("system") {
        let system_text = match system {
            Value::String(s) => s.clone(),
            Value::Array(arr) => arr
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        };
        if !system_text.is_empty() {
            input.push(serde_json::json!({
                "type": "message",
                "role": "system",
                "content": system_text
            }));
        }
    }

    if let Some(msgs) = payload.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");
            let content = map_anthropic_content_to_openai(
                msg.get("content").cloned().as_ref().unwrap_or(&Value::Null),
            );
            input.push(serde_json::json!({
                "type": "message",
                "role": role,
                "content": content
            }));
        }
    }

    let max_output_tokens = payload
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(4096);

    let mut openai = serde_json::json!({
        "model": model,
        "input": input,
        "max_output_tokens": max_output_tokens,
    });

    if let Some(temp) = payload.get("temperature") {
        openai["temperature"] = temp.clone();
    }
    if let Some(top_p) = payload.get("top_p") {
        openai["top_p"] = top_p.clone();
    }
    if let Some(stop) = payload.get("stop_sequences").and_then(|s| s.as_array()) {
        let stop_vals: Vec<Value> = stop
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .map(Value::String)
            .collect();
        if !stop_vals.is_empty() {
            openai["stop"] = Value::Array(stop_vals);
        }
    }
    if let Some(tools) = payload.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let description = tool.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let parameters = normalize_tool_schema(
                    &tool.get("input_schema").cloned().unwrap_or(serde_json::json!({"type":"object","properties":{}})),
                );
                serde_json::json!({
                    "type": "function",
                    "name": name,
                    "description": description,
                    "parameters": parameters,
                })
            })
            .collect();
        openai["tools"] = Value::Array(openai_tools);
    }
    if let Some(tc) = payload.get("tool_choice") {
        if let Some(openai_tc) = translate_anthropic_tool_choice_to_openai(tc) {
            openai["tool_choice"] = openai_tc;
        }
    }
    if let Some(stream) = payload.get("stream") {
        openai["stream"] = stream.clone();
    }

    openai
}

/// Convert an OpenAI Responses API response to Anthropic message format.
pub(super) fn convert_openai_responses_to_anthropic(response: &Value) -> Value {
    let output = response.get("output").and_then(|o| o.as_array());
    let mut content_blocks: Vec<Value> = Vec::new();
    let mut stop_reason = response
        .get("stop_reason")
        .and_then(|s| s.as_str())
        .or_else(|| {
            response.get("status").and_then(|s| s.as_str()).map(|s| match s {
                "completed" => "end_turn",
                "max_tokens" => "max_tokens",
                _ => "end_turn",
            })
        });

    if let Some(output_arr) = output {
        for item in output_arr {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match item_type {
                "message" => {
                    if let Some(content) = item.get("content") {
                        let blocks = match content {
                            Value::String(s) => {
                                vec![serde_json::json!({ "type": "text", "text": s })]
                            }
                            Value::Array(parts) => parts
                                .iter()
                                .map(|p| {
                                    if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                                        serde_json::json!({ "type": "text", "text": text })
                                    } else if p.get("type").and_then(|t| t.as_str()) == Some("function_call") {
                                        serde_json::json!({
                                            "type": "tool_use",
                                            "id": p.get("call_id").and_then(|c| c.as_str()).unwrap_or(""),
                                            "name": p.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                                            "input": p.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new())),
                                        })
                                    } else {
                                        serde_json::json!({ "type": "text", "text": "" })
                                    }
                                })
                                .collect(),
                            _ => vec![],
                        };
                        content_blocks.extend(blocks);
                    }
                    if let Some(fr) = item.get("finish_reason") {
                        stop_reason = fr.as_str().map(|s| match s {
                            "stop" => "end_turn",
                            "length" => "max_tokens",
                            "tool_calls" => "tool_use",
                            other => other,
                        });
                    }
                }
                "function_call" => {
                    content_blocks.push(serde_json::json!({
                        "type": "tool_use",
                        "id": item.get("call_id").and_then(|c| c.as_str()).unwrap_or(""),
                        "name": item.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                        "input": item.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new())),
                    }));
                }
                _ => {}
            }
        }
    }

    let usage = response.get("usage");
    let prompt_tokens = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);
    let completion_tokens = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);
    let model = response.get("model").and_then(|m| m.as_str()).unwrap_or("");

    serde_json::json!({
        "id": response.get("id").and_then(|i| i.as_str()).unwrap_or(""),
        "type": "message",
        "role": "assistant",
        "content": content_blocks,
        "model": model,
        "stop_reason": stop_reason,
        "stop_sequence": Value::Null,
        "usage": {
            "input_tokens": prompt_tokens,
            "output_tokens": completion_tokens,
        },
    })
}

// ── OpenAI Chat Completions → Responses API (Copilot vendor=openai / Codex) ──

/// Convert a `/v1/chat/completions` request body to OpenAI Responses API shape for `POST /v1/responses`.
pub(super) fn convert_openai_chat_completion_request_to_responses(body: &Value) -> Value {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gpt-4o");

    let mut input: Vec<Value> = Vec::new();
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = msg
                .get("content")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new()));
            input.push(serde_json::json!({
                "type": "message",
                "role": role,
                "content": content
            }));
        }
    }

    let mut out = serde_json::json!({
        "model": model,
        "input": Value::Array(input),
    });

    if let Some(m) = body
        .get("max_tokens")
        .or_else(|| body.get("max_completion_tokens"))
    {
        out["max_output_tokens"] = m.clone();
    }

    for key in [
        "temperature",
        "top_p",
        "stream",
        "tools",
        "tool_choice",
        "reasoning_effort",
        "metadata",
        "store",
        "service_tier",
    ] {
        if let Some(v) = body.get(key) {
            out[key] = v.clone();
        }
    }

    out
}

/// Map a non-streaming `/v1/responses` JSON body to OpenAI `chat.completion` for clients that called `/v1/chat/completions`.
pub(super) fn convert_openai_responses_json_to_chat_completion(response: &Value) -> Value {
    let mut text = String::new();
    if let Some(out) = response.get("output").and_then(|o| o.as_array()) {
        for item in out {
            match item.get("content") {
                Some(Value::String(s)) => text.push_str(s),
                Some(Value::Array(parts)) => {
                    for part in parts {
                        let pt = part.get("type").and_then(|t| t.as_str());
                        if matches!(pt, Some("output_text") | Some("text")) {
                            if let Some(t) = part.get("text").and_then(|x| x.as_str()) {
                                text.push_str(t);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let id = response
        .get("id")
        .and_then(|i| i.as_str())
        .unwrap_or("chatcmpl-copilot");
    let model = response.get("model").and_then(|m| m.as_str()).unwrap_or("");
    let usage = response.get("usage");
    let prompt_tokens = usage
        .and_then(|u| u.get("input_tokens"))
        .or_else(|| usage.and_then(|u| u.get("prompt_tokens")))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);
    let completion_tokens = usage
        .and_then(|u| u.get("output_tokens"))
        .or_else(|| usage.and_then(|u| u.get("completion_tokens")))
        .and_then(|t| t.as_i64())
        .unwrap_or(0);

    serde_json::json!({
        "id": id,
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": text,
            },
            "finish_reason": "stop",
            "logprobs": Value::Null,
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens.saturating_add(completion_tokens),
        },
    })
}

#[cfg(test)]
mod anthropic_to_openai_tests {
    use serde_json::json;

    use super::{convert_anthropic_to_openai, AnthropicToOpenAiOptions};

    #[test]
    fn tool_only_assistant_content_is_null_not_empty_string() {
        let payload = json!({
            "model": "MiniMax-M2",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "toolu_1", "name": "bash", "input": {"cmd": "ls"}}
                ]}
            ],
            "tools": [],
            "stream": true
        });
        let openai = convert_anthropic_to_openai(&payload, AnthropicToOpenAiOptions::default());
        let msgs = openai["messages"].as_array().unwrap();
        let asst = msgs.iter().find(|m| m["role"] == "assistant").unwrap();
        assert!(asst["tool_calls"].as_array().is_some_and(|a| !a.is_empty()));
        assert!(asst["content"].is_null(), "expected null content, got {:?}", asst["content"]);
    }

    #[test]
    fn default_omits_reasoning_extensions_for_strict_openai_compat() {
        let payload = json!({
            "model": "gpt-test",
            "max_tokens": 100,
            "thinking": {"type": "enabled", "budget_tokens": 2000},
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "x", "signature": "sig"},
                    {"type": "text", "text": "hello"}
                ]
            }],
            "stream": false
        });
        let openai = convert_anthropic_to_openai(&payload, AnthropicToOpenAiOptions::default());
        assert!(openai.get("reasoning").is_none());
        let asst = openai["messages"][0].as_object().unwrap();
        assert!(asst.get("reasoning_text").is_none());
        assert!(asst.get("reasoning_opaque").is_none());
    }

    #[test]
    fn emit_reasoning_extensions_preserves_copilot_style_fields() {
        let payload = json!({
            "model": "gpt-test",
            "max_tokens": 100,
            "thinking": {"type": "enabled", "budget_tokens": 2000},
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "chain", "signature": "opaque1"},
                    {"type": "text", "text": "hello"}
                ]
            }],
            "stream": false
        });
        let openai = convert_anthropic_to_openai(
            &payload,
            AnthropicToOpenAiOptions {
                emit_reasoning_extensions: true,
            },
        );
        assert!(openai.get("reasoning").is_some());
        let asst = openai["messages"][0].as_object().unwrap();
        assert_eq!(asst.get("reasoning_text").and_then(|v| v.as_str()), Some("chain"));
        assert_eq!(
            asst.get("reasoning_opaque").and_then(|v| v.as_str()),
            Some("opaque1")
        );
    }
}
