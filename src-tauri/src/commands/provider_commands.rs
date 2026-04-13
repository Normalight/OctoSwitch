use tauri::State;

use crate::{service::provider_service, services::healthcheck_service, state::AppState};

#[tauri::command]
pub fn list_providers(
    state: State<AppState>,
) -> Result<Vec<crate::domain::provider::Provider>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::list_providers(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_provider(
    state: State<AppState>,
    provider: crate::domain::provider::NewProvider,
) -> Result<crate::domain::provider::Provider, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::create_provider(&conn, provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_provider(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<crate::domain::provider::Provider, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    provider_service::update_provider(&conn, &id, patch).map_err(|e| e.to_string())
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
