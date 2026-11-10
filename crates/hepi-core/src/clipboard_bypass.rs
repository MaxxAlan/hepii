//! §6.6a — bỏ khoá clipboard của tiến trình đích.

use crate::patch::{self, STUB_RET0};
use crate::{inject, log, process_monitor, E, Res};

const SYMBOLS: [(&str, &str); 3] = [
    ("seb.exe", "ClipboardLock"),
    ("xul.dll", "nsClipboard::SetData"),
    ("seb.exe", "IsClipboardLocked"),
];

pub fn apply_for(pid: u32) -> Res<()> {
    for (module, symbol) in SYMBOLS {
        if let Ok(addr) = inject::remote_addr(pid, module, symbol) {
            patch::apply(pid, addr, &STUB_RET0, "clipboard")?;
            log::line("ClipboardBypass", "Patched");
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
