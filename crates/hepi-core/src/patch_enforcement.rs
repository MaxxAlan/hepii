//! §6.6c — chống SEB đóng app của ta.
//!
//! SEB 3.x có `ApplicationMonitor` dò tiến trình không nằm trong whitelist
//! rồi gửi WM_CLOSE (đã thấy trong log SEB thật: `belongs to not whitelisted
//! process 'hepi.exe'` → `Successfully closed window`). SEB cũng dùng
//! `TerminateProcess` để kết thúc tiến trình lạ.
//!
//! Cơ chế: patch `kernel32!TerminateProcess` ngay trong tiến trình SEB thành
//! `mov eax,1; ret` ⇒ lệnh thành công nhưng không giết ai (khớp legacy v0.20.9).

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

use crate::seb3::{self, Role};
use crate::{log, privs, E, Res};

fn open(pid: u32) -> Res<HANDLE> {
    unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid).map_err(|_| E::Access) }
}

/// Patch `kernel32!TerminateProcess` trong tiến trình SEB.
pub fn apply_for(pid: u32) -> Res<bool> {
    let Ok(addr) = crate::inject::remote_addr(
        pid,
        seb3::KERNEL32_DLL,
        seb3::SYMBOL_TERMINATE_PROCESS,
    ) else {
        return Ok(false);
    };
    let handle = open(pid)?;
    let mut head = vec![0u8; 8];
    let read = crate::dotnet::read_abs(handle, addr as u64, &mut head);
    if read.is_ok() {
        log::kv(
            "PatchEnf",
            "TerminateProcess original bytes",
            "bytes",
            &crate::patch::hex(&head),
        );
    }
    let patched = crate::dotnet::write_abs(handle, addr as u64, &seb3::NATIVE_TRUE);
    unsafe {
        let _ = CloseHandle(handle);
    }
    patched?;
    log::line("PatchEnf", "TerminateProcess patched -> SEB cannot terminate processes");
    Ok(true)
}

/// Chặn SEB giết tiến trình, trên mọi tiến trình SEB đang chạy.
pub fn apply_now() -> Res<bool> {
    let _ = privs::enable_se_debug();
    let mut any = false;
    for (role, pid, _) in seb3::running() {
        // ApplicationMonitor chạy ở client/runtime.
        if role == Role::Browser {
            continue;
        }
        if apply_for(pid)? {
            any = true;
        }
    }
    Ok(any)
}
