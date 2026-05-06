use tauri::State;

use crate::{
    database::{model_binding_dao, model_group_dao, model_group_member_dao},
    domain::error::AppError,
    domain::model_group::{ModelGroup, NewModelGroup},
    domain::routing::{RoutingGroupStatus, RoutingMemberStatus, RoutingStatus},
    service::routing_service,
    state::AppState,
    tray_support::refresh_tray_menu,
};

#[tauri::command]
pub fn list_model_groups(state: State<AppState>) -> Result<Vec<ModelGroup>, AppError> {
    let conn = state.db.get()?;
    Ok(model_group_dao::list(&conn)?)
}

#[tauri::command]
pub fn create_model_group(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group: NewModelGroup,
) -> Result<ModelGroup, AppError> {
    let conn = state.db.get()?;
    let created = model_group_dao::create(&conn, group)?;
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
) -> Result<ModelGroup, AppError> {
    let conn = state.db.get()?;
    let updated = model_group_dao::update_partial(&conn, &id, patch)?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn delete_model_group(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state.db.get()?;
    model_group_dao::delete(&conn, &id)?;
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
) -> Result<ModelGroup, AppError> {
    let conn = state.db.get()?;
    let updated =
        model_group_dao::set_active_binding(&conn, &group_id, Some(&binding_id))?;
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
) -> Result<ModelGroup, AppError> {
    let conn = state.db.get()?;
    if model_binding_dao::get_by_id(&conn, &binding_id)?
        .is_none()
    {
        return Err(AppError::Internal("Binding not found".into()));
    }
    model_group_member_dao::add(&conn, &group_id, &binding_id)?;
    let updated = model_group_dao::get_by_id(&conn, &group_id)?
        .ok_or_else(|| AppError::Internal("Model group not found".into()))?;
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
) -> Result<ModelGroup, AppError> {
    let conn = state.db.get()?;
    model_group_member_dao::remove(&conn, &group_id, &binding_id)?;
    let updated = model_group_dao::get_by_id(&conn, &group_id)?
        .ok_or_else(|| AppError::Internal("Model group not found".into()))?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}

#[tauri::command]
pub fn get_routing_status(state: State<AppState>) -> Result<RoutingStatus, AppError> {
    let conn = state.db.get()?;
    routing_service::get_routing_status(&conn)
}

#[tauri::command]
pub fn list_group_members_by_alias(
    state: State<AppState>,
    group_alias: String,
) -> Result<Vec<RoutingMemberStatus>, AppError> {
    let conn = state.db.get()?;
    routing_service::list_group_members_by_alias(&conn, &group_alias)
}

#[tauri::command]
pub fn set_group_active_member_by_alias(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    group_alias: String,
    member_name: String,
) -> Result<RoutingGroupStatus, AppError> {
    let conn = state.db.get()?;
    let updated =
        routing_service::set_group_active_member_by_alias(&conn, &group_alias, &member_name)?;
    drop(conn);
    refresh_tray_menu(&app_handle);
    Ok(updated)
}
