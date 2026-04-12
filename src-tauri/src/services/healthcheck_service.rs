use reqwest::Client;

use crate::domain::provider::Provider;

use super::model_fetch::{apply_provider_request_headers, build_models_url, provider_protocol};

#[derive(serde::Serialize)]
pub struct HealthCheckResult {
    pub ok: bool,
    pub latency_ms: u128,
    pub message: String,
}

pub async fn check_provider(provider: &Provider, client: &Client) -> HealthCheckResult {
    let start = std::time::Instant::now();
    let protocol = provider_protocol(provider);
    let endpoint = match build_models_url(&provider.base_url) {
        Ok(url) => url,
        Err(e) => {
            return HealthCheckResult {
                ok: false,
                latency_ms: start.elapsed().as_millis(),
                message: e,
            };
        }
    };

    let req = client.get(&endpoint);
    let result = apply_provider_request_headers(req, provider).send().await;

    match result {
        Ok(resp) => {
            let status = resp.status();
            let ok = status.is_success() || status.is_redirection();
            HealthCheckResult {
                ok,
                latency_ms: start.elapsed().as_millis(),
                message: format!("{protocol} GET /v1/models -> HTTP {}", status.as_u16()),
            }
        }
        Err(e) => HealthCheckResult {
            ok: false,
            latency_ms: start.elapsed().as_millis(),
            message: format!("{protocol} GET /v1/models failed: {e}"),
        },
    }
}
