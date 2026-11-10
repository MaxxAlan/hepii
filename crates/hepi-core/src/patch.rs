//! R1 — mọi sửa đổi chéo tiến trình đi qua PatchHandle có ghi byte gốc.

use std::sync::Mutex;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};
use windows::Win32::System::Memory::{
    VirtualProtectEx, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

use crate::{log, E, Res};

#[derive(Clone, Debug)]
pub struct PatchHandle {
    pub pid: u32,
    pub addr: usize,
    pub orig: Vec<u8>,
    pub old_protect: PAGE_PROTECTION_FLAGS,
    pub label: String,
}

static REGISTRY: Mutex<Vec<PatchHandle>> = Mutex::new(Vec::new());

fn open(pid: u32) -> Res<windows::Win32::Foundation::HANDLE> {
    unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid).map_err(|_| E::Access) }
}

/// Ghi byte mới, lưu byte gốc. Handle được đăng ký để teardown (§9.7).
pub fn apply(pid: u32, addr: usize, new_bytes: &[u8], label: &str) -> Res<PatchHandle> {
    let h = open(pid)?;
    let mut orig = vec![0u8; new_bytes.len()];
    let mut old = PAGE_PROTECTION_FLAGS(0);
    unsafe {
        let mut read = 0usize;
        ReadProcessMemory(
            h,
            addr as *const _,
            orig.as_mut_ptr() as *mut _,
            orig.len(),
            Some(&mut read),
        )
        .map_err(|_| {
            let _ = CloseHandle(h);
            E::Access
        })?;

        VirtualProtectEx(h, addr as *const _, new_bytes.len(), PAGE_EXECUTE_READWRITE, &mut old)
            .map_err(|_| {
                let _ = CloseHandle(h);
                E::Access
            })?;
        log::kv("Patch", "Memory unprotected (old)", "0x", &format!("{:x}", old.0));
        log::kv("Patch", "Applying patch at", "addr", &format!("0x{:x}", addr));
        log::kv("Patch", "Original bytes", "hex", &hex(&orig));

        let mut written = 0usize;
        let r = WriteProcessMemory(
            h,
            addr as *const _,
            new_bytes.as_ptr() as *const _,
            new_bytes.len(),
            Some(&mut written),
        );
        if r.is_err() || written != new_bytes.len() {
            log::kv("Patch", "WriteProcessMemory wrote", "n", &format!("{}/{}", written, new_bytes.len()));
            let _ = VirtualProtectEx(h, addr as *const _, new_bytes.len(), old, &mut old);
            let _ = CloseHandle(h);
            return Err(E::Access);
        }

        let mut tmp = PAGE_PROTECTION_FLAGS(0);
        let _ = VirtualProtectEx(h, addr as *const _, new_bytes.len(), old, &mut tmp);
        let _ = CloseHandle(h);
    }
    log::line("Patch", "Patch applied, protection restored");

    let handle = PatchHandle {
        pid,
        addr,
        orig,
        old_protect: old,
        label: label.to_string(),
    };
    REGISTRY.lock().unwrap().push(handle.clone());
    Ok(handle)
}

/// Ghi lại byte gốc. Tiến trình đã chết ⇒ coi là thành công.
pub fn rollback(h: &PatchHandle) -> Res<()> {
    let proc = match open(h.pid) {
        Ok(p) => p,
        Err(_) => {
            log::kv("Patch", "Rollback skipped, pid gone", "pid", &h.pid.to_string());
            forget(h.pid, h.addr);
            return Ok(());
        }
    };
    unsafe {
        let mut old = PAGE_PROTECTION_FLAGS(0);
        let _ = VirtualProtectEx(proc, h.addr as *const _, h.orig.len(), PAGE_READWRITE, &mut old);
        let mut written = 0usize;
        let _ = WriteProcessMemory(
            proc,
            h.addr as *const _,
            h.orig.as_ptr() as *const _,
            h.orig.len(),
            Some(&mut written),
        );
        let mut restore = PAGE_PROTECTION_FLAGS(0);
        let _ = VirtualProtectEx(proc, h.addr as *const _, h.orig.len(), h.old_protect, &mut restore);
        let _ = CloseHandle(proc);
    }
    forget(h.pid, h.addr);
    log::kv("Patch", "Rolled back", "label", &h.label);
    Ok(())
}

fn forget(pid: u32, addr: usize) {
    REGISTRY.lock().unwrap().retain(|p| !(p.pid == pid && p.addr == addr));
}

/// §9.7 — số handle còn mở.
pub fn open_count() -> usize {
    REGISTRY.lock().unwrap().len()
}

/// Rollback toàn bộ, thứ tự ngược.
pub fn rollback_all() {
    let list = { REGISTRY.lock().unwrap().clone() };
    for h in list.iter().rev() {
        let _ = rollback(h);
    }
}

/// Bỏ handle của pid đã chết (không rollback).
pub fn drop_dead(live: &[u32]) {
    REGISTRY.lock().unwrap().retain(|p| live.contains(&p.pid));
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02X}", x)).collect::<Vec<_>>().join(" ")
}

/// Stub "trả về 0": xor eax,eax; ret
pub const STUB_RET0: [u8; 3] = [0x31, 0xC0, 0xC3];

/// Toàn bộ pid đang được patch.
pub fn patched_pids() -> Vec<u32> {
    let mut v: Vec<u32> = REGISTRY.lock().unwrap().iter().map(|p| p.pid).collect();
    v.sort_unstable();
    v.dedup();
    v
}
