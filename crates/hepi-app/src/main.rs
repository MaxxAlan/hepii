#![cfg(windows)]
#![windows_subsystem = "windows"]

mod commands;
mod companion;
#[allow(dead_code)]
mod pipe;
mod runtime;
mod selftest;
mod window_guard;

use std::sync::{Arc, Mutex};

pub struct AppState {
    pub settings: Arc<Mutex<hepi_core::settings::Settings>>,  // Arc for shared mutable access across threads
    pub receipt: Mutex<Option<hepi_core::entitlement::Receipt>>,
    pub monitor: Mutex<bool>,
}

fn main() {
    // `--selftest`: chạy headless, in JSON, thoát (không mở cửa sổ).
    if selftest::maybe_run_cli() {
        return;
    }
    if let Err(e) = runtime::boot() {
        eprintln!("[Boot] {} fatal", e.code());
        std::process::exit(1);
    }
}
