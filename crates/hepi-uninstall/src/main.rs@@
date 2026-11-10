//! Trình gỡ: dừng patch, xoá route, xoá state, xoá bản sao định danh, xoá Run key.

#![cfg(windows)]
#![windows_subsystem = "windows"]

use hepi_core::{app_identity, desktop, network_guard, patch, state};

fn main() {
    println!("[Uninstall] starting");
    patch::rollback_all();
    network_guard::teardown();
    desktop::remove_hook();
    let _ = desktop::switch_to_default();
    let _ = app_identity::reset();
    state::clear();
    if let Some(p) = state::path() {
        let _ = std::fs::remove_file(p);
    }
    if let Some(p) = app_identity::state_path() {
        let _ = std::fs::remove_file(p);
    }
    println!("[Uninstall] done");
    std::thread::sleep(std::time::Duration::from_millis(800));
}
