use tauri::State;

use crate::domain::provider::ProviderSummary;
use crate::domain::provider::{Provider, NewProvider};
use crate::{service::provider_service, services::healthcheck_service, state::AppState};

#[tauri::command]
pub fn list_providers(
    state: State<AppState>,
) -> Result<Vec<ProviderSummary>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::list_providers(&conn)
        .map(|providers| providers.iter().map(|p| p.to_summary()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider(
    state: State<AppState>,
    id: String,
) -> Result<Provider, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::get_provider(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "provider not found".to_string())
}

#[tauri::command]
pub fn create_provider(
    state: State<AppState>,
    provider: NewProvider,
) -> Result<ProviderSummary, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::create_provider(&conn, provider)
        .map(|p| p.to_summary())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_provider(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<ProviderSummary, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::update_provider(&conn, &id, patch)
        .map(|p| p.to_summary())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_provider(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::delete_provider(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_provider_health_check(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<healthcheck_service::HealthCheckResult, String> {
    let provider = {
        let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
        provider_service::get_provider(&conn, &provider_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "provider not found".to_string())?
    };
    Ok(healthcheck_service::check_provider(&provider, &state.http_client).await)
}
