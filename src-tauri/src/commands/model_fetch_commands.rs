use tauri::State;

use crate::database::copilot_account_dao;
use crate::log_codes::MDL_FETCH;
use crate::service::provider_service;
use crate::services::copilot_auth;
use crate::services::model_fetch::{self, FetchedModel};
use crate::state::AppState;

#[tauri::command]
pub async fn fetch_upstream_models(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<Vec<FetchedModel>, String> {
    let (provider, copilot_account) = {
        let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
        let provider = provider_service::get_provider(&conn, &provider_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "provider not found".to_string())?;
        let copilot =
            copilot_account_dao::get_by_provider(&conn, &provider_id).map_err(|e| e.to_string())?;
        (provider, copilot)
    };

    log::info!(
        "[{MDL_FETCH}] fetch_upstream_models start provider_id={} name={} copilot_linked={}",
        provider.id,
        provider.name,
        copilot_account.is_some()
    );

    if let Some(acc) = copilot_account {
        let updated = copilot_auth::ensure_copilot_token(&acc)
            .await
            .map_err(|e| e.to_string())?;
        if updated.copilot_token != acc.copilot_token
            || updated.token_expires_at != acc.token_expires_at
        {
            let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
            copilot_account_dao::update(&conn, &updated).map_err(|e| e.to_string())?;
        }
        let copilot_jwt = updated
            .copilot_token
            .as_deref()
            .ok_or_else(|| "Copilot token missing — try refreshing Copilot auth".to_string())?;
        let ids = copilot_auth::fetch_copilot_models(copilot_jwt, updated.api_endpoint.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        let mut out: Vec<FetchedModel> = ids
            .into_iter()
            .map(|id| FetchedModel {
                id,
                owned_by: Some("GitHub Copilot".to_string()),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        log::info!(
            "[{MDL_FETCH}] fetch_upstream_models ok provider_id={} source=copilot count={}",
            provider.id,
            out.len()
        );
        return Ok(out);
    }

    let result = model_fetch::fetch_models(&state.http_client, &provider).await;
    match &result {
        Ok(list) => {
            log::info!(
                "[{MDL_FETCH}] fetch_upstream_models ok provider_id={} source=openai_compat count={}",
                provider.id,
                list.len()
            );
        }
        Err(e) => {
            log::warn!(
                "[{MDL_FETCH}] fetch_upstream_models err provider_id={} source=openai_compat: {e}",
                provider.id
            );
        }
    }
    result
}
