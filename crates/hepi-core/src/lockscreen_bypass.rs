//! §6.6b — chặn khoá màn hình của SEB 3.x.
//!
//! SEB 3.x không export `LockWorkStation`. Cơ chế thật: managed method
//! `Coordinator.RequestSessionLock` (bool) trong `SafeExamBrowser.Client.exe`.
//! Patch cho nó luôn trả `false` ⇒ SEB không khoá màn hình.
//!
//! Dự phòng: `ClientResponsibility.ShowLockScreen` (legacy v0.20.9 dùng fallback này).

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

use crate::il_patch;
use crate::seb3::{self, Role};
use crate::{log, privs, E, Res};

/// Mở tiến trình SEB với đủ quyền để patch.
fn open(pid: u32) -> Res<HANDLE> {
    unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid).map_err(|_| E::Access) }
}

/// Patch một managed method trong module đã nạp của tiến trình `pid`.
fn patch_in_module(
    pid: u32,
    module: &str,
    type_name: &str,
    method_name: &str,
) -> Res<bool> {
    let base = crate::inject::module_base_addr(pid, module)?;
    let handle = open(pid)?;
    let img = crate::dotnet::RemoteImage::new(handle, base);
    let found = crate::dotnet::find_method(&img, type_name, method_name);
    let m = match found {
        Ok(m) => m,
        Err(_) => {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Ok(false);
        }
    };
    crate::dotnet::log_found(&m);
    let r = il_patch::patch_return_false(handle, base, &m);
    unsafe {
        let _ = CloseHandle(handle);
    }
    r?;
    Ok(true)
}

/// Chặn khoá màn hình. Trả true nếu đã patch.
pub fn apply_for(pid: u32) -> Res<bool> {
    let _ = privs::enable_se_debug();
    log::kv("LockscreenBypass", "Starting prevention patch", "pid", &pid.to_string());

    // Đường chính: Coordinator.RequestSessionLock
    if patch_in_module(
        pid,
        seb3::CLIENT_EXE,
        seb3::TYPE_REQUEST_SESSION_LOCK,
        seb3::METHOD_REQUEST_SESSION_LOCK,
    )? {
        log::line("LockscreenBypass", "RequestSessionLock patched -> returns false");
        return Ok(true);
    }
    log::line("LockscreenBypass", "RequestSessionLock not found; trying ShowLockScreen");

    // Dự phòng: ClientResponsibility.ShowLockScreen
    if patch_in_module(
        pid,
        seb3::CLIENT_EXE,
        seb3::TYPE_SHOW_LOCK_SCREEN,
        seb3::METHOD_SHOW_LOCK_SCREEN,
    )? {
        log::line("LockscreenBypass", "ShowLockScreen patched -> returns false");
        return Ok(true);
    }
    Ok(false)
}

/// Áp cho tiến trình client SEB đang chạy.
pub fn apply_now() -> Res<bool> {
    let Some(pid) = seb3::pid_of(Role::Client) else {
        return Ok(false);
    };
    apply_for(pid)
}
