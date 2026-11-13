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
    /// Venice HTTP — connect và toàn bộ trao đổi.
    pub http_connect_timeout_ms: u32,
    pub http_timeout_ms: u32,
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
    http_connect_timeout_ms: 10_000,
    http_timeout_ms: 30_000,
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
            // SEB 3.x (CefSharp): không còn seb.exe/xul.exe.
            target_processes: vec![
                crate::seb3::CLIENT_EXE.into(),
                crate::seb3::RUNTIME_EXE.into(),
                crate::seb3::BROWSER_SUBPROCESS_EXE.into(),
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
        } else if s.target_processes.iter().any(|n| is_legacy_seb_name(n)) {
            // Cấu hình cũ (SEB 2.x) còn tên `seb.exe`/`xul.exe` ⇒ vô dụng với
            // SEB 3.x. Giữ danh sách riêng của người dùng không thể giữ được
            // ⇒ thay bằng danh sách SEB 3.x.
            s.target_processes = Settings::default().target_processes;
        }
        s
    }
}

/// Tên tiến trình của SEB **2.x** — không tồn tại trong SEB 3.x.
fn is_legacy_seb_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "seb.exe" | "xul.exe" | "seb_xulrunner.exe" | "seb_x64.exe" | "seb.exe.exe"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_target_processes() {
        let raw = r#"{"targetProcesses":["seb.exe","xul.exe"],"keepRunningInTray":false}"#;
        let s = Settings::load_or_default(Some(raw));
        assert_eq!(s.target_processes, Settings::default().target_processes);
        assert!(!s.keep_running_in_tray);
    }

    #[test]
    fn keeps_seb3_target_processes() {
        let raw = r#"{"targetProcesses":["SafeExamBrowser.Client.exe"]}"#;
        let s = Settings::load_or_default(Some(raw));
        assert_eq!(s.target_processes, vec!["SafeExamBrowser.Client.exe".to_string()]);
    }

    #[test]
    fn empty_falls_back_to_default() {
        let s = Settings::load_or_default(Some(r#"{"targetProcesses":[]}"#));
        assert_eq!(s.target_processes, Settings::default().target_processes);
    }
}
