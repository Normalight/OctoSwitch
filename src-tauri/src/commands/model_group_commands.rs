use tauri::State;

use crate::{
    database::{model_binding_dao, model_group_dao, model_group_member_dao},
    domain::model_group::{ModelGroup, NewModelGroup},
    state::AppState,
};

#[tauri::command]
pub fn list_model_groups(state: State<AppState>) -> Result<Vec<ModelGroup>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::list(&conn)
}

#[tauri::command]
pub fn create_model_group(state: State<AppState>, group: NewModelGroup) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::create(&conn, group)
}

#[tauri::command]
pub fn update_model_group(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::update_partial(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_model_group(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::delete(&conn, &id)
}

#[tauri::command]
pub fn set_model_group_active_binding(
    state: State<AppState>,
    group_id: String,
    binding_id: String,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::set_active_binding(&conn, &group_id, Some(&binding_id))
}

#[tauri::command]
pub fn add_model_group_member(
    state: State<AppState>,
    group_id: String,
    binding_id: String,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    if model_binding_dao::get_by_id(&conn, &binding_id)?.is_none() {
        return Err("未找到该绑定".to_string());
    }
    model_group_member_dao::add(&conn, &group_id, &binding_id)?;
    model_group_dao::get_by_id(&conn, &group_id)?
        .ok_or_else(|| "未找到模型分组".to_string())
}

#[tauri::command]
pub fn remove_model_group_member(
    state: State<AppState>,
    group_id: String,
    binding_id: String,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_member_dao::remove(&conn, &group_id, &binding_id)?;
    model_group_dao::get_by_id(&conn, &group_id)?
        .ok_or_else(|| "未找到模型分组".to_string())
}
