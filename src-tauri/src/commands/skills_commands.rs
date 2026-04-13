use tauri::State;

use crate::{
    config::app_config::{cc_switch_plugins_dir, load_gateway_config, repo_root_marketplace_manifest_path},
    database::task_route_preference_dao,
    domain::local_skill::{LocalPluginStatus, LocalPluginSyncResult},
    domain::task_route_preference::{NewTaskRoutePreference, TaskRoutePreference},
    service::{local_skills_service, plugin_dist_service},
    state::AppState,
};

fn auto_sync_if_needed(state: &State<AppState>) {
    let Ok(marketplace_manifest_path) = repo_root_marketplace_manifest_path().into_os_string().into_string() else { return };
    let plugins_root = cc_switch_plugins_dir();
    let gateway_config = load_gateway_config();
    let Ok(conn) = state.db.lock() else { return };
    let Ok(runtime_config) = plugin_dist_service::get_runtime_plugin_config(&gateway_config, &conn) else { return };
    let _ = local_skills_service::auto_sync_plugin_files(
        &marketplace_manifest_path,
        &plugins_root.to_string_lossy(),
        "octoswitch",
        &runtime_config,
    );
}

#[tauri::command]
pub fn list_task_route_preferences(
    state: State<AppState>,
) -> Result<Vec<TaskRoutePreference>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    task_route_preference_dao::list(&conn)
}

#[tauri::command]
pub fn create_task_route_preference(
    state: State<AppState>,
    preference: NewTaskRoutePreference,
) -> Result<TaskRoutePreference, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let result = task_route_preference_dao::create(&conn, preference)?;
    drop(conn);
    auto_sync_if_needed(&state);
    Ok(result)
}

#[tauri::command]
pub fn update_task_route_preference(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<TaskRoutePreference, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let result = task_route_preference_dao::update_partial(&conn, &id, patch)?;
    drop(conn);
    auto_sync_if_needed(&state);
    Ok(result)
}

#[tauri::command]
pub fn delete_task_route_preference(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let result = task_route_preference_dao::delete(&conn, &id)?;
    drop(conn);
    auto_sync_if_needed(&state);
    Ok(result)
}

#[tauri::command]
pub fn inspect_cc_switch_octoswitch_plugin(state: State<AppState>) -> Result<LocalPluginStatus, String> {
    let marketplace_manifest_path = repo_root_marketplace_manifest_path();
    let plugins_root = cc_switch_plugins_dir();
    let gateway_config = load_gateway_config();
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let runtime_config = plugin_dist_service::get_runtime_plugin_config(&gateway_config, &conn)?;
    local_skills_service::inspect_cc_switch_plugin_status(
        &marketplace_manifest_path.to_string_lossy(),
        &plugins_root.to_string_lossy(),
        "octoswitch",
        &runtime_config,
    )
}

#[tauri::command]
pub fn sync_cc_switch_octoswitch_plugin(state: State<AppState>) -> Result<LocalPluginSyncResult, String> {
    let marketplace_manifest_path = repo_root_marketplace_manifest_path();
    let plugins_root = cc_switch_plugins_dir();
    let gateway_config = load_gateway_config();
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let runtime_config = plugin_dist_service::get_runtime_plugin_config(&gateway_config, &conn)?;
    local_skills_service::sync_cc_switch_plugin_from_marketplace(
        &marketplace_manifest_path.to_string_lossy(),
        &plugins_root.to_string_lossy(),
        "octoswitch",
        &runtime_config,
    )
}
