use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep as tokio_sleep, Instant};

use crate::domain::copilot_account::{CopilotAccount, CopilotAuthError, GitHubUser};
use crate::services::copilot_headers;

const GITHUB_BASE_URL: &str = "https://github.com";
const GITHUB_API_BASE_URL: &str = "https://api.github.com";
const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const GITHUB_APP_SCOPES: &str = "read:user";
const USER_AGENT: &str = "GitHubCopilotChat/0.42.3";
const COPILOT_API_ENDPOINT: &str = "https://api.githubcopilot.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotTokenResponse {
    pub token: String,
    pub expires_at: i64,
    pub refresh_in: i64,
    #[serde(default)]
    pub account_type: Option<String>,
}

fn build_client() -> Result<Client, CopilotAuthError> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| CopilotAuthError::NetworkError(format!("HTTP client build failed: {e}")))
}

/// Lazy-initialized shared client for Copilot auth operations.
/// Avoids creating a new `reqwest::Client` per call.
fn shared_client() -> Result<&'static Client, CopilotAuthError> {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<Result<Client, CopilotAuthError>> = OnceLock::new();
    CLIENT
        .get_or_init(build_client)
        .as_ref()
        .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))
}

pub async fn request_device_code() -> Result<DeviceCodeResponse, CopilotAuthError> {
    let client = shared_client()?;
    let resp = client
        .post(format!("{GITHUB_BASE_URL}/login/device/code"))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "client_id": GITHUB_CLIENT_ID,
            "scope": GITHUB_APP_SCOPES,
        }))
        .send()
        .await
        .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CopilotAuthError::NetworkError(format!(
            "Device code request failed ({status}): {body}"
        )));
    }

    let result = resp.json::<DeviceCodeResponse>().await.map_err(|e| {
        CopilotAuthError::ParseError(format!("Failed to parse device code response: {e}"))
    })?;

    log::info!(
        "[{}] device code requested",
        crate::log_codes::COP_AUTH_START
    );
    Ok(result)
}

pub async fn poll_access_token_with_timeout(
    device_code: &str,
    max_wait: Duration,
) -> Result<Option<String>, CopilotAuthError> {
    let client = shared_client()?;
    let mut sleep_duration = tokio::time::Duration::from_secs(6);
    let deadline = Instant::now() + max_wait;

    loop {
        if Instant::now() >= deadline {
            return Ok(None);
        }

        let resp = client
            .post(format!("{GITHUB_BASE_URL}/login/oauth/access_token"))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "client_id": GITHUB_CLIENT_ID,
                "device_code": device_code,
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
            }))
            .send()
            .await
            .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            tokio_sleep(sleep_duration).await;
            continue;
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| {
            CopilotAuthError::ParseError(format!("Failed to parse token response: {e}"))
        })?;

        if let Some(access_token) = json.get("access_token").and_then(|v| v.as_str()) {
            return Ok(Some(access_token.to_string()));
        }

        if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
            match error {
                "authorization_pending" | "slow_down" => {
                    if error == "slow_down" {
                        sleep_duration = (sleep_duration + tokio::time::Duration::from_secs(2))
                            .min(tokio::time::Duration::from_secs(15));
                    }
                    tokio_sleep(sleep_duration).await;
                    continue;
                }
                "expired_token" => {
                    return Err(CopilotAuthError::ExpiredToken);
                }
                "access_denied" => {
                    return Err(CopilotAuthError::AccessDenied);
                }
                _ => {
                    return Err(CopilotAuthError::ParseError(format!(
                        "Token poll error: {error}"
                    )));
                }
            }
        }

        tokio_sleep(sleep_duration).await;
    }
}

