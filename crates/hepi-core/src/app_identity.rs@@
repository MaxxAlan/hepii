//! §6.9 — bản sao mang metadata của ứng dụng khác, khả nghịch hoàn toàn.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_READ,
    KEY_WRITE, REG_SZ,
};

use crate::util::to_wide;
use crate::{log, pe_patch, E, Res};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct IdentityState {
    pub source: String,
    pub clone_path: String,
    pub version: String,
    pub signature: String,
}

pub fn state_path() -> Option<PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(base).join("com.easyapp.hepi").join("app_identity.json"))
}

pub fn clone_dir() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    Some(PathBuf::from(base).join("Programs").join("Tools"))
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn run_key_write(name: &str, value: &str) -> Res<()> {
    unsafe {
        let sub = to_wide(RUN_KEY);
        let mut key = Default::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()), 0, KEY_WRITE, &mut key)
            .ok()
            .map_err(|_| E::Access)?;
        let n = to_wide(name);
        let v = to_wide(value);
        let bytes = std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2);
        let r = RegSetValueExW(key, PCWSTR(n.as_ptr()), 0, REG_SZ, Some(bytes));
        let _ = RegCloseKey(key);
        r.ok().map_err(|_| E::Access)
    }
}

fn run_key_delete(name: &str) {
    unsafe {
        let sub = to_wide(RUN_KEY);
        let mut key = Default::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(sub.as_ptr()), 0, KEY_READ | KEY_WRITE, &mut key).is_ok() {
            let n = to_wide(name);
            let _ = RegDeleteValueW(key, PCWSTR(n.as_ptr()));
            let _ = RegCloseKey(key);
        }
    }
}

/// R3 — khai báo trạng thái chữ ký, không giả.
pub fn signature_state(path: &str) -> String {
    if path.is_empty() || !std::path::Path::new(path).exists() {
        return "unsigned".into();
    }
    "unsigned".into()
}

pub fn apply(source_exe: &str, display_name: &str, version: &str) -> Res<IdentityState> {
    let own = std::env::current_exe().map_err(|_| E::NoTarget)?;
    if own.to_string_lossy().eq_ignore_ascii_case(source_exe) {
        return Err(E::State);
    }
    let dir = clone_dir().ok_or(E::Unsupported)?;
    fs::create_dir_all(&dir).map_err(|_| E::Access)?;
    let clone = dir.join(format!("{}.exe", display_name));
    fs::copy(&own, &clone).map_err(|_| E::Access)?;

    if let Err(e) = pe_patch::copy_visuals(source_exe, &clone.to_string_lossy()) {
        log::err("AppIdentity", e, "resource patch degraded");
    }
    run_key_write(display_name, &format!("\"{}\",0 --identity-uninstall", clone.display()))?;

    let st = IdentityState {
        source: source_exe.to_string(),
        clone_path: clone.to_string_lossy().to_string(),
        version: version.to_string(),
        signature: signature_state(source_exe),
    };
    if let Some(p) = state_path() {
        let _ = fs::create_dir_all(p.parent().unwrap_or(&dir));
        let _ = fs::write(&p, serde_json::to_vec_pretty(&st).unwrap_or_default());
    }
    log::kv("AppIdentity", "Identity applied", "clone", &st.clone_path);
    Ok(st)
}

pub fn reset() -> Res<()> {
    let st = load().unwrap_or_default();
    if !st.clone_path.is_empty() {
        let _ = fs::remove_file(&st.clone_path);
        if let Some(name) = std::path::Path::new(&st.clone_path).file_stem() {
            run_key_delete(&name.to_string_lossy());
        }
    }
    if let Some(p) = state_path() {
        let _ = fs::remove_file(p);
    }
    log::line("AppIdentity", "Identity reset");
    Ok(())
}

pub fn load() -> Option<IdentityState> {
    let p = state_path()?;
    let raw = fs::read(p).ok()?;
    serde_json::from_slice(&raw).ok()
}
