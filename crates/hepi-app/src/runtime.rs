//! §5.1 — thứ tự khởi động cố định, không đảo.

use std::sync::{Arc, Mutex};

use hepi_core::{desktop, log, process_monitor, settings, state, util, E, Res};
use tauri::Emitter;

use crate::pipe;
use crate::commands;
use crate::AppState;

fn cli_flag(name: &str) -> bool {
    std::env::args().any(|a| a == name)
}

pub fn boot() -> Res<()> {
    // 1. CLI
    if cli_flag("--identity-uninstall") {
        let _ = hepi_core::app_identity::reset();
        return Ok(());
    }
    log::line("ArchiveGuard", "pre-tauri archive check");

    // 1b. Companion: gắn luồng chính vào desktop SEB TRƯỚC khi Tauri dựng cửa sổ.
    //     (SetThreadDesktop chỉ hợp lệ khi luồng chưa tạo cửa sổ nào.)
    let companion = crate::companion::is_companion();
    if companion {
        match crate::companion::desktop_arg() {
            Some(d) => {
                if let Err(e) = desktop::attach_thread_to(&d) {
                    log::err("Companion", e, "cannot attach to desktop");
                }
            }
            None => {
                log::line("Companion", "no --desktop given, staying on current desktop");
            }
        }
    }

    // 2. single instance (companion được miễn: nó là tiến trình phụ của host)
    if !companion && pipe::already_running() {
        log::line("SingleInstance", "duplicate launch routed to existing instance");
        return Ok(());
    }

    // 3. settings
    let cfg = settings::Settings::load_or_default(
        std::fs::read_to_string(settings_path()).ok().as_deref(),
    );

    // 4. diagnostics
    let session = hepi_core::diagnostics::start_session(env!("CARGO_PKG_VERSION")).ok();

    // 5..8 qua Tauri
    let app_state = AppState {
        settings: Arc::new(Mutex::new(cfg.clone())),  // Wrap in Arc for shared mutable access
        receipt: Mutex::new(None),
        monitor: Mutex::new(false),
    };

    let settings_for_monitor = Arc::clone(&app_state.settings);

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::check_admin_status,
            commands::restart_as_admin,
            commands::get_settings,
            commands::apply_settings,
            commands::get_engine_status,
            commands::bypass_wda_now,
            commands::start_wda_monitor,
            commands::stop_wda_monitor,
            commands::get_detected_programs,
            commands::set_selected_programs,
            commands::start_kiosk_monitor,
            commands::stop_kiosk_monitor,
            commands::switch_to_seb_desktop,
            commands::switch_to_default_desktop,
            commands::desktop_switch_eligible,
            commands::start_log_filter_monitor,
            commands::stop_log_filter_monitor,
            commands::start_clipboard_bypass_monitor,
            commands::stop_clipboard_bypass_monitor,
            commands::start_lockscreen_bypass_monitor,
            commands::stop_lockscreen_bypass_monitor,
            commands::start_patch_enforcement_monitor,
            commands::stop_patch_enforcement_monitor,
            commands::scan_and_clean_seb_logs,
            commands::get_seb_log_directory,
            commands::enable_network_guard,
            commands::disable_network_guard,
            commands::apply_app_identity,
            commands::reset_app_identity,
            commands::get_app_identity_state,
            commands::launcher_get_scopes,
            commands::launcher_rebuild,
            commands::launch_app,
            commands::capture_region,
            commands::trigger_emergency_exit,
            commands::restore_lockdowns,
            commands::run_self_check,
            commands::seb_status,
            commands::seb_apply_now,
            commands::set_window_guard,
            commands::quiz_ask,
            commands::quiz_ai_status,
            commands::quiz_open_chatgpt,
            commands::quiz_close_chatgpt,
            commands::quiz_chatgpt_login,
            commands::check_update,
            commands::install_update,
            commands::capture_screen,
            commands::capture_rect,
            commands::capture_preview,
            commands::quiz_ask_image,
            commands::screen_size,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            // 6. hotkeys
            desktop::install_hook().ok();
            // 6b. tự vệ cửa sổ: SEB 3.x đóng cửa sổ của tiến trình lạ
            let guard_handle = app.handle().clone();
            crate::window_guard::install(&guard_handle);
            // 7. monitor — pass Arc for live config updates
            std::thread::spawn(move || monitor_loop(handle, settings_for_monitor, session));
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|_| E::Unsupported)?;
    Ok(())
}

