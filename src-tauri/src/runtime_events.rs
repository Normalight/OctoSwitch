use std::sync::OnceLock;

use tauri::{AppHandle, Emitter};

use crate::tray_support::refresh_tray_menu;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn register_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
}

pub fn notify_config_imported() {
    if let Some(app) = APP_HANDLE.get() {
        refresh_tray_menu(app);
        let _ = app.emit("os-config-imported", ());
    }
}
