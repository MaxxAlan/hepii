//! §6.1 — quan sát tiến trình, không giữ handle quá một chu kỳ.

use std::sync::Mutex;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

use crate::settings::TUNING;
use crate::util::from_wide;
use crate::{log, E, Res};

#[derive(Default, Clone, Debug)]
pub struct Session {
    pub primary_pid: u32,
    pub pids: Vec<u32>,
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// Toàn bộ (pid, exe) đang chạy.
pub fn snapshot() -> Res<Vec<(u32, String)>> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|_| E::NoTarget)?;
        let mut pe = PROCESSENTRY32W::default();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut out = Vec::new();
        if Process32FirstW(snap, &mut pe).is_ok() {
            loop {
                out.push((pe.th32ProcessID, from_wide(&pe.szExeFile)));
                if Process32NextW(snap, &mut pe).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        Ok(out)
    }
}

/// Tìm tiến trình theo danh sách tên module đích.
pub fn find_by_names(names: &[String]) -> Vec<(u32, String)> {
    match snapshot() {
        Ok(list) => list
            .into_iter()
            .filter(|(_, exe)| names.iter().any(|n| n.eq_ignore_ascii_case(exe)))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Một chu kỳ. Trả true khi phiên kết thúc (2 chu kỳ rỗng liên tiếp, §10).
pub fn tick(names: &[String]) -> bool {
    let hits = find_by_names(names);
    let mut guard = SESSION.lock().unwrap();

    if hits.is_empty() {
        match guard.as_mut() {
            None => return false,
            Some(s) => {
                if s.pids.is_empty() {
                    *guard = None;
                    log::line("ProcessMonitor", "SEB session ended; resetting verifier state");
                    return true;
                }
                s.pids.clear();
                return false;
            }
        }
    }

    let mut pids: Vec<u32> = hits.iter().map(|(p, _)| *p).collect();
    pids.sort_unstable();
    let primary = pids[0];
    let fresh = guard.is_none();
    *guard = Some(Session { primary_pid: primary, pids: pids.clone() });
    if fresh {
        log::kv("ProcessMonitor", "Tracking new SEB session (primary_pid)", "pids", &format!("{:?}", pids));
    }
    false
}

pub fn current() -> Option<Session> {
    SESSION.lock().unwrap().clone()
}

pub fn live_pids() -> Vec<u32> {
    SESSION.lock().unwrap().as_ref().map(|s| s.pids.clone()).unwrap_or_default()
}

pub fn cycle_ms() -> u64 {
    TUNING.process_monitor_cycle_ms as u64
}

/// Tiến trình `pid` có tên module khớp `exe` không (không phân biệt hoa thường).
pub fn pid_matches(pid: u32, exe: &str) -> bool {
    snapshot()
        .map(|list| {
            list.iter()
                .any(|(p, name)| *p == pid && exe.eq_ignore_ascii_case(name))
        })
        .unwrap_or(false)
}
