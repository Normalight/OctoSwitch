//! 系统托盘：右键菜单（含分组切换）、左键打开主窗口、托盘常显

use crate::config::app_config::{load_gateway_config, save_gateway_config};
use crate::log_codes;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Runtime};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const TRAY_ICON_ID: &str = "main";
pub const TRAY_QUIT_REQUESTED_ENV: &str = "OCTOSWITCH_TRAY_QUIT_REQUESTED";

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

/// Hide macOS Dock icon when app enters tray-only state.
#[allow(unused_variables)]
pub fn hide_dock_icon<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory).ok();
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
    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Regular).ok();
    }
}

/// Menu item ID prefix for group enable.
/// Full ID: `tray_enable_{group_id}`
const ENABLE_PREFIX: &str = "tray_enable_";

/// Menu item ID prefix for group disable.
/// Full ID: `tray_disable_{group_id}`
const DISABLE_PREFIX: &str = "tray_disable_";

/// Menu item ID prefix for group member switching.
/// Full ID: `tray_switch_{group_id}_{binding_id}`
const SWITCH_PREFIX: &str = "tray_switch_";

pub fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let cfg = load_gateway_config();

    // "打开主界面"
    let show_main = MenuItem::with_id(app, "tray_show", "打开主界面", true, None::<&str>)?;

    // "轻量模式" checkbox（与打开主界面归为一组）
    let light = CheckMenuItem::with_id(
        app,
        "tray_light_tray",
        "轻量模式",
        cfg.close_to_tray,
        cfg.light_tray_mode,
        None::<&str>,
    )?;

    // Separator
    let sep1 = PredefinedMenuItem::separator(app)?;

    // Collect group submenus
    let group_items = build_group_submenus(app).unwrap_or_default();

    // Separator
    let sep2 = PredefinedMenuItem::separator(app)?;

    // "退出 OctoSwitch"
    let quit = MenuItem::with_id(app, "tray_quit", "退出 OctoSwitch", true, None::<&str>)?;

    // Build items list with references
    let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> = vec![&show_main, &light, &sep1];
    for item in &group_items {
        items.push(item);
    }
    items.push(&sep2);
    items.push(&quit);

    Menu::with_items(app, &items)
}

