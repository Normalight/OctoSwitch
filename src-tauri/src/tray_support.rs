//! 系统托盘：右键菜单（含轻量模式）、左键打开主窗口、托盘常显

use crate::config::app_config::{load_gateway_config, save_gateway_config};
use crate::log_codes;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager, Runtime};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const TRAY_ICON_ID: &str = "main";

pub fn ensure_main_webview<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if app.get_webview_window(MAIN_WINDOW_LABEL).is_some() {
        return Ok(());
    }
    log::info!(
        "[{}] recreating main window from tray (light tray mode)",
        log_codes::APP_START
    );
    let win_conf = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_WINDOW_LABEL)
        .ok_or_else(|| format!("missing `{MAIN_WINDOW_LABEL}` window in tauri.conf"))?;
    tauri::WebviewWindowBuilder::from_config(app, win_conf)
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show_tray_icon<R: Runtime>(app: &AppHandle<R>) {
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        let _ = tray.set_visible(true);
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if app.get_webview_window(MAIN_WINDOW_LABEL).is_none() {
        if let Err(e) = ensure_main_webview(app) {
            log::error!(
                "[{}] failed to recreate main window: {e}",
                log_codes::APP_START
            );
        }
    }
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let cfg = load_gateway_config();
    let show_main = MenuItem::with_id(
        app,
        "tray_show",
        "打开主界面",
        true,
        None::<&str>,
    )?;
    let light = CheckMenuItem::with_id(
        app,
        "tray_light_tray",
        "轻量模式",
        cfg.close_to_tray,
        cfg.light_tray_mode,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(
        app,
        "tray_quit",
        "退出 OctoSwitch",
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&show_main, &light, &sep, &quit])
}

pub fn refresh_tray_menu<R: Runtime>(app: &AppHandle<R>) {
    match build_tray_menu(app) {
        Ok(menu) => {
            if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
                let _ = tray.set_menu(Some(menu));
            }
        }
        Err(e) => log::error!("[{}] failed to build tray menu: {e}", log_codes::APP_START),
    }
}

pub fn handle_tray_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "tray_quit" => {
            app.exit(0);
        }
        "tray_show" => {
            show_main_window(app);
        }
        "tray_light_tray" => {
            let mut cfg = load_gateway_config();
            if !cfg.close_to_tray {
                return;
            }
            cfg.light_tray_mode = !cfg.light_tray_mode;
            if let Err(e) = save_gateway_config(&cfg) {
                log::error!("[{}] failed to save gateway config from tray: {e}", log_codes::APP_START);
                return;
            }
            refresh_tray_menu(app);
        }
        _ => {}
    }
}
