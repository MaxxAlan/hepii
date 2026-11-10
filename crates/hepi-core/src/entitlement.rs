//! §4.3 — challenge/receipt. Chữ ký kiểm cục bộ; ở đây là khung + ghi đè base URL.

use serde::{Deserialize, Serialize};

use crate::{E, Res};

pub const DEFAULT_API: &str = "https://api.hepi.help";
pub const LEGACY_API: [&str; 2] = ["https://api.easyquiz.cc", "https://api.easyquiz.io"];
pub const PUBLIC_KEY_B64: &str = "MCowBQYDK2VwAyEAXPsiIxlXOJe7UbxucA3LmFS7CxynVF5kHPVmo/ZHa8=";

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub jwt: String,
    pub plan_name: String,
    pub features: Vec<String>,
    pub exp: u64,
}

pub fn base_url() -> String {
    std::env::var("HEPI_API_URL").unwrap_or_else(|_| DEFAULT_API.to_string())
}

pub fn is_premium(r: &Receipt) -> bool {
    r.plan_name.to_ascii_lowercase().contains("pro")
}

/// Các năng lực bị chặn khi không có quyền (§8 dòng 3).
pub fn gated(feature: &str, r: Option<&Receipt>) -> Res<()> {
    match r {
        None => Err(E::Unsupported),
        Some(r) if r.features.iter().any(|f| f == feature) => Ok(()),
        Some(_) => Err(E::Unsupported),
    }
}
