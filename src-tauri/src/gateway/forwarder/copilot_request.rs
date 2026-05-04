// Shared Copilot request preparation logic.
// Used by both streaming (copilot.rs) and non-streaming (non_streaming.rs) forward paths.

use axum::http::HeaderMap;
use serde_json::Value;

use crate::{
    database::copilot_account_dao,
    domain::{model_binding::ModelBinding, provider::Provider},
    gateway::error::ForwardRequestError,
    log_codes::COP_TOKEN_PERSIST,
    services::copilot_auth,
    state::AppState,
};

use super::resolve_binding_provider_group;

/// Context returned by `prepare_copilot_request` — contains everything needed
/// to build an upstream Copilot HTTP request in either streaming or non-streaming mode.
pub struct CopilotRequestContext {
    pub binding: ModelBinding,
    pub provider: Provider,
    pub group_name: Option<String>,
    pub copilot_token: String,
    /// Trimmed base URL (trailing slashes removed).
    pub base_url: String,
    /// Whether the Copilot vendor supports the OpenAI Responses API endpoint.
    pub use_responses_upstream: bool,
    /// Client called `/v1/messages` (Anthropic protocol).
    pub anthropic_inbound: bool,
    /// Client called `/v1/chat/completions` (OpenAI protocol).
    pub openai_chat_inbound: bool,
}

/// Resolve model binding, check circuit breaker, and prepare Copilot auth/token/endpoint info.
pub async fn prepare_copilot_request(
    state: &AppState,
    model_name: &str,
    path: &str,
    inbound_headers: Option<&HeaderMap>,
) -> Result<CopilotRequestContext, ForwardRequestError> {
    let (binding, provider, group_name) = resolve_binding_provider_group(state, model_name)?;
    prepare_copilot_request_for_provider(
        state,
        &provider,
        &binding,
        group_name,
        path,
        inbound_headers,
    )
    .await
}

/// Like `prepare_copilot_request` but takes an already-resolved provider/binding.
/// Use when the caller has already resolved the model binding (e.g., non_streaming.rs
/// that needs to handle both copilot and non-copilot providers in the same function).
pub async fn prepare_copilot_request_for_provider(
    state: &AppState,
    provider: &Provider,
    binding: &ModelBinding,
    group_name: Option<String>,
    path: &str,
    _inbound_headers: Option<&HeaderMap>,
) -> Result<CopilotRequestContext, ForwardRequestError> {
    // Check circuit breaker
    {
        let breaker = state
            .breaker
            .lock()
            .map_err(|_| ForwardRequestError::Upstream("Circuit breaker lock error".to_string()))?;
        if breaker.is_open(&provider.id) {
            return Err(ForwardRequestError::Upstream(format!(
                "Provider '{}' circuit breaker is open, please retry later",
                provider.name
            )));
        }
    }

    // Look up Copilot account and refresh token if needed
    let (copilot_token, api_endpoint) = {
        let account = {
            let conn = state
                .db
                .lock()
                .map_err(|_| ForwardRequestError::Upstream("Database lock error".to_string()))?;
            copilot_account_dao::get_by_provider(&conn, &provider.id)
                .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?
                .ok_or_else(|| {
                    ForwardRequestError::Upstream("Copilot account not authorized".to_string())
                })?
        };
        let updated = copilot_auth::ensure_copilot_token(&account)
            .await
            .map_err(|e| ForwardRequestError::Upstream(e.to_string()))?;
        let token = updated
            .copilot_token
            .clone()
            .ok_or_else(|| ForwardRequestError::Upstream("Copilot token missing".to_string()))?;
        let endpoint = updated.api_endpoint.clone();
        if updated.copilot_token != account.copilot_token
            || updated.token_expires_at != account.token_expires_at
        {
            let conn = state
                .db
                .lock()
                .map_err(|_| ForwardRequestError::Upstream("Database lock error".to_string()))?;
            if let Err(e) = copilot_account_dao::update(&conn, &updated) {
                log::warn!(
                    "[{COP_TOKEN_PERSIST}] failed to persist copilot token refresh: {e}"
                );
            }
        }
        (token, endpoint)
    };

    let copilot_base_url = api_endpoint.unwrap_or_else(|| provider.base_url.clone());
    let base_url = copilot_base_url.trim_end_matches('/').to_string();

    let path_normalized = path.trim().to_lowercase();
    let anthropic_inbound = path_normalized.contains("/v1/messages");
    let openai_chat_inbound = path_normalized.contains("/v1/chat/completions");

    let vendor_openai_responses = state
        .copilot_vendor_cache
        .copilot_upstream_is_openai_responses(
            &provider.id,
            &binding.upstream_model_name,
            &copilot_token,
            &base_url,
            &state.http_client,
        )
        .await;

    let use_responses_upstream =
        vendor_openai_responses && (anthropic_inbound || openai_chat_inbound);

    let copilot_path = if use_responses_upstream {
        "/v1/responses"
    } else {
        "/chat/completions"
    };

    log::debug!(
        target: "octoswitch::gateway",
        "[{}] copilot upstream model={} path={} responses_upstream={} anthropic_in={} openai_chat_in={}",
        crate::log_codes::COP_VENDOR,
        binding.upstream_model_name,
        copilot_path,
        use_responses_upstream,
        anthropic_inbound,
        openai_chat_inbound
    );

    Ok(CopilotRequestContext {
        binding: binding.clone(),
        provider: provider.clone(),
        group_name,
        copilot_token,
        base_url,
        use_responses_upstream,
        anthropic_inbound,
        openai_chat_inbound,
    })
}

/// Apply payload model field and format conversion for Copilot upstream.
/// Returns the possibly-converted payload.
pub fn transform_copilot_payload(
    mut payload: Value,
    upstream_model_name: &str,
    anthropic_inbound: bool,
    openai_chat_inbound: bool,
    use_responses_upstream: bool,
) -> Value {
    payload["model"] = Value::String(upstream_model_name.to_string());

    if anthropic_inbound {
        if use_responses_upstream {
            payload = super::protocol::convert_anthropic_to_openai_responses(&payload);
        } else {
            payload = super::protocol::convert_anthropic_to_openai(
                &payload,
                super::protocol::AnthropicToOpenAiOptions {
                    emit_reasoning_extensions: true,
                    preserve_reasoning_content: false,
                },
            );
        }
    } else if openai_chat_inbound && use_responses_upstream {
        payload =
            super::protocol::convert_openai_chat_completion_request_to_responses(&payload);
    }

    payload
}
