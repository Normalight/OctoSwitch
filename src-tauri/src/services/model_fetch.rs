//! List models via OpenAI-compatible `GET /v1/models`.

use std::time::Duration;

use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::domain::provider::Provider;
use crate::log_codes::MDL_FETCH;

const FETCH_TIMEOUT_SECS: u64 = 15;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedModel {
    pub id: String,
    pub owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Option<Vec<ModelEntry>>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    owned_by: Option<String>,
}

pub(crate) fn provider_protocol(_provider: &Provider) -> &'static str {
    "openai_compat"
}

pub(crate) fn build_models_url(base_url: &str) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');

    if trimmed.is_empty() {
        return Err("Base URL is empty".to_string());
    }

    if trimmed.ends_with("/v1/models") {
        return Ok(trimmed.to_string());
    }
    if trimmed.ends_with("/v1") {
        return Ok(format!("{trimmed}/models"));
    }
    if let Some(root) = trimmed.strip_suffix("/messages") {
        return Ok(format!("{root}/models"));
    }
    if let Some(root) = trimmed.strip_suffix("/chat/completions") {
        return Ok(format!("{root}/models"));
    }
    if let Some(root) = trimmed.strip_suffix("/responses") {
        return Ok(format!("{root}/models"));
    }

    Ok(format!("{trimmed}/v1/models"))
}

pub(crate) fn apply_provider_request_headers(
    mut req: RequestBuilder,
    provider: &Provider,
) -> RequestBuilder {
    if provider.auth_mode == "anthropic_api_key" {
        req = req.header("x-api-key", provider.api_key_ref.trim());
    } else {
        req = req.header(
            "Authorization",
            format!("Bearer {}", provider.api_key_ref.trim()),
        );
    }

    req
}

/// Query upstream `/v1/models` using OpenAI-compatible model discovery.
pub async fn fetch_models(
    client: &Client,
    provider: &Provider,
) -> Result<Vec<FetchedModel>, String> {
    if provider.api_key_ref.trim().is_empty() {
        return Err("API key is required to fetch models".to_string());
    }

    let url = build_models_url(provider.base_url.trim())?;
    let protocol = provider_protocol(provider);
    log::info!(
        "[{MDL_FETCH}] {} GET models provider_id={} name={} url={}",
        protocol,
        provider.id,
        provider.name,
        url
    );

    let req = client
        .get(&url)
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS));
    let response = apply_provider_request_headers(req, provider)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        log::warn!(
            "[{MDL_FETCH}] {} models HTTP error provider_id={} status={} url={}",
            protocol,
            provider.id,
            status,
            url
        );
        return Err(format!("HTTP {status}: {body}"));
    }

    let resp: ModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    let mut models: Vec<FetchedModel> = resp
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|m| FetchedModel {
            id: m.id,
            owned_by: m.owned_by,
        })
        .collect();

    models.sort_by(|a, b| a.id.cmp(&b.id));
    log::info!(
        "[{MDL_FETCH}] {} models ok provider_id={} count={}",
        protocol,
        provider.id,
        models.len()
    );
    Ok(models)
}

#[cfg(test)]
mod tests {
    use crate::domain::provider::Provider;

    use super::{apply_provider_request_headers, build_models_url, provider_protocol};

    fn provider(base_url: &str) -> Provider {
        Provider {
            id: "p1".to_string(),
            name: "provider".to_string(),
            base_url: base_url.to_string(),
            api_key_ref: "test-key".to_string(),
            timeout_ms: 60_000,
            max_retries: 0,
            is_enabled: true,
            sort_order: 0,
            api_format: None,
            auth_mode: "bearer".to_string(),
        }
    }

    #[test]
    fn builds_v1_models_from_root() {
        assert_eq!(
            build_models_url("https://api.example.com").unwrap(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn builds_v1_models_from_v1_suffix() {
        assert_eq!(
            build_models_url("https://api.example.com/v1").unwrap(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn derives_models_from_messages_endpoint() {
        assert_eq!(
            build_models_url("https://api.example.com/v1/messages").unwrap(),
            "https://api.example.com/v1/models"
        );
    }

    #[test]
    fn model_fetch_is_labeled_openai_compat() {
        let provider = provider("https://api.example.com");
        assert_eq!(provider_protocol(&provider), "openai_compat");
    }

    #[test]
    fn anthropic_api_key_mode_uses_x_api_key_only() {
        let mut provider = provider("https://api.anthropic.com");
        provider.auth_mode = "anthropic_api_key".to_string();
        let client = reqwest::Client::new();
        let req = client.get("https://api.example.com/v1/models");
        let built = apply_provider_request_headers(req, &provider)
            .build()
            .unwrap();

        assert_eq!(built.headers().get("x-api-key").unwrap(), "test-key");
        assert_eq!(built.headers().get("anthropic-version"), None);
    }
}
