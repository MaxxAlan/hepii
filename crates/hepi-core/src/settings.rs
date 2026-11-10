use serde::{Deserialize, Serialize};

/// §10 — mọi ngưỡng, cố định. AI thực thi không chọn.
pub struct Tuning {
    pub process_monitor_cycle_ms: u32,
    pub verify_cycle_ms: u64,
    pub remote_thread_timeout_ms: u32,
    pub patch_retry_max: u32,
    pub patch_retry_delay_ms: u64,
    pub verify_delay_ms: u64,
    pub session_end_empty_cycles: u8,
    pub version_check_ms: u64,
    pub diag_backoff_s: [u64; 3],
}

pub const TUNING: Tuning = Tuning {
    process_monitor_cycle_ms: 1000,
    verify_cycle_ms: 2000,
    remote_thread_timeout_ms: 3000,
    patch_retry_max: 5,
    patch_retry_delay_ms: 500,
    verify_delay_ms: 300,
    session_end_empty_cycles: 2,
    version_check_ms: 600_000,
    diag_backoff_s: [30, 120, 600],
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub content_protection: bool,
    pub wda_bypass: bool,
    pub seb_kiosk_bypass: bool,
    pub seb_desktop_switch: bool,
    pub seb_log_filter: bool,
    pub seb_patch_enforcement: bool,
    pub seb_clipboard_bypass: bool,
    pub seb_lockscreen_bypass: bool,
    pub seb_log_cleaner: bool,
    pub launch_at_startup: bool,
    pub keep_running_in_tray: bool,
    pub selected_programs: Vec<String>,
    pub target_processes: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            content_protection: true,
            wda_bypass: true,
            seb_kiosk_bypass: true,
            seb_desktop_switch: true,
            seb_log_filter: true,
            seb_patch_enforcement: true,
            seb_clipboard_bypass: true,
            seb_lockscreen_bypass: true,
            seb_log_cleaner: true,
            launch_at_startup: false,
            keep_running_in_tray: true,
            selected_programs: Vec::new(),
            target_processes: vec![
                "seb.exe".into(),
                "xul.exe".into(),
                "seb_xulrunner.exe".into(),
            ],
        }
    }
}

impl Settings {
    /// Đọc khoá sai kiểu ⇒ mặc định + log. Không bao giờ từ chối vì settings.
    pub fn load_or_default(raw: Option<&str>) -> Self {
        let mut s: Settings = raw
            .and_then(|r| serde_json::from_str(r).ok())
            .unwrap_or_default();
        if s.target_processes.is_empty() {
            s.target_processes = Settings::default().target_processes;
        }
        s
    }
}
