#![cfg(windows)]
#![windows_subsystem = "windows"]

mod commands;
#[allow(dead_code)]
mod pipe;
mod runtime;

use std::sync::Mutex;

pub struct AppState {
    pub settings: Mutex<hepi_core::settings::Settings>,
    pub receipt: Mutex<Option<hepi_core::entitlement::Receipt>>,
    pub monitor: Mutex<bool>,
}

fn main() {
    if let Err(e) = runtime::boot() {
        eprintln!("[Boot] {} fatal", e.code());
        std::process::exit(1);
    }
}