/// Build submenu for each group, with a top toggle item, separator, and CheckMenuItem per member.
fn build_group_submenus<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Vec<Submenu<R>>> {
    let state = match app.try_state::<crate::state::AppState>() {
        Some(s) => s,
        None => return Ok(vec![]),
    };
    let Ok(conn) = state.db.lock() else {
        return Ok(vec![]);
    };

    let Ok(groups) = crate::database::model_group_dao::list(&conn) else {
        return Ok(vec![]);
    };

    let mut result: Vec<Submenu<R>> = Vec::new();

    for group in &groups {
        let Ok(member_binding_ids) =
            crate::database::model_group_member_dao::list_binding_ids_for_group(&conn, &group.id)
        else {
            continue;
        };
        if member_binding_ids.is_empty() {
            continue;
        }

        // Dual toggle: 启用 / 禁用 (mutually exclusive checkmarks)
        let enable_id = format!("{}{}", ENABLE_PREFIX, group.id);
        let enable = CheckMenuItem::with_id(app, &enable_id, "启用", true, group.is_enabled, None::<&str>)?;
        let disable_id = format!("{}{}", DISABLE_PREFIX, group.id);
        let disable = CheckMenuItem::with_id(app, &disable_id, "禁用", true, !group.is_enabled, None::<&str>)?;

        // Build check items for each member
        let mut owned_checks: Vec<CheckMenuItem<R>> = Vec::new();

        for binding_id in &member_binding_ids {
            let binding_name = crate::database::model_binding_dao::get_by_id(&conn, binding_id)
                .ok()
                .flatten()
                .map(|b| b.model_name)
                .unwrap_or_else(|| binding_id.clone());

            let is_active = group.active_binding_id.as_deref() == Some(binding_id);
            let item_id = format!("{}{}_{}", SWITCH_PREFIX, group.id, binding_id);
            let check = CheckMenuItem::with_id(
                app,
                &item_id,
                &binding_name,
                group.is_enabled,
                is_active,
                None::<&str>,
            )?;
            owned_checks.push(check);
        }

        // Build reference slice: enable + disable + separator + checks
        let sep = PredefinedMenuItem::separator(app)?;
        let mut item_refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = vec![&enable, &disable, &sep];
        for c in &owned_checks {
            item_refs.push(c as &dyn tauri::menu::IsMenuItem<R>);
        }

        // Title: ✓ enabled / ✕ disabled + alias · active model
        let active_model = group
            .active_binding_id
            .as_ref()
            .and_then(|binding_id| crate::database::model_binding_dao::get_by_id(&conn, binding_id).ok().flatten())
            .map(|binding| binding.model_name)
            .unwrap_or_else(|| "-".to_string());
        let status_icon = if group.is_enabled { "●" } else { "○" };
        let submenu_title = format!("{} {} · {}", status_icon, group.alias, active_model);
        let submenu = Submenu::with_items(app, &submenu_title, true, &item_refs)?;
        result.push(submenu);
    }

    Ok(result)
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
            let _ = std::env::set_var(TRAY_QUIT_REQUESTED_ENV, "1");
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
        id if id.starts_with(ENABLE_PREFIX) => {
            let group_id = &id[ENABLE_PREFIX.len()..];
            if let Err(e) = set_group_enabled(app, group_id, true) {
                log::error!("[{}] tray enable failed: {e}", log_codes::APP_START);
            }
        }
        id if id.starts_with(DISABLE_PREFIX) => {
            let group_id = &id[DISABLE_PREFIX.len()..];
            if let Err(e) = set_group_enabled(app, group_id, false) {
                log::error!("[{}] tray disable failed: {e}", log_codes::APP_START);
            }
        }
        id if id.starts_with(SWITCH_PREFIX) => {
            // Parse: tray_switch_{group_id}_{binding_id}
            let rest = &id[SWITCH_PREFIX.len()..];
            if let Some(underscore_pos) = rest.rfind('_') {
                let group_id = &rest[..underscore_pos];
                let binding_id = &rest[underscore_pos + 1..];
                if let Err(e) = switch_active_binding(app, group_id, binding_id) {
                    log::error!("[{}] tray switch failed: {e}", log_codes::APP_START);
                }
            }
        }
        _ => {}
    }
}

fn set_group_enabled<R: Runtime>(app: &AppHandle<R>, group_id: &str, enabled: bool) -> Result<(), String> {
    use tauri::Emitter;

    let state = app.state::<crate::state::AppState>();
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    crate::database::model_group_dao::get_by_id(&conn, group_id)?
        .ok_or_else(|| "未找到模型分组".to_string())?;
    crate::database::model_group_dao::update_partial(
        &conn,
        group_id,
        serde_json::json!({ "is_enabled": enabled }),
    )?;
    drop(conn);

    refresh_tray_menu(app);
    let _ = app.emit("os-config-imported", ());

    log::info!(
        "[{}] tray set group enabled: group={} enabled={}",
        log_codes::APP_START,
        group_id,
        enabled
    );

    Ok(())
}

fn switch_active_binding<R: Runtime>(
    app: &AppHandle<R>,
    group_id: &str,
    binding_id: &str,
) -> Result<(), String> {
    use tauri::Emitter;

    let state = app.state::<crate::state::AppState>();
    let conn = state.db.lock().map_err(|_| "db lock poisoned")?;
    crate::database::model_group_dao::set_active_binding(&conn, group_id, Some(binding_id))?;
    drop(conn);

    // Refresh tray menu to reflect the new active selection
    refresh_tray_menu(app);

    // Notify frontend
    let _ = app.emit("os-config-imported", ());

    log::info!(
        "[{}] tray switch: group={} -> binding={}",
        log_codes::APP_START,
        group_id,
        binding_id
    );

    Ok(())
}
