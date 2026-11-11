//! §6.5 — chặn hàm ghi log của tiến trình đích bằng stub trong RAM.

use crate::patch::{self, STUB_RET0};
use crate::{inject, log, process_monitor, E, Res};

/// Tên symbol ghi log, thử lần lượt; dừng ở cái đầu tiên resolve được.
const SYMBOLS: [(&str, &str); 4] = [
    ("seb.exe", "WriteLogLine"),
    ("xul.dll", "PR_LogPrint"),
    ("seb.exe", "LogWrite"),
    ("chrome.dll", "logging::LogMessage::LogMessage"),
];

pub fn apply_for(pid: u32) -> Res<()> {
    let mut last_error = E::NoTarget;

    for (module, symbol) in SYMBOLS {
        match inject::remote_addr(pid, module, symbol) {
            Ok(addr) => {
                match patch::apply(pid, addr, &STUB_RET0, "logfilter") {
                    Ok(h) => {
                        log::kv("LogFilter", "Patched", "symbol", symbol);
                        log::kv("LogFilter", "Applying 3-byte patch at", "addr", &format!("0x{:x}", h.addr));
                        return Ok(());
                    }
                    Err(e) => {
                        log::kv("LogFilter", "Patch failed for symbol", "symbol", symbol);
                        last_error = e;
                        // Try next symbol on failure
                    }
                }
            }
            Err(e) => {
                log::kv("LogFilter", "Symbol not found", "module", module);
                last_error = e;
                // Try next symbol
            }
        }
    }
    Err(last_error)
}

/// Giữ patch khớp với pid còn sống (§6.6 monitor pattern).
pub fn reconcile() {
    let live = process_monitor::live_pids();
    patch::drop_dead(&live);
    for pid in live {
        if !patch::patched_pids().contains(&pid) {
            let _ = apply_for(pid);
        }
    }
    if process_monitor::live_pids().is_empty() {
        log::line("LogFilter", "All patched PIDs gone - session reset");
    }
}