pub async fn fetch_copilot_token(
    github_token: &str,
    api_endpoint: Option<&str>,
) -> Result<CopilotTokenResponse, CopilotAuthError> {
    let client = shared_client()?;

    // token 接口优先使用 GitHub API 域名；企业端点作为回退。
    let mut candidates: Vec<String> = vec![GITHUB_API_BASE_URL.to_string()];
    if let Some(ep) = api_endpoint {
        let trimmed = ep.trim_end_matches('/').to_string();
        if !trimmed.is_empty() && !candidates.iter().any(|c| c == &trimmed) {
            candidates.push(trimmed);
        }
    }
    if !candidates.iter().any(|c| c == COPILOT_API_ENDPOINT) {
        candidates.push(COPILOT_API_ENDPOINT.to_string());
    }

    let mut last_failure: Option<(reqwest::StatusCode, String, String)> = None;
    for base in candidates {
        let url = format!("{}/copilot_internal/v2/token", base.trim_end_matches('/'));
        let resp = client
            .get(&url)
            .header("Authorization", format!("token {github_token}"))
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

        if resp.status().is_success() {
            return resp.json::<CopilotTokenResponse>().await.map_err(|e| {
                CopilotAuthError::ParseError(format!("Failed to parse copilot token: {e}"))
            });
        }

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status == 403 {
            log::error!(
                "[{}] copilot subscription not found (403)",
                crate::log_codes::COP_AUTH_FAIL
            );
            return Err(CopilotAuthError::NoCopilotSubscription);
        }
        last_failure = Some((status, body, url));
    }

    if let Some((status, body, url)) = last_failure {
        return Err(CopilotAuthError::CopilotTokenFetchFailed(format!(
            "Copilot token request failed ({status}) at {url}: {body}"
        )));
    }

    Err(CopilotAuthError::CopilotTokenFetchFailed(
        "Copilot token request failed: no endpoint candidates".to_string(),
    ))
}

pub async fn fetch_github_user(github_token: &str) -> Result<GitHubUser, CopilotAuthError> {
    let client = shared_client()?;
    let resp = client
        .get(format!("{GITHUB_API_BASE_URL}/user"))
        .header("Authorization", format!("token {github_token}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CopilotAuthError::GitHubTokenInvalid(format!(
            "GitHub user request failed ({status}): {body}"
        )));
    }

    resp.json::<GitHubUser>().await.map_err(|e| {
        CopilotAuthError::ParseError(format!("Failed to parse GitHub user response: {e}"))
    })
}

