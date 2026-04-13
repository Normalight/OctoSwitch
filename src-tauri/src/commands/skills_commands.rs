use tauri::State;

use crate::{
    config::app_config::{cc_switch_skills_dir, repo_root_skills_dir},
    database::task_route_preference_dao,
    domain::local_skill::LocalSkillsStatus,
    domain::task_route_preference::{NewTaskRoutePreference, TaskRoutePreference},
    service::local_skills_service,
    state::AppState,
};

#[tauri::command]
pub fn list_task_route_preferences(state: State<AppState>) -> Result<Vec<TaskRoutePreference>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    task_route_preference_dao::list(&conn)
}

#[tauri::command]
pub fn create_task_route_preference(
    state: State<AppState>,
    preference: NewTaskRoutePreference,
) -> Result<TaskRoutePreference, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    task_route_preference_dao::create(&conn, preference)
}

#[tauri::command]
pub fn update_task_route_preference(
    state: State<AppState>,
    id: String,
    patch: serde_json::Value,
) -> Result<TaskRoutePreference, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    task_route_preference_dao::update_partial(&conn, &id, patch)
}

#[tauri::command]
pub fn delete_task_route_preference(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    task_route_preference_dao::delete(&conn, &id)
}

#[tauri::command]
pub fn inspect_local_skills_paths(
    source_path: String,
    installed_path: String,
) -> Result<LocalSkillsStatus, String> {
    Ok(local_skills_service::inspect_skills_paths(
        &source_path,
        &installed_path,
    ))
}

#[tauri::command]
pub fn quick_install_repo_skills_to_cc_switch() -> Result<LocalSkillsStatus, String> {
    let repo_skills_path = repo_root_skills_dir();
    let cc_switch_path = cc_switch_skills_dir();
    local_skills_service::install_repo_skills_to_path(
        &repo_skills_path.to_string_lossy(),
        &cc_switch_path.to_string_lossy(),
    )?;
    Ok(local_skills_service::inspect_skills_paths(
        &cc_switch_path.to_string_lossy(),
        &cc_switch_path.to_string_lossy(),
    ))
}
