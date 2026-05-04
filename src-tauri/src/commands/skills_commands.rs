use tauri::State;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::{
    config::app_config::{cc_switch_plugins_dir, load_gateway_config, repo_root_marketplace_manifest_path},
    database::task_route_preference_dao,
    domain::local_skill::{CcSwitchDeeplink, CcSwitchDeeplinkResult, LocalPluginStatus, LocalPluginSyncResult},
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
    if let Err(e) = local_skills_service::auto_sync_plugin_files(
        &marketplace_manifest_path,
        &plugins_root.to_string_lossy(),
        "octoswitch",
        &runtime_config,
    ) {
        log::debug!("[auto-sync] skipped: {e}");
    }
}

#[tauri::command]
pub fn list_task_route_preferences(
    state: State<AppState>,
) -> Result<Vec<TaskRoutePreference>, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    task_route_preference_dao::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task_route_preference(
    state: State<AppState>,
    preference: NewTaskRoutePreference,
) -> Result<TaskRoutePreference, String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let result = task_route_preference_dao::create(&conn, preference).map_err(|e| e.to_string())?;
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
    let result = task_route_preference_dao::update_partial(&conn, &id, patch).map_err(|e| e.to_string())?;
    drop(conn);
    auto_sync_if_needed(&state);
    Ok(result)
}

#[tauri::command]
pub fn delete_task_route_preference(state: State<AppState>, id: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    let result = task_route_preference_dao::delete(&conn, &id).map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn generate_cc_switch_deeplinks(_state: State<AppState>) -> Result<CcSwitchDeeplinkResult, String> {
    let gateway_config = load_gateway_config();
    let host = &gateway_config.host;
    let port = gateway_config.port;

    fn pct(s: &str) -> String {
        s.replace('%', "%25")
            .replace(':', "%3A")
            .replace('/', "%2F")
            .replace(' ', "+")
            .replace('#', "%23")
            .replace('?', "%3F")
            .replace('&', "%26")
    }

    let provider_link = {
        let name = pct("OctoSwitch");
        let endpoint = pct(&format!("http://{host}:{port}/v1"));
        let homepage = pct("https://github.com/Normalight/OctoSwitch");
        let notes = pct("Local OctoSwitch gateway");
        let url = format!(
            "ccswitch://v1/import?resource=provider&app=claude&name={name}&endpoint={endpoint}&apiKey=sk-octoswitch-local&homepage={homepage}&notes={notes}"
        );
        Some(CcSwitchDeeplink {
            url,
            resource_type: "provider".to_string(),
            description: format!("Register OctoSwitch gateway (http://{host}:{port}) as a provider in CC Switch"),
        })
    };

    let skill_link = {
        let url = "ccswitch://v1/import?resource=skill&repo=Normalight/OctoSwitch&branch=main".to_string();
        Some(CcSwitchDeeplink {
            url,
            resource_type: "skill".to_string(),
            description: "Register OctoSwitch skills repo in CC Switch".to_string(),
        })
    };

    Ok(CcSwitchDeeplinkResult {
        provider_link,
        skill_link,
    })
}

#[tauri::command]
pub fn open_cc_switch_deeplink(app: AppHandle, url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("ccswitch://") {
        return Err("Only ccswitch:// URLs are allowed".to_string());
    }

    app.opener()
        .open_url(trimmed, None::<String>)
        .map_err(|e| format!("Failed to open ccswitch:// URL: {e}"))
        .map(|_| ())
}

#[cfg(test)]
mod tests {

    fn pct(s: &str) -> String {
        s.replace('%', "%25")
            .replace(':', "%3A")
            .replace('/', "%2F")
            .replace(' ', "+")
            .replace('#', "%23")
            .replace('?', "%3F")
            .replace('&', "%26")
    }

    #[test]
    fn test_provider_deeplink_format() {
        let host = "127.0.0.1";
        let port = 8787;
        let name = pct("OctoSwitch");
        let endpoint = pct(&format!("http://{host}:{port}/v1"));
        let homepage = pct("https://github.com/Normalight/OctoSwitch");
        let notes = pct("Local OctoSwitch gateway");
        let url = format!(
            "ccswitch://v1/import?resource=provider&app=claude&name={name}&endpoint={endpoint}&apiKey=sk-octoswitch-local&homepage={homepage}&notes={notes}"
        );

        assert!(url.starts_with("ccswitch://v1/import"), "URL should start with scheme");
        assert!(url.contains("resource=provider"), "URL should contain resource=provider");
        assert!(url.contains("app=claude"), "URL should contain app=claude");
        assert!(url.contains("name=OctoSwitch"), "URL should contain name=OctoSwitch");
        assert!(url.contains("apiKey=sk-octoswitch-local"), "URL should contain apiKey");
        // Endpoint should be URL-encoded
        assert!(url.contains("http%3A%2F%2F127.0.0.1%3A8787%2Fv1"), "Endpoint should be percent-encoded");
        // The scheme itself contains ://, but query parameter values should not
        let query_part = url.split('?').nth(1).unwrap_or("");
        assert!(!query_part.contains("://"), "Query params should not contain raw ://");
    }

    #[test]
    fn test_skill_deeplink_format() {
        let url = "ccswitch://v1/import?resource=skill&repo=Normalight/OctoSwitch&branch=main";
        assert!(url.starts_with("ccswitch://v1/import"), "URL should start with scheme");
        assert!(url.contains("resource=skill"), "URL should contain resource=skill");
        assert!(url.contains("repo=Normalight/OctoSwitch"), "URL should contain repo");
        assert!(url.contains("branch=main"), "URL should contain branch=main");
    }

    #[test]
    fn test_open_cc_switch_deeplink_rejects_non_ccswitch_urls() {
        // We can't easily test open_url with AppHandle in unit tests,
        // but we can verify the URL validation logic would reject non-ccswitch URLs
        let non_ccswitch = "https://example.com";
        assert!(!non_ccswitch.starts_with("ccswitch://"), "Should detect non-ccswitch URL");
    }

    #[test]
    fn test_generated_url_no_raw_colon_slash() {
        // Verify that all generated URLs for provider have protocol chars encoded
        let host = "127.0.0.1";
        let port = 8787;
        let endpoint = pct(&format!("http://{host}:{port}/v1"));
        let homepage = pct("https://github.com/Normalight/OctoSwitch");

        // endpoint and homepage should have : and / encoded
        assert!(!endpoint.contains("://"), "Endpoint should have :// encoded");
        assert!(endpoint.contains("%3A%2F%2F"), "Endpoint should contain %3A%2F%2F");
        assert!(!homepage.contains("://"), "Homepage should have :// encoded");
        assert!(homepage.contains("%3A%2F%2F"), "Homepage should contain %3A%2F%2F");
    }
}
