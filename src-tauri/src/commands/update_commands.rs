//! 更新检查命令：检查 GitHub Releases、忽略版本、获取当前版本信息
//! 并支持直接下载安装更新（下载 → 静默安装 → 自动重启）

use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

use crate::config::app_config::{load_gateway_config, save_gateway_config};
use crate::domain::error::AppError;
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
pub async fn check_for_update(state: State<'_, AppState>) -> Result<UpdateCheckResult, AppError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let cfg = load_gateway_config();
    let ignored_version = cfg.ignored_update_version.clone();

    let tag = fetch_latest_tag(&state.http_client).await?;
    let version = tag.trim_start_matches('v').to_string();
    let notes = fetch_changelog_notes(&state.http_client, &tag).await;
    let installer = probe_installer_url(&state.http_client, &tag).await;
    let release_url = format!("https://github.com/Normalight/OctoSwitch/releases/tag/{tag}");

    let has_update = is_newer_version(&current_version, &version);
    let is_ignored = ignored_version.as_deref() == Some(&version);

    Ok(UpdateCheckResult {
        current_version: current_version.clone(),
        latest_version: version.clone(),
        has_update: has_update && !is_ignored,
        release_notes: notes,
        release_url,
        is_ignored,
        installer_url: installer.as_ref().map(|a| a.url.clone()),
        installer_size: installer.as_ref().map(|a| a.size),
    })
}

/// 忽略指定版本（写入配置）
#[tauri::command]
pub fn ignore_update_version(version: String) -> Result<(), AppError> {
    let mut cfg = load_gateway_config();
    cfg.ignored_update_version = Some(version);
    save_gateway_config(&cfg).map_err(AppError::from)
}

/// 清除忽略的版本（写入配置）
#[tauri::command]
pub fn clear_ignored_update_version() -> Result<(), AppError> {
    let mut cfg = load_gateway_config();
    cfg.ignored_update_version = None;
    save_gateway_config(&cfg).map_err(AppError::from)
}

/// 防止并发调用下载安装（单例锁）
static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// 下载并安装更新：从 GitHub Release 下载安装包，静默运行后自动重启
#[tauri::command]
pub async fn download_and_install_update(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    // Reject if another download is already running
    if DOWNLOAD_IN_PROGRESS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(AppError::Internal("A download is already in progress".into()));
    }

    let result = download_and_install_update_impl(state, app).await;
    DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);
    result
}

async fn download_and_install_update_impl(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    let tag = fetch_latest_tag(&state.http_client).await?;
    let installer = probe_installer_url(&state.http_client, &tag)
        .await
        .ok_or_else(|| AppError::Internal("No installer asset found for this platform in the latest release".into()))?;

    let installer_path =
        download_installer(&app, &state.http_client, &installer.url, installer.size).await?;

    run_installer(&app, &installer_path)
}

struct InstallerAsset {
    url: String,
    size: u64,
}

/// Fetch latest release tag by following GitHub's /releases/latest redirect.
/// No API token required — works for everyone without rate limits.
async fn fetch_latest_tag(client: &reqwest::Client) -> Result<String, AppError> {
    let resp = client
        .get("https://github.com/Normalight/OctoSwitch/releases/latest")
        .header("User-Agent", "OctoSwitch")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to reach GitHub: {e}")))?;

    let final_url = resp.url().to_string();
    // URL format: https://github.com/Normalight/OctoSwitch/releases/tag/vX.Y.Z
    let tag = final_url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();

    if tag.is_empty() || !tag.starts_with('v') {
        return Err(AppError::Internal("Could not determine latest version".into()));
    }

    Ok(tag)
}

