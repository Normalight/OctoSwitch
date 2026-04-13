use std::fs;

use tauri::State;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::{
    config::app_config::load_gateway_config,
    runtime_events,
    service::{config_service, default_groups_service},
    state::AppState,
};

#[tauri::command]
pub fn export_config(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    config_service::export_config(&conn)
}

#[tauri::command]
pub async fn export_config_to_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let json = {
        let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
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
    .map_err(|e| e.to_string())?;

    let path = path.ok_or("save cancelled")?;
    let path_str = match &path {
        FilePath::Path(p) => p.to_string_lossy().to_string(),
        FilePath::Url(_) => return Err("save cancelled".to_string()),
    };

    fs::write(&path_str, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_config(state: State<AppState>, json: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    config_service::import_config(&conn, &json)?;
    drop(conn);
    runtime_events::notify_config_imported();
    Ok(())
}

#[tauri::command]
pub fn clear_all_data(state: State<AppState>) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    default_groups_service::reset_with_default_model_groups(&mut conn)?;
    drop(conn);
    runtime_events::notify_config_imported();
    Ok(())
}

#[tauri::command]
pub fn import_cc_switch_providers(state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let gw_config = load_gateway_config();
    let report =
        config_service::import_cc_switch_providers(&mut conn, &gw_config.host, gw_config.port)?;
    drop(conn);
    runtime_events::notify_config_imported();
    serde_json::to_value(report).map_err(|e| e.to_string())
}
