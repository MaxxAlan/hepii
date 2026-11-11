//! §6.4 — gỡ WDA_EXCLUDEFROMCAPTURE khỏi cửa sổ của tiến trình đích.

use std::collections::HashMap;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowDisplayAffinity, IsWindowVisible, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    WINDOW_DISPLAY_AFFINITY,
};

use crate::settings::TUNING;
use crate::{inject, log, privs, process_monitor, E, Res};

struct RankWithCache {
    best: Option<(HWND, i32)>,
    pid_cache: HashMap<u32, String>,
}

/// Xếp hạng §6.4 bước 2: protected > visible&sized > còn lại; hoà ⇒ pid nhỏ hơn.
pub fn pick_window(targets: &[String]) -> Res<HWND> {
    let hits = process_monitor::find_by_names(targets);
    if hits.is_empty() {
        return Err(E::NoTarget);
    }

    // Cache PID->name mapping from snapshot #1 to avoid recursive snapshot call inside EnumWindows callback
    let pid_cache: HashMap<u32, String> = hits.into_iter().collect();

    let mut r = RankWithCache { best: None, pid_cache };
    unsafe {
        let _ = EnumWindows(Some(rank_cb_cached), LPARAM(&mut r as *mut _ as isize));
    }
    log::kv("WdaBypass", "Window search produced candidate(s), best", "best", &format!("{:?}", r.best));
    r.best.map(|(h, _)| h).ok_or(E::NoTarget)
}

unsafe extern "system" fn rank_cb_cached(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let r = &mut *(lparam.0 as *mut RankWithCache);
    let mut pid = 0u32;
    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
    // Lookup cached PID->name mapping instead of calling snapshot() (which would recurse)
    if !r.pid_cache.contains_key(&pid) {
        return BOOL::from(true);
    }
    let mut aff = 0u32;
    let _ = GetWindowDisplayAffinity(hwnd, &mut aff);
    let score = if aff == WDA_EXCLUDEFROMCAPTURE.0 {
        3
    } else if IsWindowVisible(hwnd).as_bool() {
        2
    } else {
        1
    };
    if r.best.map(|(_, s)| score > s).unwrap_or(true) {
        r.best = Some((hwnd, score));
    }
    BOOL::from(true)
}

pub fn affinity(hwnd: HWND) -> WINDOW_DISPLAY_AFFINITY {
    let mut a = 0u32;
    unsafe {
        let _ = GetWindowDisplayAffinity(hwnd, &mut a);
    }
    WINDOW_DISPLAY_AFFINITY(a)
}

/// Một lượt patch, retry theo TUNING.
pub fn bypass_now(targets: &[String]) -> Res<()> {
    privs::enable_se_debug()?;
    let hwnd = pick_window(targets)?;
    let mut pid = 0u32;
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    let mut last = E::NoTarget;
    for attempt in 1..=TUNING.patch_retry_max {
        match inject::remote_addr(pid, "user32.dll", "SetWindowDisplayAffinity") {
            Ok(fn_addr) => {
                log::kv("WdaBypass", "SetWindowDisplayAffinity at", "addr", &format!("0x{:x}", fn_addr));
                match inject::call_remote(pid, fn_addr, hwnd.0 as u64, WDA_NONE.0) {
                    Ok(()) => {
                        std::thread::sleep(std::time::Duration::from_millis(TUNING.verify_delay_ms));
                        if crate::assertions::wda_clear(hwnd) {
                            log::line("WdaBypass", "Bypass verified");
                            return Ok(());
                        }
                        last = E::Timeout;
                    }
                    Err(e) => last = e,
                }
            }
            Err(e) => last = e,
        }
        std::thread::sleep(std::time::Duration::from_millis(TUNING.patch_retry_delay_ms));
        log::kv("WdaBypass", "Retry", "attempt", &format!("{}/{}", attempt, TUNING.patch_retry_max));
    }
    Err(last)
}
