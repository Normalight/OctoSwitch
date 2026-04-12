#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod database;
mod domain;
mod gateway;
mod log_codes;
mod repository;
mod service;
mod services;
mod state;
mod tray_support;

use std::sync::{Arc, Mutex};
use std::{fs, path::Path};

use rusqlite::Connection;
use state::AppState;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::Manager;
use tokio::sync::{mpsc, oneshot};

use crate::config::app_config::{load_gateway_config, AppConfig, GatewayConfig};

use tray_support::{show_main_window, show_tray_icon, MAIN_WINDOW_LABEL, TRAY_ICON_ID};

/// 写入系统自启动项时的附带参数，用于区分「从自启动拉起」与「用户手动启动」
const AUTOSTART_MARKER_ARG: &str = "--octoswitch-autostart";

fn copy_if_exists(src: &Path, dst: &Path) -> Result<(), String> {
    if src.exists() {
        fs::copy(src, dst).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn migrate_legacy_db_if_needed(target_db: &Path) -> Result<(), String> {
    if target_db.exists() {
        return Ok(());
    }

    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let legacy_appdata = crate::config::app_config::legacy_app_data_dir_litellm();
    let legacy_candidates = [
        cwd.join("octoswitch.db"),
        cwd.join("src-tauri").join("octoswitch.db"),
        legacy_appdata.join("octoswitch.db"),
        legacy_appdata.join("litellm.db"),
    ];

    for src_db in legacy_candidates {
        if src_db == target_db || !src_db.exists() {
            continue;
        }

        fs::copy(&src_db, target_db).map_err(|e| e.to_string())?;
        let src_wal = src_db.with_file_name(format!(
            "{}-wal",
            src_db.file_name().and_then(|n| n.to_str()).unwrap_or("octoswitch.db")
        ));
        let src_shm = src_db.with_file_name(format!(
            "{}-shm",
            src_db.file_name().and_then(|n| n.to_str()).unwrap_or("octoswitch.db")
        ));
        let dst_wal = target_db.with_file_name(format!(
            "{}-wal",
            target_db.file_name().and_then(|n| n.to_str()).unwrap_or("octoswitch.db")
        ));
        let dst_shm = target_db.with_file_name(format!(
            "{}-shm",
            target_db.file_name().and_then(|n| n.to_str()).unwrap_or("octoswitch.db")
        ));
        let _ = copy_if_exists(&src_wal, &dst_wal);
        let _ = copy_if_exists(&src_shm, &dst_shm);
        break;
    }

    Ok(())
}

fn ensure_db_path_ready(db_path: &str) -> Result<(), String> {
    let db_path = Path::new(db_path);
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    migrate_legacy_db_if_needed(db_path)
}

fn build_state(
    config: &AppConfig,
    restart_tx: mpsc::Sender<(GatewayConfig, oneshot::Sender<Result<(), String>>)>,
) -> Result<AppState, String> {
    ensure_db_path_ready(&config.db_path)?;
    let mut conn = Connection::open(&config.db_path).map_err(|e| e.to_string())?;
    database::init_schema(&mut conn)?;
    let http_client = services::http_client::build_shared_client(config)?;

    Ok(AppState {
        db: Arc::new(Mutex::new(conn)),
        metrics: Arc::new(Mutex::new(
            services::metrics_aggregator::MetricsAggregator::default(),
        )),
        breaker: Arc::new(Mutex::new(
            services::circuit_breaker_service::CircuitBreakerService::default(),
        )),
        config: Arc::new(config.clone()),
        restart_tx: Arc::new(Mutex::new(Some(restart_tx))),
        http_client,
        copilot_vendor_cache: Arc::new(services::copilot_vendor_cache::CopilotVendorCache::new()),
    })
}

fn spawn_metrics_warmup(state: AppState) {
    tokio::spawn(async move {
        let result: Result<(), String> = (|| {
            let conn = Connection::open(&state.config.db_path).map_err(|e| e.to_string())?;
            let mut metrics = state
                .metrics
                .lock()
                .map_err(|_| "metrics lock poisoned".to_string())?;
            services::metrics_collector::hydrate_aggregator_from_logs(&conn, &mut metrics)
        })();

        if let Err(e) = result {
            log::error!("[{}] metrics warmup failed: {e}", log_codes::MET_HYDRATE);
        }
    });
}

#[tokio::main]
async fn main() {
    let app_config = AppConfig::default();
    let gw_config = load_gateway_config();

    // Channel: (new_config, response_sender)
    let (restart_tx, mut restart_rx) =
        mpsc::channel::<(GatewayConfig, oneshot::Sender<Result<(), String>>)>(1);

    let state = build_state(&app_config, restart_tx.clone())
        .expect("failed to initialize app state");

    // Apply persisted log level (plugin init sets Trace, we narrow it here)
    log::set_max_level(gw_config.log_level_filter());

    // Warm up metrics in background to avoid blocking first window render.
    spawn_metrics_warmup(state.clone());

    // Gateway supervisor task
    let mgr_state = state.clone();
    tokio::spawn(async move {
        use crate::gateway::router::build_router;
        use tokio::net::TcpListener;

        let mut current_config = gw_config;

        loop {
            let addr = format!("{}:{}", current_config.host, current_config.port);
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("[{}] gateway bind failed on {addr}: {e}", log_codes::GW_BIND);
                    if let Some((cfg, ack)) = restart_rx.recv().await {
                        current_config = cfg;
                        let _ = ack.send(Err(format!("Bind failed: {e}")));
                        continue;
                    }
                    break;
                }
            };

            let app = build_router(mgr_state.clone());
            log::info!("[{}] gateway listening on {addr}", log_codes::GW_START);
            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            let mut shutdown_holder = Some(shutdown_tx);

            let serve = axum::serve(listener, app).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });

            tokio::select! {
                result = serve => {
                    if let Err(e) = result {
                        log::error!("[{}] gateway error: {e}", log_codes::GW_ERROR);
                    }
                }
                Some((new_cfg, ack)) = restart_rx.recv() => {
                    log::info!("[{}] gateway restart requested", log_codes::GW_RESTART);
                    if let Some(tx) = shutdown_holder.take() {
                        let _ = tx.send(());
                    }
                    current_config = new_cfg;
                    let _ = ack.send(Ok(()));
                }
            }
        }
    });

    log::info!("[{}] OctoSwitch starting", log_codes::APP_START);

    // Log directory: %LOCALAPPDATA%/OctoSwitch/logs/ (same parent as config & DB)
    let log_dir = crate::config::app_config::config_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    // Clean old log on startup for a fresh-slate (matches cc-switch behavior)
    let _ = std::fs::remove_file(log_dir.join("OctoSwitch.log"));

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout))
                .target(tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
                    path: log_dir,
                    file_name: Some("OctoSwitch".into()),
                }))
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(2))
                .max_file_size(50_000_000) // 50 MB
                .level(log::LevelFilter::Trace)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                        record.level(),
                        record.target(),
                        message,
                    ))
                })
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin({
            #[cfg(target_os = "macos")]
            {
                tauri_plugin_autostart::Builder::new()
                    .arg(AUTOSTART_MARKER_ARG)
                    .macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent)
                    .build()
            }
            #[cfg(not(target_os = "macos"))]
            {
                tauri_plugin_autostart::Builder::new()
                    .arg(AUTOSTART_MARKER_ARG)
                    .build()
            }
        })
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::provider_commands::list_providers,
            commands::provider_commands::create_provider,
            commands::provider_commands::update_provider,
            commands::provider_commands::delete_provider,
            commands::provider_commands::run_provider_health_check,
            commands::model_commands::list_model_bindings,
            commands::model_fetch_commands::fetch_upstream_models,
            commands::model_commands::create_model_binding,
            commands::model_commands::update_model_binding,
            commands::model_commands::delete_model_binding,
            commands::model_group_commands::list_model_groups,
            commands::model_group_commands::create_model_group,
            commands::model_group_commands::update_model_group,
            commands::model_group_commands::delete_model_group,
            commands::model_group_commands::set_model_group_active_binding,
            commands::model_group_commands::add_model_group_member,
            commands::model_group_commands::remove_model_group_member,
            commands::config_commands::export_config,
            commands::config_commands::import_config,
            commands::config_commands::clear_all_data,
            commands::config_commands::import_cc_switch_providers,
            commands::metrics_commands::get_metrics_kpi,
            commands::metrics_commands::get_metrics_series,
            commands::metrics_commands::list_request_logs,
            commands::gateway_config_commands::get_gateway_config,
            commands::gateway_config_commands::update_gateway_config,
            commands::copilot_commands::start_copilot_auth,
            commands::copilot_commands::complete_copilot_auth,
            commands::copilot_commands::get_copilot_status,
            commands::copilot_commands::refresh_copilot_token,
            commands::copilot_commands::revoke_copilot_auth,
            commands::copilot_commands::open_external_url,
            commands::copilot_commands::list_copilot_accounts,
            commands::copilot_commands::remove_copilot_account,
            commands::copilot_commands::get_copilot_models,
            commands::copilot_commands::get_copilot_usage,
        ])
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }
            let app = window.app_handle();
            let cfg = load_gateway_config();
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if !cfg.close_to_tray {
                        return;
                    }
                    api.prevent_close();
                    if cfg.light_tray_mode {
                        let _ = window.destroy();
                    } else {
                        let _ = window.hide();
                    }
                    show_tray_icon(app);
                }
                _ => {}
            }
        })
        .setup(|app| {
            let tray_menu = tray_support::build_tray_menu(&app.handle())?;
            let tray = tauri::tray::TrayIconBuilder::with_id(TRAY_ICON_ID)
                .tooltip("OctoSwitch")
                .icon(app.default_window_icon().cloned().unwrap())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    tray_support::handle_tray_menu_event(app, event.id.as_ref());
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button,
                        button_state,
                        ..
                    } = event
                    {
                        if button == MouseButton::Left && button_state == MouseButtonState::Up {
                            show_main_window(tray.app_handle());
                        }
                    }
                })
                .build(app)?;
            let _ = tray.set_visible(true);

            // 开机自启动 + 静默：仅当进程由带标记的自启动项拉起且配置开启时，启动后直接进入托盘
            let from_os_autostart = std::env::args().any(|a| a == AUTOSTART_MARKER_ARG);
            if from_os_autostart {
                let cfg = load_gateway_config();
                if cfg.silent_autostart {
                    if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                        let _ = w.hide();
                    }
                    let _ = tray.set_visible(true);
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let cfg = load_gateway_config();
                // 轻量托盘下主窗口被 destroy 后会触发退出请求，需保留进程以便托盘与网关继续运行
                if cfg.close_to_tray && cfg.light_tray_mode {
                    api.prevent_exit();
                }
            }
        });
}
