use tauri::State;

use crate::domain::error::AppError;
use crate::domain::provider::ProviderSummary;
use crate::domain::provider::{Provider, NewProvider};
use crate::{service::provider_service, services::healthcheck_service, state::AppState};

#[tauri::command]
pub fn list_providers(
    state: State<AppState>,
) -> Result<Vec<ProviderSummary>, AppError> {
    let conn = state.db.get()?;
    let providers = provider_service::list_providers(&conn)?;
    Ok(providers.iter().map(|p| p.to_summary()).collect())
}

#[tauri::command]
pub fn get_provider(
    state: State<AppState>,
    id: String,
) -> Result<Provider, AppError> {
    let conn = state.db.get()?;
    provider_service::get_provider(&conn, &id)?
        .ok_or(AppError::ProviderNotFound { provider_id: id })
}

#[tauri::command]
pub fn create_provider(
    state: State<AppState>,
    provider: NewProvider,
) -> Result<ProviderSummary, AppError> {
    let conn = state.db.get()?;
    let p = provider_service::create_provider(&conn, provider)?;
    Ok(p.to_summary())
}

#[tauri::command]
pub fn update_provider(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<ProviderSummary, AppError> {
    let conn = state.db.get()?;
    let p = provider_service::update_provider(&conn, &id, patch)?;
    Ok(p.to_summary())
}

#[tauri::command]
pub fn delete_provider(state: State<AppState>, id: String) -> Result<(), AppError> {
    let conn = state.db.get()?;
    provider_service::delete_provider(&conn, &id)
}

#[tauri::command]
pub async fn run_provider_health_check(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<healthcheck_service::HealthCheckResult, AppError> {
    let provider = {
        let conn = state.db.get()?;
        provider_service::get_provider(&conn, &provider_id)?
            .ok_or_else(|| AppError::ProviderNotFound { provider_id: provider_id.clone() })?
    };
    Ok(healthcheck_service::check_provider(&provider, &state.http_client).await)
}
