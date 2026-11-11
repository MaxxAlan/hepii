//! §6.6a — bỏ khoá clipboard của tiến trình đích.

use crate::patch::{self, STUB_RET0};
use crate::{inject, log, process_monitor, E, Res};

const SYMBOLS: [(&str, &str); 3] = [
    ("seb.exe", "ClipboardLock"),
    ("xul.dll", "nsClipboard::SetData"),
    ("seb.exe", "IsClipboardLocked"),
];

pub fn apply_for(pid: u32) -> Res<()> {
    let mut last_error = E::NoTarget;

    for (module, symbol) in SYMBOLS {
        match inject::remote_addr(pid, module, symbol) {
            Ok(addr) => {
                match patch::apply(pid, addr, &STUB_RET0, "clipboard") {
                    Ok(h) => {
                        log::kv("ClipboardBypass", "Patched", "symbol", symbol);
                        log::kv("ClipboardBypass", "Applied 3-byte patch at", "addr", &format!("0x{:x}", h.addr));
                        return Ok(());
                    }
                    Err(e) => {
                        log::kv("ClipboardBypass", "Patch failed for symbol", "symbol", symbol);
                        last_error = e;
                        // Try next symbol on failure
                    }
                }
            }
            Err(e) => {
                log::kv("ClipboardBypass", "Symbol not found", "module", module);
                last_error = e;
                // Try next symbol
            }
        }
    }
    Err(last_error)
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
