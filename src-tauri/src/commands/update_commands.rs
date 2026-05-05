//! 更新检查命令：检查 GitHub Releases、忽略版本、获取当前版本信息
//! 并支持直接下载安装更新（下载 → 静默安装 → 自动重启）

use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

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
    /// 可直接下载的 Windows 安装包地址（如果 release 中有 .exe 资源）
    pub installer_url: Option<String>,
    /// 安装包大小（用于前端显示进度）
    pub installer_size: Option<u64>,
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

    let json = fetch_latest_release_json().await?;

    let latest = parse_release_json(&json);
    let installer = extract_installer_asset(&json);

    let has_update = is_newer_version(&current_version, &latest.version);
    let is_ignored = ignored_version.as_deref() == Some(&latest.version);

    Ok(UpdateCheckResult {
        current_version: current_version.clone(),
        latest_version: latest.version.clone(),
        has_update: has_update && !is_ignored,
        release_notes: latest.notes,
        release_url: latest.url,
        is_ignored,
        installer_url: installer.as_ref().map(|a| a.url.clone()),
        installer_size: installer.as_ref().map(|a| a.size),
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

/// 防止并发调用下载安装（单例锁）
static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// 下载并安装更新：从 GitHub Release 下载安装包，静默运行后自动重启
#[tauri::command]
pub async fn download_and_install_update(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Reject if another download is already running
    if DOWNLOAD_IN_PROGRESS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("A download is already in progress".to_string());
    }

    let result = download_and_install_update_impl(state, app).await;
    DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);
    result
}

async fn download_and_install_update_impl(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let json = fetch_latest_release_json().await?;
    let installer = extract_installer_asset(&json)
        .ok_or_else(|| "No installer asset found for this platform in the latest release".to_string())?;

    let installer_path =
        download_installer(&app, &state.http_client, &installer.url, installer.size).await?;

    run_installer(&app, &installer_path)
}

struct ReleaseInfo {
    version: String,
    notes: String,
    url: String,
}

struct InstallerAsset {
    url: String,
    size: u64,
}

/// 从 GitHub Releases API 获取最新版本（返回原始 JSON）
async fn fetch_latest_release_json() -> Result<serde_json::Value, String> {
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

    resp.json().await.map_err(|e| e.to_string())
}

/// 从 release JSON 解析基本信息（版本号、更新日志、链接）
fn parse_release_json(json: &serde_json::Value) -> ReleaseInfo {
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

    ReleaseInfo {
        version,
        notes,
        url,
    }
}

/// 从 release 的 assets 数组中找到当前平台的安装包
#[cfg(target_os = "macos")]
fn pick_installer_asset(name: &str) -> bool {
    name.ends_with(".dmg") || name.ends_with(".app.tar.gz")
}

#[cfg(target_os = "linux")]
fn pick_installer_asset(name: &str) -> bool {
    name.ends_with(".AppImage") || name.ends_with(".deb")
}

#[cfg(target_os = "windows")]
fn pick_installer_asset(name: &str) -> bool {
    name.ends_with(".exe") || name.ends_with(".msi")
}

fn extract_installer_asset(json: &serde_json::Value) -> Option<InstallerAsset> {
    let assets = json.get("assets").and_then(|a| a.as_array())?;

    for asset in assets {
        let name = asset.get("name").and_then(|v| v.as_str())?;
        if pick_installer_asset(name) {
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())?
                .to_string();
            let size = asset.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            return Some(InstallerAsset { url, size });
        }
    }

    None
}

/// 流式下载文件，通过 Tauri 事件向前端报告进度
async fn download_installer(
    app: &AppHandle,
    client: &reqwest::Client,
    url: &str,
    total_size: u64,
) -> Result<std::path::PathBuf, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let file_name = url.rsplit('/').next().unwrap_or("OctoSwitch-setup.exe");
    let target = std::env::temp_dir().join(file_name);

    let mut file =
        std::fs::File::create(&target).map_err(|e| format!("Cannot create {target:?}: {e}"))?;

    let mut downloaded = 0u64;
    let mut last_progress = 0u8;
    let mut stream = resp.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {e}"))?;
        std::io::Write::write_all(&mut file, &chunk)
            .map_err(|e| format!("Write error: {e}"))?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
            if progress >= last_progress + 5 || progress == 100 {
                last_progress = progress;
                app.emit(
                    "update-download-progress",
                    serde_json::json!({
                        "progress": progress,
                        "downloaded_bytes": downloaded,
                        "total_bytes": total_size,
                    }),
                )
                .ok();
            }
        }
    }

    app.emit(
        "update-download-complete",
        serde_json::json!({ "path": target.to_string_lossy() }),
    )
    .ok();

    Ok(target)
}

