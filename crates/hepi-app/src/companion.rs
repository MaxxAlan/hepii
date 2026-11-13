//! Companion UI trên desktop cách ly của SEB.
//!
//! SEB 3 tạo một desktop riêng (`CreateNewDesktop`) rồi switch sang ⇒ mọi cửa
//! sổ tạo trên 'Default' (kể cả cửa sổ của app) **không hiển thị**. Đó là
//! lý do "SEB khởi động là app lỏ văng mẹ".
//!
//! Legacy v0.20.9 chống bằng `sidekick.exe`: một tiến trình phụ được spawn
//! trên desktop của SEB để UI vẫn thấy được. Ở đây ta làm đúng mô hình đó:
//!
//! - Host (`hepi.exe`): chạy trên 'Default', lo engine (patch SEB), giữ IPC.
//! - Companion (`hepi.exe --companion --desktop <name>`): chạy trên desktop SEB,
//!   chỉ mang UI. Không patch (tránh tranh chấp), chỉ hiển thị + điều khiển.
//!
//! Cơ chế: `SetThreadDesktop` phải được gọi **trước khi** luồng tạo cửa sổ.
//! Vì vậy companion gắn desktop ở đầu `runtime::boot()`, trước khi Tauri dựng
//! cửa sổ.

use std::process::{Command, Stdio};

/// Tham số CLI của companion.
pub const ARG_COMPANION: &str = "--companion";
pub const ARG_DESKTOP: &str = "--desktop";

/// Đọc `--desktop <name>` từ argv.
pub fn desktop_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let i = args.iter().position(|a| a == ARG_DESKTOP)?;
    args.get(i + 1).cloned().filter(|s| !s.is_empty())
}

/// Có đang chạy ở chế độ companion không.
pub fn is_companion() -> bool {
    std::env::args().any(|a| a == ARG_COMPANION)
}

/// Spawn companion lên desktop của SEB. Trả PID nếu thành công.
///
/// Companion là tiến trình con, không chờ ⇒ host không bị treo.
pub fn spawn_companion(desktop_name: &str) -> Option<u32> {
    let exe = std::env::current_exe().ok()?;
    let child = Command::new(exe)
        .arg(ARG_COMPANION)
        .arg(ARG_DESKTOP)
        .arg(desktop_name)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let pid = child.id();
    hepi_core::log::kv(
        "Companion",
        "Spawned companion UI on SEB desktop",
        "desktop",
        desktop_name,
    );
    Some(pid)
}
