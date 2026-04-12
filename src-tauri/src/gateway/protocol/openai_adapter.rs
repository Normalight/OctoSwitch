//! OpenAI Chat Completions 请求体解析（`/v1/chat/completions`）。

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
    fn extracts_openai_model() {
        let model = extract_model(&json!({"model":"gpt-4o"}));
        assert_eq!(model.as_deref(), Some("gpt-4o"));
    }
}
