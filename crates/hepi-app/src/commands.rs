//! Toàn bộ IPC. Mỗi lệnh là một bước mỏng gọi engine, không phán đoán.

use hepi_core::settings::Settings;
use hepi_core::{
    app_identity, capture, desktop, entitlement, launcher, log, log_cleaner, network_guard,
    process_monitor, profiles, settings, util, wda_bypass, E,
};
use tauri::{Manager, State};

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
    match state.settings.lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

#[tauri::command]
pub fn apply_settings(state: State<'_, AppState>, patch: Settings) -> Result<(), String> {
    match state.settings.lock() {
        Ok(mut guard) => *guard = patch.clone(),
        Err(poisoned) => *poisoned.into_inner() = patch.clone(),
    }
    let path = crate::runtime::settings_path();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(path, serde_json::to_vec_pretty(&patch).unwrap_or_default())
        .map_err(|_| "E_ACCESS".to_string())
}

#[tauri::command]
pub fn get_engine_status(state: State<'_, AppState>) -> serde_json::Value {
    let cfg = match state.settings.lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
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
    let cfg = match state.settings.lock() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    wda_bypass::bypass_now(&cfg.selected_programs).map_err(map)
}

#[tauri::command]
pub fn start_wda_monitor(state: State<'_, AppState>) -> Result<(), String> {
    let mut m = state.monitor.lock().unwrap();
    *m = true;
    drop(m);
    // Clone Arc to share with monitor thread; monitor will re-read settings on each iteration
    let settings = std::sync::Arc::clone(&state.settings);
    std::thread::spawn(move || loop {
        match settings.lock() {
            Ok(cfg) => {
                let _ = wda_bypass::bypass_now(&cfg.selected_programs);
            }
            Err(poisoned) => {
                let cfg = poisoned.into_inner();
                let _ = wda_bypass::bypass_now(&cfg.selected_programs);
            }
        }
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
    match state.settings.lock() {
        Ok(mut guard) => guard.selected_programs = names,
        Err(poisoned) => poisoned.into_inner().selected_programs = names,
    }
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
                let _ = $reconcile();
                std::thread::sleep(std::time::Duration::from_millis(settings::TUNING.verify_cycle_ms));
            });
            Ok(())
        }

        #[tauri::command]
        pub fn $stop() {}
    };
}

