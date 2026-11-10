//! §5.4 — WASAPI loopback. Khung; chưa bind thiết bị.

use crate::{log, E, Res};

pub fn start() -> Res<()> {
    log::line("Audio", "Starting WASAPI loopback: (not bound)");
    Err(E::Unsupported)
}

pub fn stop() {
    log::line("Audio", "loopback stopped");
}