/// Try known installer asset URL patterns for the current platform.
/// Returns (url, content_length) if found.
async fn probe_installer_url(
    client: &reqwest::Client,
    tag: &str,
) -> Option<InstallerAsset> {
    let ver = tag.trim_start_matches('v');

    // Build the actual pattern list
    let urls: Vec<String> = {
        let base = format!("https://github.com/Normalight/OctoSwitch/releases/download/{tag}");
        let mut v = Vec::new();

        #[cfg(target_os = "macos")]
        {
            v.push(format!("{base}/OctoSwitch_{ver}_aarch64.dmg"));
            v.push(format!("{base}/OctoSwitch_{ver}_x64.dmg"));
            v.push(format!("{base}/OctoSwitch_aarch64.app.tar.gz"));
            v.push(format!("{base}/OctoSwitch.app.tar.gz"));
        }

        #[cfg(target_os = "linux")]
        {
            v.push(format!("{base}/OctoSwitch_{ver}_amd64.AppImage"));
            v.push(format!("{base}/OctoSwitch_{ver}_amd64.deb"));
            v.push(format!("{base}/OctoSwitch_amd64.AppImage"));
            v.push(format!("{base}/OctoSwitch_amd64.deb"));
        }

        #[cfg(target_os = "windows")]
        {
            v.push(format!("{base}/OctoSwitch_{ver}_x64-setup.exe"));
            v.push(format!("{base}/OctoSwitch_{ver}_x64_en-US.msi"));
            v.push(format!("{base}/OctoSwitch_x64-setup.exe"));
            v.push(format!("{base}/OctoSwitch_x64_en-US.msi"));
        }

        v
    };

    for url in &urls {
        match client.head(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let size = resp.content_length().unwrap_or(0);
                return Some(InstallerAsset { url: url.clone(), size });
            }
            _ => continue,
        }
    }

    None
}

/// Try to fetch release notes from raw CHANGELOG.md (no API needed).
async fn fetch_changelog_notes(client: &reqwest::Client, _tag: &str) -> String {
    match client
        .get("https://raw.githubusercontent.com/Normalight/OctoSwitch/main/CHANGELOG.md")
        .header("User-Agent", "OctoSwitch")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            match resp.text().await {
                Ok(text) => {
                    // Extract the first version section
                    if let Some(start) = text.find("## [v") {
                        if let Some(end) = text[start + 1..].find("\n## [v") {
                            return text[start..start + 1 + end].to_string();
                        }
                    }
                    text.lines().take(30).collect::<Vec<_>>().join("\n")
                }
                Err(_) => String::new(),
            }
        }
        Err(_) => String::new(),
    }
}


