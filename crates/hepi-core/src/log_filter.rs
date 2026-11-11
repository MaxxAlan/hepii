//! §6.5 — chặn SEB gửi log (kiosk) ra ngoài.
//!
//! SEB 3.x gửi log qua managed method `LogRequest.TryExecute` và
//! `LockScreenRequest.TryExecute` trong `SafeExamBrowser.Server.dll`.
//! Patch cho chúng luôn trả `true` ⇒ coi như đã gửi, nhưng thực tế không
//! truyền đi (khớp hành vi legacy v0.20.9: "always returns TRUE").
//!
//! Không sửa file trên đĩa (R2) — chỉ ghi vào bộ nhớ tiến trình đích.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

use crate::il_patch;
use crate::seb3::{self, Role};
use crate::{log, privs, E, Res};

fn open(pid: u32) -> Res<HANDLE> {
    unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid).map_err(|_| E::Access) }
}

/// Patch `type.method` trong `module` của tiến trình `pid` cho trả true.
fn patch_true_in(
    pid: u32,
    module: &str,
    type_name: &str,
    method_name: &str,
) -> Res<bool> {
    let base = match crate::inject::module_base_addr(pid, module) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
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
    let r = il_patch::patch_return_true(handle, base, &m);
    unsafe {
        let _ = CloseHandle(handle);
    }
    r?;
    log::kv(
        "LogFilter",
        "TryExecute patched -> always returns TRUE",
        "type",
        &m.type_name,
    );
    Ok(true)
}

/// Chặn gửi log. Trả true nếu patch được ít nhất một method.
pub fn apply_for(pid: u32) -> Res<bool> {
    let _ = privs::enable_se_debug();
    let mut any = false;
    if patch_true_in(
        pid,
        seb3::SERVER_DLL,
        seb3::TYPE_LOG_REQUEST,
        seb3::METHOD_TRY_EXECUTE,
    )? {
        any = true;
    }
    if patch_true_in(
        pid,
        seb3::SERVER_DLL,
        seb3::TYPE_LOCKSCREEN_REQUEST,
        seb3::METHOD_TRY_EXECUTE,
    )? {
        any = true;
    }
    Ok(any)
}

/// Áp cho mọi tiến trình SEB đang chạy (client + runtime + browser subprocess).
pub fn apply_now() -> Res<bool> {
    let _ = privs::enable_se_debug();
    let mut any = false;
    for (role, pid, _) in seb3::running() {
        // Server.dll nạp trong client và runtime; browser subprocess không cần.
        if role == Role::Browser {
            continue;
        }
        if apply_for(pid)? {
            any = true;
        }
    }
    Ok(any)
}
