use tauri::State;

use crate::{
    database::model_binding_dao,
    domain::error::AppError,
    domain::model_binding::{ModelBinding, NewModelBinding},
    state::AppState,
    tray_support::refresh_tray_menu,
};

#[tauri::command]
pub fn list_model_bindings(state: State<AppState>) -> Result<Vec<ModelBinding>, AppError> {
    let conn = state.db.get()?;
    model_binding_dao::list(&conn).map_err(AppError::from)
}

#[tauri::command]
pub fn create_model_binding(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    binding: NewModelBinding,
) -> Result<ModelBinding, AppError> {
    let conn = state.db.get()?;
    let created = model_binding_dao::create(&conn, binding)?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(created)
}

#[tauri::command]
pub fn update_model_binding(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<ModelBinding, AppError> {
    let conn = state.db.get()?;
    let updated = model_binding_dao::update_partial(&conn, &id, patch)?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn delete_model_binding(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state.db.get()?;
    model_binding_dao::delete(&conn, &id)?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(())
}
