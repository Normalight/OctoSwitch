//! GitHub Copilot：根据 `/models` 返回的 `vendor` 字段选择上游路径（对齐 cc-switch）。
//! - `vendor == "openai"`（忽略大小写）且客户端为 Anthropic `/v1/messages` 时 → 上游 `/v1/responses` + Responses 格式转换
//! - 其余（Anthropic、Gemini 等）→ `/chat/completions` + Chat Completions 格式转换

use std::collections::HashMap;
use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::domain::copilot_account::CopilotAuthError;
use crate::log_codes;
use crate::services::copilot_headers;

const CACHE_TTL: Duration = Duration::from_secs(600);

struct CachedEntry {
    fetched_at: Instant,
    /// model id → vendor 字符串（如 `openai`、`anthropic`、`google`）
    vendors: HashMap<String, String>,
}

pub struct CopilotVendorCache {
    inner: Mutex<HashMap<String, CachedEntry>>,
}

impl CopilotVendorCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Anthropic 入站时，是否应对 Copilot 上游调用 OpenAI **Responses** API（`/v1/responses`）。
    pub async fn copilot_upstream_is_openai_responses(
        &self,
        provider_id: &str,
        upstream_model_id: &str,
        copilot_bearer: &str,
        api_base: &str,
        http: &Client,
    ) -> bool {
        let vendor = self
            .resolve_vendor(provider_id, upstream_model_id, copilot_bearer, api_base, http)
            .await;
        match vendor.as_deref() {
            Some(v) if v.eq_ignore_ascii_case("openai") => true,
            _ => false,
        }
    }

    async fn resolve_vendor(
        &self,
        provider_id: &str,
        upstream_model_id: &str,
        copilot_bearer: &str,
        api_base: &str,
        http: &Client,
    ) -> Option<String> {
        let now = Instant::now();
        {
            let guard = self.inner.lock().await;
            if let Some(e) = guard.get(provider_id) {
                let fresh = now.duration_since(e.fetched_at) < CACHE_TTL;
                if fresh {
                    if let Some(v) = e.vendors.get(upstream_model_id) {
                        return Some(v.clone());
                    }
                }
            }
        }

        let map = match fetch_model_vendor_map(http, copilot_bearer, api_base).await {
            Ok(m) => m,
            Err(e) => {
                log::warn!(
                    "[{}] copilot model vendor map fetch failed: {e}",
                    log_codes::COP_VENDOR
                );
                return None;
            }
        };

        let mut guard = self.inner.lock().await;
        guard.insert(
            provider_id.to_string(),
            CachedEntry {
                fetched_at: Instant::now(),
                vendors: map.clone(),
            },
        );
        map.get(upstream_model_id).cloned()
    }
}

fn parse_vendor_map(json: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(arr) = json.get("data").and_then(|a| a.as_array()) else {
        return out;
    };
    for item in arr {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let vendor = item
            .get("vendor")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        out.insert(id.to_string(), vendor);
    }
    out
}

async fn fetch_model_vendor_map(
    client: &Client,
    copilot_bearer: &str,
    api_base: &str,
) -> Result<HashMap<String, String>, CopilotAuthError> {
    let base = api_base.trim_end_matches('/');
    let path_candidates = ["/models", "/v1/models"];
    let mut last_err: Option<CopilotAuthError> = None;

    for path in path_candidates {
        let url = format!("{base}{path}");
        let (mut headers, _, editor_device_id) = copilot_headers::build_copilot_headers(copilot_bearer);
        headers.remove(reqwest::header::CONTENT_TYPE);
        headers.insert(
            "Accept",
            "application/json"
                .parse()
                .map_err(|e| CopilotAuthError::NetworkError(format!("header Accept: {e}")))?,
        );
        headers.insert(
            "Editor-Device-Id",
            editor_device_id
                .parse()
                .map_err(|e| CopilotAuthError::NetworkError(format!("header Editor-Device-Id: {e}")))?,
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
                "[{}] vendor map path status={} url={} body_len={}",
                log_codes::COP_VENDOR,
                status,
                url,
                body.len()
            );
            last_err = Some(CopilotAuthError::NetworkError(format!(
                "models {status} at {url}"
            )));
            continue;
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| CopilotAuthError::ParseError(format!("parse models json: {e}")))?;

        let map = parse_vendor_map(&json);
        if map.is_empty() {
            log::debug!(
                "[{}] vendor map empty from url={}",
                log_codes::COP_VENDOR,
                url
            );
            last_err = Some(CopilotAuthError::ParseError(
                "models response missing data[]".into(),
            ));
            continue;
        }

        log::info!(
            "[{}] copilot vendor map loaded url={} entries={}",
            log_codes::COP_VENDOR,
            url,
            map.len()
        );
        return Ok(map);
    }

    Err(last_err.unwrap_or_else(|| {
        CopilotAuthError::NetworkError("copilot models: no path succeeded".into())
    }))
}
