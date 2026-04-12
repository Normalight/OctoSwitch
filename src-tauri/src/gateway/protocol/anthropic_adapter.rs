//! Anthropic Messages 请求体解析（`/v1/messages`）。

use serde_json::Value;

pub fn extract_model(payload: &Value) -> Option<String> {
    payload
        .get("model")
        .and_then(|m| m.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::extract_model;

    #[test]
    fn extracts_anthropic_model() {
        let model = extract_model(&json!({"model":"claude-3-7-sonnet"}));
        assert_eq!(model.as_deref(), Some("claude-3-7-sonnet"));
    }
}
