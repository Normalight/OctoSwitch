use std::time::Duration;

use reqwest::Client;

use crate::config::app_config::AppConfig;

/// Resolve proxy URL from app config, falling back to standard env vars.
///
/// Priority: app `http_proxy` (from `OCTOSWITCH_HTTP_PROXY` or legacy `MG_HTTP_PROXY` when using default `AppConfig`) > HTTPS_PROXY > HTTP_PROXY
fn resolve_proxy_url(config: &AppConfig) -> String {
    if let Some(ref proxy) = config.http_proxy {
        if !proxy.is_empty() {
            return proxy.clone();
        }
    }
    for var in &["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return val;
            }
        }
    }
    String::new()
}

/// Build a shared reqwest Client for reuse across requests.
/// Uses no connection-level timeout; per-request timeouts are applied on the RequestBuilder.
pub fn build_shared_client(config: &AppConfig) -> Result<Client, String> {
    let mut builder = Client::builder();
    let proxy_url = resolve_proxy_url(config);
    if !proxy_url.is_empty() {
        let proxy = reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| e.to_string())
}

/// Build a reqwest Client with a connection-level timeout for outbound requests.
#[allow(dead_code)]
pub fn build_outbound_client(timeout: Duration, config: &AppConfig) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(timeout);
    let proxy_url = resolve_proxy_url(config);
    if !proxy_url.is_empty() {
        let proxy = reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(|e| e.to_string())
}
