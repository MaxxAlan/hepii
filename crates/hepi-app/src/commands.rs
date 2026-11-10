//! Toàn bộ IPC. Mỗi lệnh là một bước mỏng gọi engine, không phán đoán.

use hepi_core::settings::Settings;
use hepi_core::{
    app_identity, capture, desktop, entitlement, launcher, log, log_cleaner, network_guard,
    process_monitor, settings, util, wda_bypass, E,
};
use tauri::State;

use crate::AppState;

fn ok<T: serde::Serialize>(v: T) -> Result<T, String> {
    Ok(v)
}

fn map(e: E) -> String {
    e.code().to_string()
}

#[tauri::command]
pub fn check_admin_status() -> bool {
    util::is_admin()
}

#[tauri::command]
pub fn restart_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|_| "E_NO_TARGET")?;
    let w = hepi_core::util::to_wide(&exe.to_string_lossy());
    let verb = hepi_core::util::to_wide("runas");
    unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(
            None,
            windows::core::PCWSTR(verb.as_ptr()),
            windows::core::PCWSTR(w.as_ptr()),
            None,
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn apply_settings(state: State<'_, AppState>, patch: Settings) -> Result<(), String> {
    *state.settings.lock().unwrap() = patch.clone();
    let path = crate::runtime::settings_path();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(path, serde_json::to_vec_pretty(&patch).unwrap_or_default())
        .map_err(|_| "E_ACCESS".to_string())
}

#[tauri::command]
pub fn get_engine_status(state: State<'_, AppState>) -> serde_json::Value {
    let cfg = state.settings.lock().unwrap().clone();
    serde_json::json!({
        "admin": util::is_admin(),
        "patchedPids": hepi_core::patch::patched_pids(),
        "openPatches": hepi_core::patch::open_count(),
        "livePids": process_monitor::live_pids(),
        "routes": network_guard::added_count(),
        "unsigned": true,
        "settings": cfg,
    })
}

#[tauri::command]
pub fn bypass_wda_now(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = state.settings.lock().unwrap().clone();
    wda_bypass::bypass_now(&cfg.selected_programs).map_err(map)
}

#[tauri::command]
pub fn start_wda_monitor(state: State<'_, AppState>) -> Result<(), String> {
    let mut m = state.monitor.lock().unwrap();
    *m = true;
    drop(m);
    let cfg = state.settings.lock().unwrap().clone();
    std::thread::spawn(move || loop {
        let _ = wda_bypass::bypass_now(&cfg.selected_programs);
        std::thread::sleep(std::time::Duration::from_millis(settings::TUNING.verify_cycle_ms));
    });
    Ok(())
}

#[tauri::command]
pub fn stop_wda_monitor(state: State<'_, AppState>) {
    *state.monitor.lock().unwrap() = false;
}

#[tauri::command]
pub fn get_detected_programs() -> Vec<String> {
    let mut v: Vec<String> = process_monitor::snapshot()
        .unwrap_or_default()
        .into_iter()
        .map(|(_, e)| e)
        .collect();
    v.sort();
    v.dedup();
    v
}

#[tauri::command]
pub fn set_selected_programs(state: State<'_, AppState>, names: Vec<String>) {
    state.settings.lock().unwrap().selected_programs = names;
}

#[tauri::command]
pub fn start_kiosk_monitor() -> Result<(), String> {
    log::line("KioskMonitor", "monitor requested");
    Ok(())
}

#[tauri::command]
pub fn stop_kiosk_monitor() {
    hepi_core::patch::rollback_all();
}

#[tauri::command]
pub fn switch_to_seb_desktop() -> Result<String, String> {
    let s = process_monitor::current().ok_or("E_NO_TARGET")?;
    let d = desktop::desktop_of_pid(s.primary_pid).map_err(map)?;
    desktop::switch_to(&d).map_err(map)?;
    ok(d)
}

#[tauri::command]
pub fn switch_to_default_desktop() -> Result<(), String> {
    desktop::switch_to_default().map_err(map)
}

#[tauri::command]
pub fn desktop_switch_eligible() -> bool {
    !desktop::list_desktops().unwrap_or_default().is_empty()
}

macro_rules! monitor_cmd {
    ($start:ident, $stop:ident, $reconcile:path) => {
        #[tauri::command]
        pub fn $start() -> Result<(), String> {
            std::thread::spawn(|| loop {
                $reconcile();
                std::thread::sleep(std::time::Duration::from_millis(settings::TUNING.verify_cycle_ms));
            });
            Ok(())
        }

        #[tauri::command]
        pub fn $stop() {}
    };
}

monitor_cmd!(start_log_filter_monitor, stop_log_filter_monitor, hepi_core::log_filter::reconcile);
monitor_cmd!(
    start_clipboard_bypass_monitor,
    stop_clipboard_bypass_monitor,
    hepi_core::clipboard_bypass::reconcile
);
monitor_cmd!(
    start_lockscreen_bypass_monitor,
    stop_lockscreen_bypass_monitor,
    hepi_core::lockscreen_bypass::reconcile
);

#[tauri::command]
pub fn start_patch_enforcement_monitor() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn stop_patch_enforcement_monitor() {}

#[tauri::command]
pub fn scan_and_clean_seb_logs() -> Result<usize, String> {
    let d = log_cleaner::default_dir().ok_or("E_NO_TARGET")?;
    log_cleaner::clean_dir(&d).map_err(map)
}

#[tauri::command]
pub fn get_seb_log_directory() -> Option<String> {
    log_cleaner::default_dir().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn enable_network_guard() -> Result<(), String> {
    network_guard::enable().map_err(map)
}

#[tauri::command]
pub fn disable_network_guard() {
    network_guard::teardown();
}

#[tauri::command]
pub fn apply_app_identity(source: String, name: String, version: String) -> Result<app_identity::IdentityState, String> {
    app_identity::apply(&source, &name, &version).map_err(map)
}

#[tauri::command]
pub fn reset_app_identity() -> Result<(), String> {
    app_identity::reset().map_err(map)
}

#[tauri::command]
pub fn get_app_identity_state() -> Option<app_identity::IdentityState> {
    app_identity::load()
}

#[tauri::command]
pub fn launcher_get_scopes() -> Vec<String> {
    launcher::scopes()
}

#[tauri::command]
pub fn launcher_rebuild() -> Result<Vec<launcher::Entry>, String> {
    launcher::rebuild().map_err(map)
}

#[tauri::command]
pub fn launch_app(path: String) -> Result<(), String> {
    launcher::launch(&path).map_err(map)
}

#[tauri::command]
pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    let png = capture::grab(x, y, w, h).map_err(map)?;
    Ok(base64(&png))
}

fn base64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[tauri::command]
pub fn trigger_emergency_exit() -> Result<(), String> {
    hepi_core::patch::rollback_all();
    network_guard::teardown();
    desktop::remove_hook();
    hepi_core::state::clear();
    let _ = desktop::switch_to_default();
    log::line("Kiosk", "emergency exit");
    Ok(())
}

#[tauri::command]
pub fn restore_lockdowns() {
    hepi_core::patch::rollback_all();
    network_guard::teardown();
}

#[tauri::command]
pub fn run_self_check() -> serde_json::Value {
    serde_json::json!({
        "wda": hepi_core::assertions::rewrite_sample_ok(),
        "teardownClean": hepi_core::assertions::teardown_clean(),
        "clipboard": hepi_core::assertions::clipboard_readable(),
        "admin": util::is_admin(),
        "api": entitlement::base_url(),
    })
}
