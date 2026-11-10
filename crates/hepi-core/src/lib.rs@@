//! hepi-core — tầng engine. Chỉ Windows.
//!
//! Mọi engine hiện thực đúng khuôn §4: start / verify / patch / rollback / stop.
//! Không engine nào được phán đoán: mọi ngưỡng nằm trong `settings::TUNING`.
#![cfg(windows)]

pub mod app_identity;
pub mod assertions;
pub mod audio;
pub mod capture;
pub mod clipboard_bypass;
pub mod desktop;
pub mod diagnostics;
pub mod entitlement;
pub mod error;
pub mod inject;
pub mod launcher;
pub mod log;
pub mod log_cleaner;
pub mod log_filter;
pub mod lockscreen_bypass;
pub mod network_guard;
pub mod patch;
pub mod pe_patch;
pub mod privs;
pub mod process_monitor;
pub mod settings;
pub mod state;
pub mod util;
pub mod wda_bypass;

pub use error::{Res, E};
