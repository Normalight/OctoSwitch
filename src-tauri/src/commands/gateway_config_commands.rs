use serde::Serialize;
use tauri::State;
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::oneshot;

use crate::config::app_config::{load_gateway_config, save_gateway_config, GatewayConfig};
use crate::runtime_events;
use crate::state::AppState;
use crate::tray_support::refresh_tray_menu;

#[tauri::command]
pub fn get_gateway_config() -> Result<GatewayConfig, String> {
    Ok(load_gateway_config())
}

#[tauri::command]
pub async fn update_gateway_config(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    config: GatewayConfig,
) -> Result<(), String> {
    let prev = load_gateway_config();
    let need_gateway_restart = prev.host != config.host || prev.port != config.port;

    save_gateway_config(&config)?;
    refresh_tray_menu(&app_handle);
    runtime_events::notify_config_imported();

    // Apply log level immediately (no restart needed)
    log::set_max_level(config.log_level_filter());
    log::info!("[APP-001] log level changed to {}", config.log_level);

    // Sync autostart with OS (skip in dev builds — the debug exe shows a CMD window
    // and requires a Vite dev server that won't be running on boot)
    if cfg!(debug_assertions) {
        if config.auto_start {
            log::warn!(
                "[APP-001] autostart registration skipped in dev mode — use a release build for autostart"
            );
        }
    } else {
        let autostart_mgr = app_handle.autolaunch();
        if config.auto_start {
            let _ = autostart_mgr.enable();
        } else {
            let _ = autostart_mgr.disable();
        }
    }

    if !need_gateway_restart {
        return Ok(());
    }

    // Clone the sender out of the mutex before awaiting
    let sender = {
        let tx = state
            .restart_tx
            .lock()
            .map_err(|_| "restart channel lock poisoned")?;
        tx.clone()
    };

    if let Some(sender) = sender {
        let (ack_tx, ack_rx) = oneshot::channel();
        sender
            .send((config, ack_tx))
            .await
            .map_err(|_| "Failed to send restart signal")?;
        ack_rx
            .await
            .map_err(|_| "Restart ack channel closed")?
            .map_err(|e| format!("Gateway restart failed: {e}"))?;
    }

    Ok(())
}

#[derive(Serialize, Clone)]
pub struct GatewayHealthStatus {
    pub is_running: bool,
    pub host: String,
    pub port: u16,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn restart_gateway(state: State<'_, AppState>) -> Result<(), String> {
    let config = load_gateway_config();

    let sender = {
        let tx = state
            .restart_tx
            .lock()
            .map_err(|_| "restart channel lock poisoned")?;
        tx.clone()
    };

    if let Some(sender) = sender {
        let (ack_tx, ack_rx) = oneshot::channel();
        sender
            .send((config, ack_tx))
            .await
            .map_err(|_| "Failed to send restart signal")?;
        ack_rx
            .await
            .map_err(|_| "Restart ack channel closed")?
            .map_err(|e| format!("Gateway restart failed: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn check_gateway_health() -> Result<GatewayHealthStatus, String> {
    let cfg = load_gateway_config();
    let url = format!("http://{}:{}/healthz", cfg.host, cfg.port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    match client.get(&url).send().await {
        Ok(resp) => {
            match resp.json::<serde_json::Value>().await {
                Ok(body) => {
                    let is_running = body.get("ok").and_then(|v| v.as_bool()) == Some(true);
                    let err = if is_running {
                        None
                    } else {
                        Some(format!("Unexpected response: {body}"))
                    };
                    Ok(GatewayHealthStatus {
                        is_running,
                        host: cfg.host,
                        port: cfg.port,
                        error: err,
                    })
                }
                Err(e) => Ok(GatewayHealthStatus {
                    is_running: false,
                    host: cfg.host,
                    port: cfg.port,
                    error: Some(format!("Failed to parse health response: {e}")),
                }),
            }
        }
        Err(e) => Ok(GatewayHealthStatus {
            is_running: false,
            host: cfg.host,
            port: cfg.port,
            error: Some(e.to_string()),
        }),
    }
}
