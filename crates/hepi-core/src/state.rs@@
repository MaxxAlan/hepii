//! Trạng thái phiên kiosk, phục hồi sau crash.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Res;

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KioskBypassPersistedState {
    pub seb_desktop_name: String,
    pub patched_pids: Vec<u32>,
    pub timestamp_ms: u128,
}

pub fn path() -> Option<PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(base).join("com.easyapp.hepi").join("kiosk_state.json"))
}

pub fn save(st: &KioskBypassPersistedState) -> Res<()> {
    let p = path().ok_or(crate::E::Unsupported)?;
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let raw = serde_json::to_vec_pretty(st).unwrap_or_default();
    std::fs::write(p, raw).map_err(|_| crate::E::Access)
}

pub fn load() -> Option<KioskBypassPersistedState> {
    serde_json::from_slice(&std::fs::read(path()?).ok()?).ok()
}

pub fn clear() {
    if let Some(p) = path() {
        let _ = std::fs::remove_file(p);
    }
}
