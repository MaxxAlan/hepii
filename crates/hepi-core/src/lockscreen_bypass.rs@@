//! §6.6b — chặn khoá workstation của tiến trình đích.

use crate::patch::{self, STUB_RET0};
use crate::{inject, process_monitor, E, Res};

const SYMBOLS: [(&str, &str); 2] = [
    ("seb.exe", "LockWorkStation"),
    ("seb.exe", "EnableLockScreen"),
];

pub fn apply_for(pid: u32) -> Res<()> {
    for (module, symbol) in SYMBOLS {
        if let Ok(addr) = inject::remote_addr(pid, module, symbol) {
            patch::apply(pid, addr, &STUB_RET0, "lockscreen")?;
            return Ok(());
        }
    }
    Err(E::NoTarget)
}

pub fn reconcile() {
    let live = process_monitor::live_pids();
    patch::drop_dead(&live);
    for pid in live {
        if !patch::patched_pids().contains(&pid) {
            let _ = apply_for(pid);
        }
    }
}
