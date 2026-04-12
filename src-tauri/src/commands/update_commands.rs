//! 更新检查命令：检查 GitHub Releases、忽略版本、获取当前版本信息

use serde::Serialize;
use tauri::State;

use crate::config::app_config::{load_gateway_config, save_gateway_config};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: String,
    pub release_url: String,
    /// 用户是否忽略了最新版本
    pub is_ignored: bool,
}

/// 获取当前应用版本（从 Cargo.toml 的 version 字段）
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 检查 GitHub Releases 是否有新版本
#[tauri::command]
pub async fn check_for_update(_state: State<'_, AppState>) -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let cfg = load_gateway_config();
    let ignored_version = cfg.ignored_update_version.clone();

    let latest = fetch_latest_release().await?;

    let has_update = is_newer_version(&current_version, &latest.version);
    let is_ignored = ignored_version.as_deref() == Some(&latest.version);

    Ok(UpdateCheckResult {
        current_version: current_version.clone(),
        latest_version: latest.version.clone(),
        has_update: has_update && !is_ignored,
        release_notes: latest.notes,
        release_url: latest.url,
        is_ignored,
    })
}

/// 忽略指定版本（写入配置）
#[tauri::command]
pub fn ignore_update_version(version: String) -> Result<(), String> {
    let mut cfg = load_gateway_config();
    cfg.ignored_update_version = Some(version);
    save_gateway_config(&cfg)
}

/// 清除忽略的版本（写入配置）
#[tauri::command]
pub fn clear_ignored_update_version() -> Result<(), String> {
    let mut cfg = load_gateway_config();
    cfg.ignored_update_version = None;
    save_gateway_config(&cfg)
}

struct ReleaseInfo {
    version: String,
    notes: String,
    url: String,
}

/// 从 GitHub Releases API 获取最新版本
async fn fetch_latest_release() -> Result<ReleaseInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get("https://api.github.com/repos/Normalight/OctoSwitch/releases/latest")
        .header("User-Agent", "OctoSwitch")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let version = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .trim_start_matches('v')
        .to_string();

    let notes = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let url = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/Normalight/OctoSwitch/releases")
        .to_string();

    Ok(ReleaseInfo {
        version,
        notes,
        url,
    })
}

/// 比较两个 semver 版本，判断 b 是否比 a 新
fn is_newer_version(a: &str, b: &str) -> bool {
    let parse_parts = |v: &str| -> Vec<u64> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let a_parts = parse_parts(a);
    let b_parts = parse_parts(b);

    // 逐段比较：major, minor, patch
    for i in 0..3 {
        let a_val = a_parts.get(i).copied().unwrap_or(0);
        let b_val = b_parts.get(i).copied().unwrap_or(0);
        if b_val > a_val {
            return true;
        }
        if b_val < a_val {
            return false;
        }
    }

    false
}
