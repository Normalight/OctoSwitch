use tauri::{Emitter, State};

use crate::{config::app_config::load_gateway_config, database, service::config_service, state::AppState};

#[tauri::command]
pub fn export_config(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    config_service::export_config(&conn)
}

#[tauri::command]
pub fn import_config(app: tauri::AppHandle, state: State<AppState>, json: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    config_service::import_config(&conn, &json)?;
    app.emit("os-config-imported", ()).ok();
    Ok(())
}

#[tauri::command]
pub fn clear_all_data(state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    database::clear_all_data(&conn)
}

#[tauri::command]
pub fn import_cc_switch_providers(app: tauri::AppHandle, state: State<AppState>) -> Result<serde_json::Value, String> {
    let mut conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let gw_config = load_gateway_config();
    let report = config_service::import_cc_switch_providers(&mut conn, &gw_config.host, gw_config.port)?;
    app.emit("os-config-imported", ()).ok();
    serde_json::to_value(report).map_err(|e| e.to_string())
}