/// 动态发现 Copilot API 端点（支持企业 GitHub）
pub async fn discover_api_endpoint(github_token: &str) -> Result<String, CopilotAuthError> {
    let client = shared_client()?;
    let resp = client
        .get(format!("{GITHUB_API_BASE_URL}/copilot_internal/user"))
        .header("Authorization", format!("token {github_token}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

    if resp.status().is_success() {
        let json: serde_json::Value = resp.json::<serde_json::Value>().await.map_err(|e| {
            CopilotAuthError::ParseError(format!("Failed to parse user response: {e}"))
        })?;

        if let Some(endpoints) = json
            .get("endpoints")
            .and_then(|v| v.get("api"))
            .and_then(|v| v.as_str())
        {
            let trimmed = endpoints.trim_end_matches('/').to_string();
            // 验证返回的端点是有效的 URL
            if trimmed.starts_with("https://") && !trimmed.is_empty() {
                log::info!(
                    "[{}] copilot API endpoint discovered: {}",
                    crate::log_codes::COP_DISCOVER,
                    trimmed
                );
                return Ok(trimmed);
            }
        }
    }

    log::info!(
        "[{}] copilot API endpoint discovered: {}",
        crate::log_codes::COP_DISCOVER,
        COPILOT_API_ENDPOINT
    );
    Ok(COPILOT_API_ENDPOINT.to_string())
}

/// Copilot `/models` 会混入内部路由/容量节点 id（如 `accounts/msft/routers/...`），并非聊天可用的 `model` 名称。
fn copilot_model_id_looks_internal(id: &str) -> bool {
    let id = id.trim();
    if id.is_empty() {
        return true;
    }
    if id.contains("/routers/") {
        return true;
    }
    // 账号作用域下的内部资源路径，不是发给上游的 model id
    if id.starts_with("accounts/") {
        return true;
    }
    false
}

/// `policy.state` 表示组织策略禁用时，该条目不应出现在可选模型列表中。
fn copilot_policy_blocks_chat(policy: Option<&serde_json::Value>) -> bool {
    let Some(p) = policy else {
        return false;
    };
    let Some(state) = p.get("state").and_then(|v| v.as_str()) else {
        return false;
    };
    matches!(
        state.trim().to_ascii_lowercase().as_str(),
        "blocked"
            | "disabled"
            | "denied"
            | "unavailable"
            | "not_available"
            | "restricted"
    )
}

fn copilot_model_object_is_chat_usable(item: &serde_json::Value) -> bool {
    let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
        return false;
    };
    if copilot_model_id_looks_internal(id) {
        return false;
    }
    if item.get("blocked").and_then(|v| v.as_bool()) == Some(true) {
        return false;
    }
    if copilot_policy_blocks_chat(item.get("policy")) {
        return false;
    }
    true
}

fn parse_copilot_models_array(arr: &[serde_json::Value]) -> (Vec<String>, usize) {
    let mut out: Vec<String> = Vec::new();
    let mut dropped = 0usize;
    for m in arr {
        if m.get("id").and_then(|v| v.as_str()).is_some() {
            if copilot_model_object_is_chat_usable(m) {
                if let Some(id) = m.get("id").and_then(|v| v.as_str()) {
                    out.push(id.trim().to_string());
                }
            } else {
                dropped += 1;
            }
        } else if let Some(s) = m.as_str() {
            let id = s.trim();
            if !copilot_model_id_looks_internal(id) {
                out.push(id.to_string());
            } else {
                dropped += 1;
            }
        } else {
            dropped += 1;
        }
    }
    out.sort();
    let before_dedup = out.len();
    out.dedup();
    dropped += before_dedup.saturating_sub(out.len());
    (out, dropped)
}

fn parse_copilot_models_json(json: &serde_json::Value) -> (Vec<String>, usize) {
    if let Some(arr) = json.get("data").and_then(|v| v.as_array()) {
        return parse_copilot_models_array(arr);
    }
    if let Some(arr) = json.as_array() {
        return parse_copilot_models_array(arr);
    }
    if let Some(arr) = json.get("models").and_then(|v| v.as_array()) {
        return parse_copilot_models_array(arr);
    }
    (vec![], 0)
}

/// 获取可用模型列表（需使用 Copilot JWT，与网关 `build_copilot_headers` 一致；不能用 GitHub OAuth token）
pub async fn fetch_copilot_models(
    copilot_bearer_token: &str,
    api_endpoint: Option<&str>,
) -> Result<Vec<String>, CopilotAuthError> {
    let endpoint = api_endpoint
        .unwrap_or(COPILOT_API_ENDPOINT)
        .trim_end_matches('/');
    let client = shared_client()?;

    // Copilot 宿主与 OpenAI 兼容路径均可能出现；须带与 `/chat/completions` 相同的客户端头，否则 individual 等端点常返回 401/403。
    let path_candidates = ["/v1/models", "/models"];
    let mut last_failure: Option<(reqwest::StatusCode, String, String)> = None;

    for path in path_candidates {
        let url = format!("{endpoint}{path}");
        let (mut headers, _, editor_device_id) =
            copilot_headers::build_copilot_headers(copilot_bearer_token);
        headers.remove(reqwest::header::CONTENT_TYPE);
        headers.insert(
            "Accept",
            "application/json"
                .parse()
                .map_err(|e| CopilotAuthError::NetworkError(format!("header Accept: {e}")))?,
        );
        headers.insert(
            "Editor-Device-Id",
            editor_device_id.parse().map_err(|e| {
                CopilotAuthError::NetworkError(format!("header Editor-Device-Id: {e}"))
            })?,
        );

        let mut req = client.get(&url);
        for (name, value) in &headers {
            req = req.header(name, value);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            log::debug!(
                "[{}] models path not usable status={} url={}",
                crate::log_codes::COP_MODELS,
                status,
                url
            );
            last_failure = Some((status, body, url));
            continue;
        }

        let json: serde_json::Value = resp
            .json::<serde_json::Value>()
            .await
            .map_err(|e| CopilotAuthError::ParseError(format!("Failed to parse models: {e}")))?;

        let (models, dropped_unusable) = parse_copilot_models_json(&json);
        log::info!(
            "[{}] models ok base={} path={} kept={} dropped_unusable={}",
            crate::log_codes::COP_MODELS,
            endpoint,
            path,
            models.len(),
            dropped_unusable
        );
        return Ok(models);
    }

    if let Some((status, body, url)) = last_failure {
        log::warn!(
            "[{}] models failed after path fallbacks status={} url={}",
            crate::log_codes::COP_MODELS,
            status,
            url
        );
        return Err(CopilotAuthError::NetworkError(format!(
            "Failed to fetch models ({status}) at {url}: {body}"
        )));
    }

    log::warn!(
        "[{}] models failed: no successful path",
        crate::log_codes::COP_MODELS
    );
    Err(CopilotAuthError::NetworkError(
        "Failed to fetch models: no URL candidates".to_string(),
    ))
}

/// 获取 Copilot 用量信息
pub async fn fetch_copilot_usage(
    github_token: &str,
    api_endpoint: Option<&str>,
) -> Result<serde_json::Value, CopilotAuthError> {
    let endpoint = api_endpoint.unwrap_or(COPILOT_API_ENDPOINT);
    let client = shared_client()?;
    let resp = client
        .get(format!("{endpoint}/copilot_internal/user"))
        .header("Authorization", format!("token {github_token}"))
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| CopilotAuthError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(CopilotAuthError::NetworkError(format!(
            "Failed to fetch usage: {}",
            resp.status()
        )));
    }

    let json: serde_json::Value = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| CopilotAuthError::ParseError(format!("Failed to parse usage: {e}")))?;

    Ok(json)
}

