//! Copilot 上游 HTTP 头：与 VS Code Copilot 扩展行为对齐，供聊天转发与 `/v1/models` 等请求复用。

use reqwest::header::HeaderMap;

/// 返回 (headers, request_id, editor_device_id)。`POST /chat/completions` 等需在最终请求上再设置 `Editor-Device-Id`。
pub fn build_copilot_headers(token: &str) -> (HeaderMap, String, String) {
    let request_id = uuid::Uuid::new_v4().to_string();
    let editor_device_id = uuid::Uuid::new_v4().to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {token}").parse().expect("valid header"),
    );
    headers.insert(
        "Content-Type",
        "application/json".parse().expect("valid header"),
    );
    headers.insert(
        "User-Agent",
        "GitHubCopilotChat/0.42.3".parse().expect("valid header"),
    );
    headers.insert(
        "Copilot-Integration-Id",
        "vscode-chat".parse().expect("valid header"),
    );
    headers.insert(
        "Editor-Version",
        "vscode/1.99.3".parse().expect("valid header"),
    );
    headers.insert(
        "Editor-Plugin-Version",
        "copilot-chat/0.42.3".parse().expect("valid header"),
    );
    headers.insert(
        "Openai-Intent",
        "conversation-agent".parse().expect("valid header"),
    );
    headers.insert("X-Request-Id", request_id.parse().expect("valid header"));
    headers.insert("X-Agent-Task-Id", request_id.parse().expect("valid header"));
    headers.insert(
        "X-Github-Api-Version",
        "2025-10-01".parse().expect("valid header"),
    );
    headers.insert(
        "X-Interaction-Type",
        "conversation-agent".parse().expect("valid header"),
    );
    headers.insert(
        "X-Vscode-User-Agent-Library-Version",
        "electron-fetch".parse().expect("valid header"),
    );

    (headers, request_id, editor_device_id)
}
