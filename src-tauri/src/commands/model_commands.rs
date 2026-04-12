use tauri::State;

use crate::{
    database::model_binding_dao,
    domain::model_binding::{ModelBinding, NewModelBinding},
    state::AppState,
};

#[tauri::command]
pub fn list_model_bindings(state: State<AppState>) -> Result<Vec<ModelBinding>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_binding_dao::list(&conn)
}

#[tauri::command]
pub fn create_model_binding(
    state: State<AppState>,
    binding: NewModelBinding,
) -> Result<ModelBinding, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_binding_dao::create(&conn, binding)
}

#[tauri::command]
pub fn update_model_binding(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<ModelBinding, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_binding_dao::update_partial(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_model_binding(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_binding_dao::delete(&conn, &id)
}
