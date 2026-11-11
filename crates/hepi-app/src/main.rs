#![cfg(windows)]
#![windows_subsystem = "windows"]

mod commands;
#[allow(dead_code)]
mod pipe;
mod runtime;
mod window_guard;

use std::sync::{Arc, Mutex};

pub struct AppState {
    pub settings: Arc<Mutex<hepi_core::settings::Settings>>,  // Arc for shared mutable access across threads
    pub receipt: Mutex<Option<hepi_core::entitlement::Receipt>>,
    pub monitor: Mutex<bool>,
}

fn main() {
    if let Err(e) = runtime::boot() {
        eprintln!("[Boot] {} fatal", e.code());
        std::process::exit(1);
    }
}
