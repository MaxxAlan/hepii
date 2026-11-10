//! §6.8 — route chia đôi; teardown xoá đúng route đã thêm.

use std::sync::Mutex;

use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_MULTI_SZ,
};

use crate::util::{run_cmd, to_wide};
use crate::{log, util, E, Res};

static ADDED: Mutex<Vec<String>> = Mutex::new(Vec::new());

const IFACE_KEY: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters";

pub fn gateway() -> Res<String> {
    let sub = to_wide(IFACE_KEY);
    let val = to_wide("DhcpDefaultGateway");
    let mut buf = vec![0u8; 1024];
    let mut cb = buf.len() as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(sub.as_ptr()),
            PCWSTR(val.as_ptr()),
            RRF_RT_REG_MULTI_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut cb),
        )
    };
    if !rc.is_ok() {
        return Err(E::NoTarget);
    }
    let words: Vec<u16> = buf[..cb as usize]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let first = util::from_wide(&words);
    if first.is_empty() {
        return Err(E::NoTarget);
    }
    log::kv("NetworkGuard", "Cached default gateway", "gw", &first);
    Ok(first)
}

pub fn enable() -> Res<()> {
    if !util::is_admin() {
        log::line("NetworkGuard", "Hepi is not running as Administrator. Route defense requires elevation.");
        return Err(E::NoAdmin);
    }
    let gw = gateway()?;
    for mask in ["128.0.0.0", "64.0.0.0"] {
        let (code, _) = run_cmd("route", &["add", "0.0.0.0", "mask", mask, &gw])?;
        if code == 0 {
            ADDED.lock().unwrap().push(mask.to_string());
        }
    }
    log::line("NetworkGuard", "Split route defense enabled");
    Ok(())
}

pub fn teardown() {
    let masks = { ADDED.lock().unwrap().clone() };
    for m in masks {
        let _ = run_cmd("route", &["delete", "0.0.0.0", "mask", &m]);
    }
    ADDED.lock().unwrap().clear();
    log::line("NetworkGuard", "Split route defense disabled and cleaned up");
}

/// Sau khi tiến trình đích chết: nếu route mặc định mất thì thêm lại.
pub fn restore_default_if_missing() {
    let Ok((_, text)) = run_cmd("route", &["print", "0.0.0.0"]) else { return };
    if text.contains("0.0.0.0") && text.contains("On-link") {
        return;
    }
    if let Ok(gw) = gateway() {
        log::line("NetworkGuard", "Default route 0.0.0.0/0 missing after proctor exit; restoring via add");
        let _ = run_cmd("route", &["add", "0.0.0.0", "mask", "0.0.0.0", &gw]);
    }
}

pub fn added_count() -> usize {
    ADDED.lock().unwrap().len()
}