/// 流式下载文件，通过 Tauri 事件向前端报告进度
async fn download_installer(
    app: &AppHandle,
    client: &reqwest::Client,
    url: &str,
    total_size: u64,
) -> Result<std::path::PathBuf, AppError> {
    let resp = client
        .get(url)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to start download: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal(format!("Download failed: HTTP {}", resp.status())));
    }

    let file_name = url.rsplit('/').next().unwrap_or("OctoSwitch-setup.exe");
    let target = std::env::temp_dir().join(file_name);

    let mut file =
        tokio::fs::File::create(&target).await.map_err(|e| AppError::Internal(format!("Cannot create {target:?}: {e}")))?;

    let mut downloaded = 0u64;
    let mut last_progress = 0u8;
    let mut stream = resp.bytes_stream();
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(120));

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    loop {
        tokio::select! {
            biased;
            chunk_opt = stream.next() => {
                match chunk_opt {
                    Some(Ok(chunk)) => {
                        file.write_all(&chunk).await.map_err(|e| AppError::Internal(format!("Write error: {e}")))?;
                        downloaded += chunk.len() as u64;
                    }
                    Some(Err(e)) => return Err(AppError::Internal(format!("Download error: {e}"))),
                    None => break,
                }
            }
            _ = tick.tick() => {
                // fallthrough to emit block
            }
        }

        // Emit progress if changed (after either chunk or tick)
        if total_size > 0 {
            let progress = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
            if progress != last_progress || progress == 100 {
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

    file.flush().await.ok();
    drop(file);

    app.emit(
        "update-download-complete",
        serde_json::json!({ "path": target.to_string_lossy() }),
    )
    .ok();

    Ok(target)
}

/// 运行已下载的安装程序（Windows 上以 /S 静默模式运行 NSIS 安装程序）
#[cfg(target_os = "windows")]
fn run_installer(app: &AppHandle, path: &Path) -> Result<(), AppError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    app.emit("update-installer-launching", serde_json::json!({}))
        .ok();

    let mut child = std::process::Command::new(path)
        .arg("/S")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to launch installer: {e}")))?;

    let status = child.wait().map_err(|e| AppError::Internal(e.to_string()))?;

    if !status.success() {
        return Err(AppError::Internal(format!(
            "Installer exited with code {:?}",
            status.code()
        )));
    }

    // 给安装程序一点时间完成文件操作
    std::thread::sleep(std::time::Duration::from_secs(2));

    // 手动启动新实例而非依赖 app.restart()（安装后可能不可靠）
    let current_exe =
        std::env::current_exe().map_err(|e| AppError::Internal(format!("Cannot determine current exe path: {e}")))?;

    log::info!("[update] relaunching from: {:?}", current_exe);

    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to restart app: {e}")))?;

    // 再等一会确保新进程启动
    std::thread::sleep(std::time::Duration::from_millis(500));

    log::info!("[update] exiting old process");
    std::process::exit(0);
}

/// macOS/Linux: handle installation per platform
#[cfg(target_os = "macos")]
fn run_installer(app: &AppHandle, path: &Path) -> Result<(), AppError> {
    let path_str = path.to_string_lossy();
    let path_ref: &str = path_str.as_ref();
    app.emit("update-installer-launching", serde_json::json!({}))
        .ok();

    // Handle DMG: mount → copy .app → unmount → quarantine removal → self-sign → open
    if path_ref.ends_with(".dmg") {
        let output = std::process::Command::new("hdiutil")
            .args(["attach", path_ref, "-nobrowse", "-readonly"])
            .output()
            .map_err(|e| AppError::Internal(format!("Failed to mount DMG: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let vol_line = stdout.lines()
            .find(|l| l.contains("/Volumes/"))
            .ok_or(AppError::Internal("DMG mounted but no volume path found".into()))?;
        let vol_path = vol_line.split('\t').last().unwrap_or(vol_line).trim();

        // Find .app in mounted volume
        let app_name = std::fs::read_dir(vol_path)
            .map_err(|e| AppError::Internal(format!("Cannot read DMG volume: {e}")))?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().ends_with(".app"))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .ok_or(AppError::Internal("No .app found in DMG".into()))?;

        let target = std::path::PathBuf::from("/Applications").join(&app_name);
        // Remove existing installation
        if target.exists() {
            std::fs::remove_dir_all(&target).ok();
        }

        // Copy to /Applications
        std::process::Command::new("cp")
            .args(["-R", &format!("{}/{}", vol_path, app_name), &target.to_string_lossy()])
            .output()
            .map_err(|e| AppError::Internal(format!("Failed to copy app: {e}")))?;

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

        // Launch the new version in a new process group
        std::process::Command::new("open")
            .args(["-n", "-a", &target.to_string_lossy()])
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to launch new version: {e}")))?;

        // Sleep to allow launch services to register the new process
        std::thread::sleep(std::time::Duration::from_secs(2));
        log::info!("[update] installed {} successfully, restarting", app_name);
        std::process::exit(0);
    }

    // Fallback for tar.gz or other formats
    let mut child = std::process::Command::new("open")
        .arg(path)
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to launch installer: {e}")))?;

    child.wait().map_err(|e| AppError::Internal(e.to_string()))?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    let current_exe =
        std::env::current_exe().map_err(|e| AppError::Internal(format!("Cannot determine current exe path: {e}")))?;
    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to restart app: {e}")))?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::process::exit(0);
}

/// Linux fallback path
#[cfg(target_os = "linux")]
fn run_installer(app: &AppHandle, path: &Path) -> Result<(), AppError> {
    app.emit("update-installer-launching", serde_json::json!({}))
        .ok();

    let path_str = path.to_string_lossy();
    let path_ref: &str = path_str.as_ref();

    if path_ref.ends_with(".AppImage") {
        std::process::Command::new("chmod")
            .args(["+x", path_ref])
            .output()
            .ok();
        std::process::Command::new(path_ref)
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to launch AppImage: {e}")))?;
    } else if path_ref.ends_with(".deb") {
        std::process::Command::new("sudo")
            .args(["dpkg", "-i", path_ref])
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to install .deb: {e}")))?;
    } else {
        let mut child = std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to launch installer: {e}")))?;
        child.wait().map_err(|e| AppError::Internal(e.to_string()))?;
    }

    std::thread::sleep(std::time::Duration::from_secs(2));
    let current_exe =
        std::env::current_exe().map_err(|e| AppError::Internal(format!("Cannot determine current exe path: {e}")))?;
    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to restart app: {e}")))?;
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
