//! §6.6a — bỏ khoá clipboard của SEB 3.x.
//!
//! Hai tầng (như legacy v0.20.9):
//! - **Layer A**: patch `user32!EmptyClipboard` ngay trong tiến trình SEB client,
//!   để SEB không xoá clipboard.
//! - **Layer B**: patch managed method
//!   `Monitoring.Keyboard.KeyboardInterceptor.KeyboardHookCallback` trong
//!   `SafeExamBrowser.Monitoring.dll` ⇒ SEB không chặn phím (Ctrl+C/V).
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

/// Layer A — patch `user32!EmptyClipboard` trong tiến trình SEB.
fn layer_a(pid: u32) -> Res<bool> {
    let Ok(addr) = crate::inject::remote_addr(pid, seb3::USER32_DLL, seb3::SYMBOL_EMPTY_CLIPBOARD)
    else {
        return Ok(false);
    };
    let handle = open(pid)?;
    let mut head = vec![0u8; 8];
    let read = crate::dotnet::read_abs(handle, addr as u64, &mut head);
    unsafe {
        let _ = CloseHandle(handle);
    }
    if read.is_err() {
        return Ok(false);
    }
    log::kv(
        "ClipboardBypass",
        "Layer A: EmptyClipboard original bytes",
        "bytes",
        &crate::patch::hex(&head),
    );
    // Ghi đè prolog bằng `mov eax,1; ret` ⇒ EmptyClipboard trả true nhưng không xoá.
    let patched = crate::dotnet::write_abs(handle, addr as u64, &seb3::NATIVE_TRUE);
    unsafe {
        let _ = CloseHandle(handle);
    }
    patched?;
    log::line("ClipboardBypass", "Layer A: EmptyClipboard patched -> clipboard clearing disabled");
    Ok(true)
}

/// Layer B — patch managed `KeyboardHookCallback` trong Monitoring.dll.
fn layer_b(pid: u32) -> Res<bool> {
    let base = match crate::inject::module_base_addr(pid, seb3::MONITORING_DLL) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let handle = open(pid)?;
    let img = crate::dotnet::RemoteImage::new(handle, base);
    let found = crate::dotnet::find_method(
        &img,
        seb3::TYPE_KEYBOARD_INTERCEPTOR,
        seb3::METHOD_KEYBOARD_HOOK_CALLBACK,
    );
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
    log::line("ClipboardBypass", "Layer B: KeyboardHookCallback patched -> all keys pass through");
    Ok(true)
}

/// Chặn clipboard + phím tắt của SEB. Trả true nếu patch được ít nhất một tầng.
pub fn apply_for(pid: u32) -> Res<bool> {
    let _ = privs::enable_se_debug();
    let mut any = false;
    if layer_a(pid)? {
        any = true;
    }
    if layer_b(pid)? {
        any = true;
    }
    Ok(any)
}

/// Áp cho tiến trình client SEB đang chạy.
pub fn apply_now() -> Res<bool> {
    let Some(pid) = seb3::pid_of(Role::Client) else {
        return Ok(false);
    };
    apply_for(pid)
}
