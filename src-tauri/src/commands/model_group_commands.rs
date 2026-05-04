use tauri::State;

use crate::{
    database::{model_binding_dao, model_group_dao, model_group_member_dao},
    domain::model_group::{ModelGroup, NewModelGroup},
    domain::routing::{RoutingGroupStatus, RoutingMemberStatus, RoutingStatus},
    service::routing_service,
    state::AppState,
    tray_support::refresh_tray_menu,
};

#[tauri::command]
pub fn list_model_groups(state: State<AppState>) -> Result<Vec<ModelGroup>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_model_group(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group: NewModelGroup,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let created = model_group_dao::create(&conn, group).map_err(|e| e.to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(created)
}

#[tauri::command]
pub fn update_model_group(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let updated = model_group_dao::update_partial(&conn, &id, patch).map_err(|e| e.to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn delete_model_group(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_dao::delete(&conn, &id).map_err(|e| e.to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_model_group_active_binding(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group_id: String,
    binding_id: String,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let updated =
        model_group_dao::set_active_binding(&conn, &group_id, Some(&binding_id))
            .map_err(|e| e.to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn add_model_group_member(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group_id: String,
    binding_id: String,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    if model_binding_dao::get_by_id(&conn, &binding_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err("Binding not found".to_string());
    }
    model_group_member_dao::add(&conn, &group_id, &binding_id).map_err(|e| e.to_string())?;
    let updated = model_group_dao::get_by_id(&conn, &group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Model group not found".to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn remove_model_group_member(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group_id: String,
    binding_id: String,
) -> Result<ModelGroup, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    model_group_member_dao::remove(&conn, &group_id, &binding_id).map_err(|e| e.to_string())?;
    let updated = model_group_dao::get_by_id(&conn, &group_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Model group not found".to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn get_routing_status(state: State<AppState>) -> Result<RoutingStatus, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    routing_service::get_routing_status(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_group_members_by_alias(
    state: State<AppState>,
    group_alias: String,
) -> Result<Vec<RoutingMemberStatus>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    routing_service::list_group_members_by_alias(&conn, &group_alias).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_group_active_member_by_alias(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group_alias: String,
    member_name: String,
) -> Result<RoutingGroupStatus, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let updated =
        routing_service::set_group_active_member_by_alias(&conn, &group_alias, &member_name)
            .map_err(|e| e.to_string())?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}