/// 运行已下载的安装程序（Windows 上以 /S 静默模式运行 NSIS 安装程序）
#[cfg(target_os = "windows")]
fn run_installer(app: &AppHandle, path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    app.emit("update-installer-launching", serde_json::json!({}))
        .ok();

    let mut child = std::process::Command::new(path)
        .arg("/S")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {e}"))?;

    let status = child.wait().map_err(|e| e.to_string())?;

    if !status.success() {
        return Err(format!(
            "Installer exited with code {:?}",
            status.code()
        ));
    }

    // 给安装程序一点时间完成文件操作
    std::thread::sleep(std::time::Duration::from_secs(2));

    // 手动启动新实例而非依赖 app.restart()（安装后可能不可靠）
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot determine current exe path: {e}"))?;

    log::info!("[update] relaunching from: {:?}", current_exe);

    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Failed to restart app: {e}"))?;

    // 再等一会确保新进程启动
    std::thread::sleep(std::time::Duration::from_millis(500));

    log::info!("[update] exiting old process");
    std::process::exit(0);
}

/// macOS/Linux: handle installation per platform
#[cfg(target_os = "macos")]
fn run_installer(app: &AppHandle, path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    app.emit("update-installer-launching", serde_json::json!({}))
        .ok();

    // Handle DMG: mount → copy .app → unmount → quarantine removal → self-sign → open
    if path_str.ends_with(".dmg") {
        let output = std::process::Command::new("hdiutil")
            .args(["attach", &path_str, "-nobrowse", "-readonly"])
            .output()
            .map_err(|e| format!("Failed to mount DMG: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let vol_line = stdout.lines()
            .find(|l| l.contains("/Volumes/"))
            .ok_or("DMG mounted but no volume path found")?;
        let vol_path = vol_line.split('\t').last().unwrap_or(vol_line).trim();

        // Find .app in mounted volume
        let app_name = std::fs::read_dir(vol_path)
            .map_err(|e| format!("Cannot read DMG volume: {e}"))?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().ends_with(".app"))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .ok_or("No .app found in DMG")?;

        let target = std::path::PathBuf::from("/Applications").join(&app_name);
        // Remove existing installation
        if target.exists() {
            std::fs::remove_dir_all(&target).ok();
        }

        // Copy to /Applications
        std::process::Command::new("cp")
            .args(["-R", &format!("{}/{}", vol_path, app_name), &target.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to copy app: {e}"))?;

        // Unmount DMG
        std::process::Command::new("hdiutil")
            .args(["detach", vol_path, "-quiet"])
            .output()
            .ok();

        // Bypass Gatekeeper
        std::process::Command::new("xattr")
            .args(["-cr", &target.to_string_lossy()])
            .output()
            .ok();
        std::process::Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-", &target.to_string_lossy()])
            .output()
            .ok();

        // Launch the new version
        std::process::Command::new("open")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Failed to launch new version: {e}"))?;

        // Give it a moment then exit
        std::thread::sleep(std::time::Duration::from_secs(1));
        log::info!("[update] installed {} successfully, exiting old process", app_name);
        std::process::exit(0);
    }

    // Fallback for tar.gz or other formats
    let mut child = std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {e}"))?;

    child.wait().map_err(|e| e.to_string())?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot determine current exe path: {e}"))?;
    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Failed to restart app: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::process::exit(0);
}

/// Linux fallback path
#[cfg(target_os = "linux")]
fn run_installer(app: &AppHandle, path: &Path) -> Result<(), String> {
    app.emit("update-installer-launching", serde_json::json!({}))
        .ok();

    let path_str = path.to_string_lossy();

    if path_str.ends_with(".AppImage") {
        // Make executable and launch (AppImage is self-contained)
        std::process::Command::new("chmod")
            .args(["+x", &path_str])
            .output()
            .ok();
        std::process::Command::new(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to launch AppImage: {e}"))?;
    } else if path_str.ends_with(".deb") {
        std::process::Command::new("sudo")
            .args(["dpkg", "-i", &path_str])
            .spawn()
            .map_err(|e| format!("Failed to install .deb: {e}"))?;
    } else {
        let mut child = std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {e}"))?;
        child.wait().map_err(|e| e.to_string())?;
    }

    std::thread::sleep(std::time::Duration::from_secs(2));
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Cannot determine current exe path: {e}"))?;
    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("Failed to restart app: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::process::exit(0);
}

/// 比较两个 semver 版本，判断 b 是否比 a 新
fn is_newer_version(a: &str, b: &str) -> bool {
    let parse_parts =
        |v: &str| -> Vec<u64> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

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
