//! Giữ cửa sổ Hepi nhìn thấy được khi SEB chạy.
//!
//! SEB 3.x có `ApplicationMonitor`: dò cửa sổ của tiến trình không nằm trong
//! whitelist rồi gọi `ShowWindow(SW_HIDE)` / gửi `WM_CLOSE`.
//! Log SEB thật: `Window '' (197992, Overlay) belongs to not whitelisted process
//! 'hepi.exe'!` → `Failed to hide window` → `Sent close message` → `Successfully
//! closed window`. Hậu quả: app còn sống nhưng mất cửa sổ ⇒ trông như
//! "đứng".
//!
//! Legacy v0.20.9 chống bằng 2 lớp:
//! - `ShowWindow` filter trong tiến trình SEB (chặn `SW_HIDE`).
//! - `topmost_keeper`: hằng kỳ đặt lại `HWND_TOPMOST`.
//!
//! Ở đây ta làm lớp tự vệ phía app (không cần patch code SEB):
//! - Chặn `WM_CLOSE` từ bên ngoài (SEB gửi qua `PostMessage`).
//! - Hằng kỳ nhấc lại `HWND_TOPMOST` + `ShowWindow(SW_SHOWNOACTIVATE)` nếu bị ẩn.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, Runtime, WindowEvent};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE,
    HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE, WS_EX_TOPMOST,
};

static KEEP_TOPMOST: AtomicBool = AtomicBool::new(true);

/// Bật/tắt lớp tự vệ cửa sổ.
pub fn set_enabled(on: bool) {
    KEEP_TOPMOST.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    KEEP_TOPMOST.load(Ordering::Relaxed)
}

/// Đặt `WS_EX_TOPMOST` cho một cửa sổ.
fn make_topmost(hwnd: HWND) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | (WS_EX_TOPMOST.0 as isize));
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Nếu cửa sổ bị ẩn → hiện lại (không chiếm focus).
fn ensure_visible(hwnd: HWND) {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
}

/// HWND thô của cửa sổ "main" (tauri dùng `windows` 0.62, ta dùng 0.58 ⇒ lấy con trỏ).
fn main_hwnd<R: Runtime>(app: &AppHandle<R>) -> Option<HWND> {
    let win = app.get_webview_window("main")?;
    let h = win.hwnd().ok()?;
    Some(HWND(h.0))
}

/// Đăng ký chặn `WM_CLOSE` từ bên ngoài và khởi động thread giữ topmost.
///
/// `WM_CLOSE` do SEB gửi tới cửa sổ của ta sẽ bị chặn ở đây: app vẫn sống,
/// cửa sổ vẫn còn.
pub fn install<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();

    // Chặn WM_CLOSE từ bên ngoài (SEB gửi qua PostMessage).
    if let Some(win) = app.get_webview_window("main") {
        win.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if enabled() {
                    api.prevent_close();
                }
            }
        });
    }

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            if !enabled() {
                continue;
            }
            let Some(hwnd) = main_hwnd(&handle) else {
                continue;
            };
            ensure_visible(hwnd);
            make_topmost(hwnd);
        }
    });
}

/// Đặt topmost cho cửa sổ chính (dùng khi app vừa bật).
#[allow(dead_code)]
pub fn assert_topmost<R: Runtime>(app: &AppHandle<R>) {
    if let Some(hwnd) = main_hwnd(app) {
        ensure_visible(hwnd);
        make_topmost(hwnd);
    }
}
