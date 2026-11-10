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
    for (module, symbol) in SYMBOLS {
        if let Ok(addr) = inject::remote_addr(pid, module, symbol) {
            let h = patch::apply(pid, addr, &STUB_RET0, "logfilter")?;
            log::kv("LogFilter", "Applying 3-byte patch at", "addr", &format!("0x{:x}", h.addr));
            return Ok(());
        }
    }
    Err(E::NoTarget)
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
