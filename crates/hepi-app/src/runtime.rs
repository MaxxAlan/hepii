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

    // 2. single instance
    if pipe::already_running() {
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
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            settings: Mutex::new(cfg.clone()),
            receipt: Mutex::new(None),
            monitor: Mutex::new(false),
        })
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
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let cfg2 = cfg.clone();
            // 6. hotkeys
            desktop::install_hook().ok();
            // 7. monitor
            std::thread::spawn(move || monitor_loop(handle, cfg2, session));
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

fn monitor_loop(app: tauri::AppHandle, cfg: settings::Settings, _session: Option<hepi_core::diagnostics::SessionMarker>) {
    let mut ended = false;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(
            process_monitor::cycle_ms() as u64,
        ));
        if !cfg.seb_kiosk_bypass {
            continue;
        }
        let session_end = process_monitor::tick(&cfg.target_processes);
        if session_end && !ended {
            ended = true;
            on_session_end(&app, &cfg);
        } else if !session_end {
            ended = false;
            if let Some(s) = process_monitor::current() {
                if cfg.seb_desktop_switch {
                    if let Ok(d) = desktop::desktop_of_pid(s.primary_pid) {
                        let _ = desktop::switch_to(&d);
                    }
                }
                if cfg.seb_log_filter {
                    hepi_core::log_filter::reconcile();
                }
                if cfg.seb_clipboard_bypass {
                    hepi_core::clipboard_bypass::reconcile();
                }
                if cfg.seb_lockscreen_bypass {
                    hepi_core::lockscreen_bypass::reconcile();
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
