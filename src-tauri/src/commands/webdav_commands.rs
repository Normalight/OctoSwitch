use tauri::State;

use crate::config::webdav_config::{self, WebDavConfig};
use crate::domain::error::AppError;
use crate::runtime_events;
use crate::service::config_service;
use crate::state::AppState;

fn require_configured() -> Result<WebDavConfig, AppError> {
    webdav_config::load().ok_or_else(|| AppError::Internal("未配置 WebDAV 同步".into()))
}

#[tauri::command]
pub async fn webdav_test_connection(
    state: State<'_, AppState>,
    config: WebDavConfig,
) -> Result<serde_json::Value, AppError> {
    config.validate()?;
    webdav_sync::check_connection(&state.http_client, &config).await?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn webdav_get_settings() -> Result<serde_json::Value, AppError> {
    match webdav_config::load() {
        Some(config) => Ok(serde_json::json!({
            "baseUrl": config.base_url,
            "username": config.username,
            "password": "",
            "remoteRoot": config.remote_root,
            "isConfigured": config.is_configured(),
        })),
        None => Ok(serde_json::json!({
            "baseUrl": "",
            "username": "",
            "password": "",
            "remoteRoot": "octoswitch-sync",
            "isConfigured": false,
        })),
    }
}

#[tauri::command]
pub fn webdav_save_settings(config: WebDavConfig) -> Result<serde_json::Value, AppError> {
    config.validate()?;
    webdav_config::save(&config)?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn webdav_upload(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    let config = require_configured()?;
    let conn = state.db.get().map_err(|e| AppError::Database(e.to_string()))?;
    let config_json = config_service::export_config(&conn).map_err(AppError::Internal)?;
    drop(conn);
    webdav_sync::upload(&state.http_client, &config, config_json).await
}

#[tauri::command]
pub async fn webdav_download(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    let config = require_configured()?;
    let json_str = webdav_sync::download(&state.http_client, &config).await?;
    let conn = state.db.get().map_err(|e| AppError::Database(e.to_string()))?;
    config_service::import_config(&conn, &json_str).map_err(AppError::Internal)?;
    drop(conn);
    runtime_events::notify_config_imported();
    Ok(serde_json::json!({ "status": "downloaded" }))
}

use crate::services::webdav_sync;
