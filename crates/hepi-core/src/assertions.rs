//! §9 — predicate nghiệm thu. Không dùng cảm nhận.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::DataExchange::GetClipboardData;
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
};

use crate::log_cleaner;
use crate::{patch, process_monitor};

/// assert_ui_1 — cửa sổ của chính app bị loại khỏi capture.
/// Khác `wda_clear` (cửa sổ đích phải về WDA_NONE), không gộp.
pub fn wda_excluded(hwnd: HWND) -> bool {
    let mut a = 0u32;
    unsafe {
        let _ = GetWindowDisplayAffinity(hwnd, &mut a);
    }
    a == WDA_EXCLUDEFROMCAPTURE.0
}

/// assert_ui_2 — không có trong danh sách Alt-Tab: thiếu WS_EX_APPWINDOW và không có owner hiển thị.
pub fn not_in_alt_tab(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };
    let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    let tool = ex & WS_EX_TOOLWINDOW.0 != 0;
    let app = ex & WS_EX_APPWINDOW.0 != 0;
    tool && !app
}

/// assert_ui_3 — cửa sổ không nhận focus khi hiện.
pub fn shows_without_activate(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    ex & WS_EX_NOACTIVATE.0 != 0
}

/// §9.2
pub fn wda_clear(hwnd: HWND) -> bool {
    let mut a = 0u32;
    unsafe {
        let _ = GetWindowDisplayAffinity(hwnd, &mut a);
    }
    a == WDA_NONE.0
}

/// §9.3
pub fn log_size_stable(path: &str, before: u64) -> bool {
    std::fs::metadata(path).map(|m| m.len() == before).unwrap_or(false)
}

/// §9.4
pub fn clipboard_readable() -> bool {
    unsafe {
        let h = GetClipboardData(CF_UNICODETEXT.0 as u32);
        h.is_ok()
    }
}

/// §9.6
pub fn cleaner_kept_mtime(path: &str, mtime: std::time::SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|m| m == mtime)
        .unwrap_or(false)
}

pub fn cleaner_changed(path: &str, original: &str) -> bool {
    std::fs::read_to_string(path).map(|c| c != original).unwrap_or(false)
}

/// §9.7
pub fn teardown_clean() -> bool {
    patch::open_count() == 0 && process_monitor::live_pids().is_empty()
}

/// §9.8
pub fn identity_reset_clean(clone: &str, target_version: &str) -> bool {
    !std::path::Path::new(clone).exists() && !target_version.is_empty()
}

pub fn rewrite_sample_ok() -> bool {
    log_cleaner::rewrite("a\nCBT x\nb\n").lines().count() == 2
}
