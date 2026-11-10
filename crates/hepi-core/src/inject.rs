//! §6.3 — khuôn patch chéo tiến trình: alloc, write, protect, remote thread.
//!
//! Shellcode x64 nhận con trỏ khối tham số qua RCX (lpParameter của CreateRemoteThread):
//!   { fn: u64, a: u64, b: u32 }

use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE,
    TH32CS_SNAPMODULE32,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
};
use windows::Win32::System::Threading::{CreateRemoteThread, OpenProcess, WaitForSingleObject, PROCESS_ALL_ACCESS};

use crate::settings::TUNING;
use crate::util::{from_wide, to_wide};
use crate::{log, E, Res};

/// 48 89 CB            mov rbx, rcx
/// 48 8B 03            mov rax, [rbx]
/// 48 8B 4B 08         mov rcx, [rbx+8]
/// 8B 53 10            mov edx, [rbx+16]
/// 48 83 EC 28         sub rsp, 0x28
/// FF D0               call rax
/// 48 83 C4 28         add rsp, 0x28
/// C3                  ret
pub const CALL2_SHELLCODE: [u8; 18] = [
    0x48, 0x89, 0xCB, 0x48, 0x8B, 0x03, 0x48, 0x8B, 0x4B, 0x08, 0x8B, 0x53, 0x10, 0x48, 0x83,
    0xEC, 0x28, 0xFF,
];

/// Bản đầy đủ 20 byte (gồm call + add + ret).
pub fn shellcode_call2() -> Vec<u8> {
    let mut v = CALL2_SHELLCODE.to_vec();
    v.extend_from_slice(&[0xD0, 0x48, 0x83, 0xC4, 0x28, 0xC3]);
    v
}

/// RVA của symbol trong module, cùng giá trị cho mọi tiến trình cùng session.
pub fn remote_symbol(module: &str, symbol: &str) -> Res<usize> {
    unsafe {
        let m = to_wide(module);
        let h = GetModuleHandleW(windows::core::PCWSTR(m.as_ptr())).map_err(|_| E::NoTarget)?;
        let name = std::ffi::CString::new(symbol).map_err(|_| E::NoTarget)?;
        let f = GetProcAddress(h, PCSTR(name.as_ptr() as *const u8)).ok_or(E::NoTarget)?;
        Ok(f as usize - h.0 as usize)
    }
}

/// Địa chỉ tuyệt đối của symbol trong tiến trình đích (base + RVA).
pub fn remote_addr(pid: u32, module: &str, symbol: &str) -> Res<usize> {
    let rva = remote_symbol(module, symbol)?;
    let base = module_base(pid, module)?;
    Ok(base + rva)
}

fn module_base(pid: u32, module: &str) -> Res<usize> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid)
            .map_err(|_| E::NoTarget)?;
        let mut me = MODULEENTRY32W::default();
        me.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;
        let mut found = None;
        if Module32FirstW(snap, &mut me).is_ok() {
            loop {
                if from_wide(&me.szModule).eq_ignore_ascii_case(module) {
                    found = Some(me.modBaseAddr as usize);
                    break;
                }
                if Module32NextW(snap, &mut me).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        found.ok_or(E::NoTarget)
    }
}

/// Gọi hàm trong tiến trình đích với (u64, u32). Timeout cứng theo TUNING.
pub fn call_remote(pid: u32, fn_addr: usize, a: u64, b: u32) -> Res<()> {
    log::kv("Inject", "Opening target process", "pid", &pid.to_string());
    let h = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid).map_err(|_| E::Access)? };

    let code = shellcode_call2();
    let mut params = Vec::with_capacity(24);
    params.extend_from_slice(&(fn_addr as u64).to_le_bytes());
    params.extend_from_slice(&a.to_le_bytes());
    params.extend_from_slice(&b.to_le_bytes());

    unsafe {
        let mem = VirtualAllocEx(
            h,
            None,
            code.len() + params.len() + 16,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        if mem.is_null() {
            log::line("Inject", "could not allocate memory in target process");
            let _ = CloseHandle(h);
            return Err(E::Access);
        }

        let pblock = (mem as usize + code.len() + 8) as *const u8;
        let mut w = 0usize;
        WriteProcessMemory(h, mem, code.as_ptr() as *const _, code.len(), Some(&mut w)).map_err(|_| E::Access)?;
        WriteProcessMemory(h, pblock as *const _, params.as_ptr() as *const _, params.len(), Some(&mut w))
            .map_err(|_| E::Access)?;

        log::line("Inject", "Creating remote thread...");
        let start: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 = std::mem::transmute(mem);
        let t = CreateRemoteThread(h, None, 0, Some(start), Some(pblock as *const _), 0, None)
            .map_err(|_| E::Access)?;
        let r = WaitForSingleObject(t, TUNING.remote_thread_timeout_ms);
        let _ = CloseHandle(t);
        let _ = VirtualFreeEx(h, mem, 0, MEM_RELEASE);
        let _ = CloseHandle(h);

        if r != WAIT_OBJECT_0 {
            log::line("Inject", "Remote thread timed out");
            return Err(E::Timeout);
        }
        Ok(())
    }
}
