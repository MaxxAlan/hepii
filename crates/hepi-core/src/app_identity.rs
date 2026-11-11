//! §6.9 — bản sao mang metadata của ứng dụng khác, khả nghịch hoàn toàn.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_READ,
    KEY_WRITE, REG_SZ,
};

use crate::profiles::Profile;
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

    // Verify Authenticode signature using WinVerifyTrust.
    // WTD_REVOKE_NONE: không gọi CRL/OCSP (offline-safe, không treo).
    #[allow(unsafe_code)]
    unsafe {
        use windows::Win32::Foundation::{HWND, TRUST_E_NOSIGNATURE, TRUST_E_SUBJECT_NOT_TRUSTED};
        use windows::Win32::Security::WinTrust::{
            WinVerifyTrustEx, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO,
            WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY,
            WTD_UI_NONE,
        };

        let w = to_wide(path);
        let mut file_info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(w.as_ptr()),
            hFile: Default::default(),
            pgKnownSubject: std::ptr::null_mut(),
        };

        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let mut trust_data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            dwStateAction: WTD_STATEACTION_VERIFY,
            ..Default::default()
        };
        trust_data.Anonymous.pFile = std::ptr::from_mut(&mut file_info);

        let result = WinVerifyTrustEx(HWND::default(), &mut action, &mut trust_data);

        // Đóng state cache do WinVerifyTrust giữ.
        trust_data.dwStateAction = WTD_STATEACTION_CLOSE;
        let _ = WinVerifyTrustEx(HWND::default(), &mut action, &mut trust_data);

        if result == 0 {
            "signed".into()
        } else if result == TRUST_E_NOSIGNATURE.0 {
            "unsigned".into()
        } else if result == TRUST_E_SUBJECT_NOT_TRUSTED.0 {
            "unsigned_untrusted".into()
        } else {
            format!("error:{:08x}", result as u32)
        }
    }
}

/// P1 — nhận `&Profile` thay vì chuỗi rời. Logic clone giữ nguyên.
/// `visual_source` là tệp lấy icon/version/string; `None` ⇒ giữ visual của chính app.
pub fn apply(p: &Profile, visual_source: Option<&str>) -> Res<IdentityState> {
    let own = std::env::current_exe().map_err(|_| E::NoTarget)?;
    let dir = clone_dir().ok_or(E::Unsupported)?;
    fs::create_dir_all(&dir).map_err(|_| E::Access)?;

    let local = std::env::var("LOCALAPPDATA").map_err(|_| E::Unsupported)?;
    let clone = p.clone_path(std::path::Path::new(&local));
    if let Some(parent) = clone.parent() {
        fs::create_dir_all(parent).map_err(|_| E::Access)?;
    }
    if clone.exists() {
        let _ = fs::remove_file(&clone);
    }
    fs::copy(&own, &clone).map_err(|_| E::Access)?;

    let src = visual_source.map(|s| s.to_string()).unwrap_or_default();
    if !src.is_empty() {
        let c = clone.to_string_lossy().to_string();
        if let Err(e) = pe_patch::copy_visuals(&src, &c) {
            log::err("AppIdentity", e, "resource patch degraded");
        }
        if let Err(e) = pe_patch::copy_strings(&src, &c) {
            log::err("AppIdentity", e, "string table degraded");
        }
    }

    run_key_write(p.run_value_name, &format!("\"{}\",0 --identity-uninstall", clone.display()))?;

    let st = IdentityState {
        source: src.clone(),
        clone_path: clone.to_string_lossy().to_string(),
        version: "0.0.0".to_string(), // TODO(sig): đọc từ GetFileVersionInfoW của src
        signature: signature_state(&src),
    };
    if let Some(sp) = state_path() {
        if let Some(parent) = sp.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&sp, serde_json::to_vec_pretty(&st).unwrap_or_default());
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