pub fn settings_path() -> std::path::PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(base)
        .join("com.easyapp.hepi")
        .join("settings.json")
}

fn monitor_loop(app: tauri::AppHandle, cfg: Arc<Mutex<settings::Settings>>, _session: Option<hepi_core::diagnostics::SessionMarker>) {
    let mut ended = false;
    // Desktop SEB đang dùng + pid companion đã spawn (0 = chưa spawn).
    let mut companion_desktop = String::new();
    let mut companion_pid = 0u32;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(
            process_monitor::cycle_ms() as u64,
        ));
        // Re-read settings on every iteration to respect real-time UI changes
        let cfg_current = match cfg.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => {
                log::err("Monitor", E::State, "settings lock poisoned, using last known");
                poisoned.into_inner().clone()
            }
        };

        if !cfg_current.seb_kiosk_bypass {
            continue;
        }
        let session_end = process_monitor::tick(&cfg_current.target_processes);
        if session_end && !ended {
            ended = true;
            companion_pid = 0;
            companion_desktop.clear();
            on_session_end(&app, &cfg_current);
        } else if !session_end {
            ended = false;
            if let Some(s) = process_monitor::current() {
                // --- Companion UI trên desktop SEB ---
                // SEB 3 tạo desktop riêng rồi switch sang ⇒ cửa sổ trên
                // 'Default' không hiển thị. Spawn UI phụ trên desktop SEB.
                // Companion KHÔNG spawn tiếp (tránh vòng lặp).
                if !crate::companion::is_companion() {
                    let seb_desktop = desktop::seb_desktop_of(&s.pids);
                    if let Some(d) = seb_desktop.as_deref() {
                        if d != companion_desktop {
                            if companion_pid != 0 {
                                let _ = std::process::Command::new("taskkill")
                                    .args(["/F", "/PID", &companion_pid.to_string()])
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                            }
                            companion_desktop = d.to_string();
                            companion_pid = crate::companion::spawn_companion(d).unwrap_or(0);
                        }
                    }
                }

                if cfg_current.seb_desktop_switch {
                    if let Ok(d) = desktop::desktop_of_pid(s.primary_pid) {
                        let _ = desktop::switch_to(&d);
                    }
                }
                // Companion chỉ mang UI: KHÔNG patch, KHÔNG đổi desktop
                // (host đã làm) — tránh hai tiến trình tranh engine.
                if !crate::companion::is_companion() {
                    if cfg_current.seb_log_filter {
                        let _ = hepi_core::log_filter::apply_now();
                    }
                    if cfg_current.seb_clipboard_bypass {
                        let _ = hepi_core::clipboard_bypass::apply_now();
                    }
                    if cfg_current.seb_lockscreen_bypass {
                        let _ = hepi_core::lockscreen_bypass::apply_now();
                    }
                    if cfg_current.seb_patch_enforcement {
                        let _ = hepi_core::patch_enforcement::apply_now();
                    }
                }
            }
        }
    }
}

fn on_session_end(app: &tauri::AppHandle, cfg: &settings::Settings) {
    log::line("Kiosk", "SEB session ended; resetting verifier state");
    hepi_core::patch::rollback_all();
    hepi_core::network_guard::teardown();
    desktop::remove_hook();
    let _ = desktop::switch_to_default();
    state::clear();
    if cfg.seb_log_cleaner {
        if let Some(d) = hepi_core::log_cleaner::default_dir() {
            let _ = hepi_core::log_cleaner::clean_dir(&d);
        }
    }
    let _ = app.emit("session-ended", ());
    let _ = util::is_admin();
    let _: Arc<()> = Arc::new(());
}
