use std::fs;

use tauri::State;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::{
    config::app_config::load_gateway_config,
    domain::error::AppError,
    runtime_events,
    service::{config_service, default_groups_service},
    state::AppState,
};

#[tauri::command]
pub fn export_config(state: State<AppState>) -> Result<String, AppError> {
    let conn = state.db.get()?;
    Ok(config_service::export_config(&conn)?)
}

#[tauri::command]
pub async fn export_config_to_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let json = {
        let conn = state.db.get()?;
        config_service::export_config(&conn)?
    };

    let path = tokio::task::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .set_file_name("octoswitch-config.json")
                .add_filter("JSON", &["json"])
                .blocking_save_file()
        }
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let path = path.ok_or(AppError::Internal("save cancelled".into()))?;
    let path_str = match &path {
        FilePath::Path(p) => p.to_string_lossy().to_string(),
        FilePath::Url(_) => return Err(AppError::Internal("save cancelled".into())),
    };

    fs::write(&path_str, json).map_err(|e| AppError::from(e.to_string()))
}

#[tauri::command]
pub fn import_config(state: State<AppState>, json: String) -> Result<(), AppError> {
    let conn = state.db.get()?;
    config_service::import_config(&conn, &json)?;
    drop(conn);
    runtime_events::notify_config_imported();
    Ok(())
}

#[tauri::command]
pub fn clear_all_data(state: State<AppState>) -> Result<(), AppError> {
    let mut conn = state.db.get()?;
    default_groups_service::reset_with_default_model_groups(&mut conn)?;
    drop(conn);
    runtime_events::notify_config_imported();
    Ok(())
}

#[tauri::command]
pub fn import_cc_switch_providers(state: State<AppState>) -> Result<serde_json::Value, AppError> {
    let mut conn = state.db.get()?;
    let gw_config = load_gateway_config();
    let report =
        config_service::import_cc_switch_providers(&mut conn, &gw_config.host, gw_config.port)?;
    drop(conn);
    runtime_events::notify_config_imported();
    Ok(serde_json::to_value(report)?)
}
