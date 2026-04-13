use tauri::State;

use crate::{
    config::app_config::load_gateway_config, domain::plugin_dist::PluginDistBuildResult,
    service::plugin_dist_service, state::AppState,
};

#[tauri::command]
pub fn build_plugin_dist(state: State<AppState>) -> Result<PluginDistBuildResult, String> {
    let cfg = load_gateway_config();
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    plugin_dist_service::build_plugin_dist(&cfg, &conn)
}

#[tauri::command]
pub fn build_marketplace_dist(_state: State<AppState>) -> Result<PluginDistBuildResult, String> {
    let cfg = load_gateway_config();
    plugin_dist_service::build_marketplace_dist(&cfg)
}
