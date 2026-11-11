//! Kiến thức về Safe Exam Browser 3.x (CefSharp + .NET).
//!
//! SEB 3.x khác SEB 2.x hoàn toàn:
//! - Tiến trình: `SafeExamBrowser.Client.exe` (client), `CefSharp.BrowserSubprocess.exe`
//!   (tiến trình trình duyệt con), `SafeExamBrowser.exe` (runtime host).
//! - Không có `seb.exe` / `xul.exe` ⇒ `GetProcAddress` theo tên export là vô dụng.
//! - Các hàm cần chặn đều là **managed method** trong assembly .NET ⇒ phải đọc
//!   metadata (module `dotnet`) rồi patch IL/native body.
//!
//! Danh sách dưới đây lấy từ bản legacy v0.20.9 (đã chạy được với SEB 3.10.2)
//! và đã đối chiếu byte-for-byte bằng parser của module `dotnet`.

/// Tên tiến trình client (quản lý khóa màn hình, clipboard).
pub const CLIENT_EXE: &str = "SafeExamBrowser.Client.exe";
/// Tiến trình trình duyệt con (gửi log).
pub const BROWSER_SUBPROCESS_EXE: &str = "CefSharp.BrowserSubprocess.exe";
/// Tiến trình runtime host (quản lý desktop, terminate process).
pub const RUNTIME_EXE: &str = "SafeExamBrowser.exe";

/// Assembly chứa `LogRequest.TryExecute` / `LockScreenRequest.TryExecute`.
pub const SERVER_DLL: &str = "SafeExamBrowser.Server.dll";
/// Assembly chứa `KeyboardInterceptor.KeyboardHookCallback`.
pub const MONITORING_DLL: &str = "SafeExamBrowser.Monitoring.dll";

/// Managed method chặn khoá màn hình (bool ⇒ trả false).
pub const TYPE_REQUEST_SESSION_LOCK: &str = "Coordinator.RequestSessionLock";
pub const METHOD_REQUEST_SESSION_LOCK: &str = "RequestSessionLock";
/// Phương án dự phòng nếu type đổi tên.
pub const TYPE_SHOW_LOCK_SCREEN: &str = "ClientResponsibility";
pub const METHOD_SHOW_LOCK_SCREEN: &str = "ShowLockScreen";

/// Managed method chặn truyền log (bool ⇒ trả true ⇒ coi như đã gửi, không thực sự gửi).
pub const TYPE_LOG_REQUEST: &str = "Requests.LogRequest";
pub const TYPE_LOCKSCREEN_REQUEST: &str = "Requests.LockScreenRequest";
pub const METHOD_TRY_EXECUTE: &str = "TryExecute";

/// Managed method chặn hook bàn phím SEB (bool ⇒ trả false ⇒ không chặn phím nào).
pub const TYPE_KEYBOARD_INTERCEPTOR: &str = "Monitoring.Keyboard.KeyboardInterceptor";
pub const METHOD_KEYBOARD_HOOK_CALLBACK: &str = "KeyboardHookCallback";

/// Module native chứa `EmptyClipboard` (xoá clipboard) — patch trong tiến trình client.
pub const USER32_DLL: &str = "user32.dll";
pub const SYMBOL_EMPTY_CLIPBOARD: &str = "EmptyClipboard";

/// Module native chứa `TerminateProcess` — patch trong tiến trình SEB để SEB
/// không kill được tiến trình khác (chống bị đóng app).
pub const KERNEL32_DLL: &str = "kernel32.dll";
pub const SYMBOL_TERMINATE_PROCESS: &str = "TerminateProcess";

/// Module native chứa `ShowWindow` — SEB gọi `ShowWindow(hwnd, SW_HIDE)` để ẩn
/// cửa sổ của ta. Patch để `SW_HIDE` bị bỏ qua (khớp legacy v0.20.9:
/// "ShowWindow filter installed — blocks SW_HIDE").
pub const SYMBOL_SHOW_WINDOW: &str = "ShowWindow";

/// `SW_HIDE` — giá trị mà bộ lọc phải chặn.
pub const SW_HIDE: i32 = 0;

/// `mov eax,1; ret` — thay hàm, trả 1 (thành công) mà không làm gì.
pub const NATIVE_TRUE: [u8; 6] = [0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3];

/// Tiến trình nào nên theo dõi theo từng vai trò.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Client: khoá màn hình, clipboard, khóa log.
    Client,
    /// Runtime host: terminate process, desktop switch.
    Runtime,
    /// Trình duyệt con: gửi log.
    Browser,
}

/// Tên tiến trình ứng với vai trò.
pub fn exe_for(role: Role) -> &'static str {
    match role {
        Role::Client => CLIENT_EXE,
        Role::Runtime => RUNTIME_EXE,
        Role::Browser => BROWSER_SUBPROCESS_EXE,
    }
}

/// Toàn bộ tiến trình SEB đang chạy, theo vai trò.
pub fn running() -> Vec<(Role, u32, String)> {
    let mut out = Vec::new();
    let Ok(list) = crate::process_monitor::snapshot() else {
        return out;
    };
    for (pid, name) in list {
        let role = if name.eq_ignore_ascii_case(CLIENT_EXE) {
            Role::Client
        } else if name.eq_ignore_ascii_case(RUNTIME_EXE) {
            Role::Runtime
        } else if name.eq_ignore_ascii_case(BROWSER_SUBPROCESS_EXE) {
            Role::Browser
        } else {
            continue;
        };
        out.push((role, pid, name));
    }
    out
}

/// PID của tiến trình theo vai trò (None nếu chưa chạy).
pub fn pid_of(role: Role) -> Option<u32> {
    running()
        .into_iter()
        .find(|(r, _, _)| *r == role)
        .map(|(_, pid, _)| pid)
}
