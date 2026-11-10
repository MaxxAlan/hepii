//! §9 — phiên chẩn đoán, hàng đợi báo cáo, backoff cố định.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings::TUNING;
use crate::{E, Res};

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionMarker {
    pub started_at: u128,
    pub heartbeat_at: u128,
    pub app_version: String,
    pub session_id: String,
}

fn dir() -> Option<PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(base).join("com.easyapp.hepi").join("diagnostics"))
}

pub fn start_session(version: &str) -> Res<SessionMarker> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let m = SessionMarker {
        started_at: now,
        heartbeat_at: now,
        app_version: version.to_string(),
        session_id: format!("{:x}", now),
    };
    if let Some(d) = dir() {
        let _ = std::fs::create_dir_all(&d);
        let _ = std::fs::write(
            d.join(format!("{}.json", m.session_id)),
            serde_json::to_vec_pretty(&m).unwrap_or_default(),
        );
    }
    Ok(m)
}

pub fn heartbeat(m: &SessionMarker) -> Res<()> {
    let mut m2 = m.clone();
    m2.heartbeat_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let d = dir().ok_or(E::Unsupported)?;
    std::fs::write(
        d.join(format!("{}.json", m.session_id)),
        serde_json::to_vec_pretty(&m2).unwrap_or_default(),
    )
    .map_err(|_| E::Access)
}

pub fn backoff_ms(attempt: usize) -> u64 {
    TUNING
        .diag_backoff_s
        .get(attempt.min(2))
        .copied()
        .unwrap_or(600)
        * 1000
}