monitor_cmd!(start_log_filter_monitor, stop_log_filter_monitor, hepi_core::log_filter::apply_now);
monitor_cmd!(
    start_clipboard_bypass_monitor,
    stop_clipboard_bypass_monitor,
    hepi_core::clipboard_bypass::apply_now
);
monitor_cmd!(
    start_lockscreen_bypass_monitor,
    stop_lockscreen_bypass_monitor,
    hepi_core::lockscreen_bypass::apply_now
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
pub fn apply_app_identity(
    source: String,
    name: String,
    version: String,
) -> Result<app_identity::IdentityState, String> {
    // Giữ nguyên hình dạng IPC: `name` chọn profile, `source` là tệp visual,
    // `version` không còn dùng (đọc từ PE của nguồn) nhưng vẫn nhận để không
    // phá frontend cũ.
    let _ = version;
    let profile = profiles::by_name(&name).unwrap_or_else(|| {
        profiles::by_name(profiles::DEFAULT_PROFILE).expect("default profile")
    });
    let visual = if source.trim().is_empty() { None } else { Some(source.as_str()) };
    app_identity::apply(profile, visual).map_err(map)
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

// ---- Quiz: AI + ChatGPT web ---------------------------------------------------

/// Hỏi Gemini. Rỗng khi chưa có `GEMINI_API_KEY` trong `.env`.
#[tauri::command]
pub async fn quiz_ask(prompt: String, system: Option<String>) -> Result<String, String> {
    let client = hepi_core::gemini::Client::from_env().map_err(map)?;
    let sys = system.as_deref();
    client.chat(sys, &prompt).map_err(map)
}

/// Trạng thái nguồn AI: khoá Gemini có trong `.env` không (không gọi mạng).
#[tauri::command]
pub fn quiz_ai_status() -> serde_json::Value {
    serde_json::json!({
        "geminiKey": hepi_core::gemini::Client::key_present(),
        "geminiModel": hepi_core::gemini::DEFAULT_MODEL,
    })
}

/// Mở ChatGPT web trong cửa sổ riêng (webview ngoài).
#[tauri::command]
pub fn quiz_open_chatgpt(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::config::WebviewUrl;
    use tauri::{Manager, WebviewWindowBuilder};
    let url = "https://chatgpt.com/"
        .parse::<url::Url>()
        .map_err(|_| "E_NO_TARGET".to_string())?;
    let _ = app.get_webview_window("chatgpt");
    WebviewWindowBuilder::new(&app, "chatgpt", WebviewUrl::External(url))
        .title("ChatGPT")
        .inner_size(1100.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Đóng cửa sổ ChatGPT web nếu đang mở.
#[tauri::command]
pub fn quiz_close_chatgpt(app: tauri::AppHandle) -> bool {
    match app.get_webview_window("chatgpt") {
        Some(w) => w.close().is_ok(),
        None => false,
    }
}

/// Trạng thái đăng nhập ChatGPT web: đọc cookie phiên của webview.
#[tauri::command]
pub fn quiz_chatgpt_login(app: tauri::AppHandle) -> serde_json::Value {
    // Cookie của ChatGPT nằm trong profile WebView2 của app.
    // Đọc qua webview chính (cùng profile ⇒ cùng cookie store).
    let Some(win) = app.get_webview_window("main") else {
        return serde_json::json!({ "loggedIn": false, "reason": "no window" });
    };
    let Ok(url) = "https://chatgpt.com/".parse::<url::Url>() else {
        return serde_json::json!({ "loggedIn": false });
    };
    match win.cookies_for_url(url) {
        Ok(cookies) => {
            // `__Secure-next-auth.session-token` là cookie phiên của ChatGPT web.
            let logged = cookies.iter().any(|c| {
                let n = c.name();
                n.contains("session-token") || n == "__Secure-next-auth.session-token"
            });
            serde_json::json!({ "loggedIn": logged, "cookies": cookies.len() })
        }
        Err(e) => serde_json::json!({ "loggedIn": false, "reason": e.to_string() }),
    }
}

// ---- SEB 3.x -----------------------------------------------------------------

/// Trạng thái engine chống SEB: tiến trình SEB nào đang chạy, patch nào đã áp.
#[tauri::command]
pub fn seb_status() -> serde_json::Value {
    let mut procs = serde_json::Map::new();
    for (role, pid, name) in hepi_core::seb3::running() {
        procs.insert(
            format!("{role:?}").to_lowercase(),
            serde_json::json!({ "pid": pid, "name": name }),
        );
    }
    serde_json::json!({
        "processes": procs,
        "patchedPids": hepi_core::patch::patched_pids(),
        "openPatches": hepi_core::patch::open_count(),
        "admin": util::is_admin(),
        "windowGuard": crate::window_guard::enabled(),
        "logDir": hepi_core::log_cleaner::default_dir()
            .map(|p| p.to_string_lossy().to_string()),
    })
}

/// Áp toàn bộ engine chống SEB ngay lập tức (dùng khi SEB vừa bật).
#[tauri::command]
pub fn seb_apply_now() -> serde_json::Value {
    let log_filter = hepi_core::log_filter::apply_now().unwrap_or(false);
    let clipboard = hepi_core::clipboard_bypass::apply_now().unwrap_or(false);
    let lockscreen = hepi_core::lockscreen_bypass::apply_now().unwrap_or(false);
    let enforcement = hepi_core::patch_enforcement::apply_now().unwrap_or(false);
    serde_json::json!({
        "logFilter": log_filter,
        "clipboard": clipboard,
        "lockscreen": lockscreen,
        "patchEnforcement": enforcement,
    })
}

/// Bật/tắt lớp tự vệ cửa sổ (chặn SEB đóng app).
#[tauri::command]
pub fn set_window_guard(enabled: bool) -> bool {
    crate::window_guard::set_enabled(enabled);
    crate::window_guard::enabled()
}