/// 确保 Copilot token 有效（过期前 60 秒自动刷新）
pub async fn ensure_copilot_token(
    account: &CopilotAccount,
) -> Result<CopilotAccount, CopilotAuthError> {
    let github_token = account
        .github_token
        .as_deref()
        .ok_or_else(|| CopilotAuthError::AccountNotFound("Not authenticated".to_string()))?;

    let needs_refresh = match &account.token_expires_at {
        Some(expires) => {
            let expires_ts: i64 = expires.parse().unwrap_or(0);
            let now_ts = chrono::Utc::now().timestamp();
            now_ts >= expires_ts - 60
        }
        None => true,
    };

    if !needs_refresh && account.copilot_token.is_some() {
        return Ok(account.clone());
    }

    log::info!(
        "[{}] copilot token refresh needed",
        crate::log_codes::COP_AUTH_REFRESH
    );
    let copilot_resp = fetch_copilot_token(github_token, account.api_endpoint.as_deref()).await?;
    let mut updated = account.clone();
    updated.copilot_token = Some(copilot_resp.token);
    updated.token_expires_at = Some(copilot_resp.expires_at.to_string());
    updated.account_type = copilot_resp
        .account_type
        .clone()
        .unwrap_or(updated.account_type);
    Ok(updated)
}

#[cfg(test)]
mod copilot_models_parse_tests {
    use super::*;

    #[test]
    fn filters_internal_router_and_accounts_paths() {
        let json = serde_json::json!({
            "data": [
                { "id": "claude-sonnet-4" },
                { "id": "accounts/msft/routers/abc" },
                { "id": "accounts/ent/some-capacity" }
            ]
        });
        let (out, dropped) = parse_copilot_models_json(&json);
        assert_eq!(out, vec!["claude-sonnet-4".to_string()]);
        assert_eq!(dropped, 2);
    }

    #[test]
    fn filters_policy_blocked() {
        let json = serde_json::json!({
            "data": [
                { "id": "gpt-5", "policy": { "state": "available" } },
                { "id": "gpt-4o", "policy": { "state": "Blocked" } }
            ]
        });
        let (out, _) = parse_copilot_models_json(&json);
        assert_eq!(out, vec!["gpt-5".to_string()]);
    }

    #[test]
    fn keeps_when_policy_missing() {
        let json = serde_json::json!({
            "data": [ { "id": "claude-opus-4" } ]
        });
        let (out, dropped) = parse_copilot_models_json(&json);
        assert_eq!(out, vec!["claude-opus-4".to_string()]);
        assert_eq!(dropped, 0);
    }
}
