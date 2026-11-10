//! Ống dẫn có PID guard. Client lạ ⇒ đóng, log reject.

use std::io::{Read, Write};
use std::sync::Mutex;

use hepi_core::{log, E, Res};
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;
use windows::Win32::System::Threading::GetCurrentProcessId;

static EXPECTED: Mutex<u32> = Mutex::new(0);
pub const LISTENER: &str = r"\\.\pipe\hepi-single-instance-listener";

pub fn already_running() -> bool {
    let w = hepi_core::util::to_wide(LISTENER);
    unsafe {
        let h = windows::Win32::System::Pipes::CreateFileW(
            PCWSTR(w.as_ptr()),
            0x40000000,
            0,
            None,
            3,
            0,
            None,
        );
        let ok = h.is_ok();
        if let Ok(h) = h {
            let _ = CloseHandle(h);
        }
        ok
    }
}

pub fn set_expected(pid: u32) {
    *EXPECTED.lock().unwrap() = pid;
}

/// Kiểm tra client của pipe có đúng pid mong đợi.
pub fn guard(pipe: windows::Win32::Foundation::HANDLE) -> Res<u32> {
    let mut pid = 0u32;
    unsafe { GetNamedPipeClientProcessId(pipe, &mut pid).map_err(|_| E::Access)? };
    let want = *EXPECTED.lock().unwrap();
    if want != 0 && pid != want {
        log::kv("SidekickPipe", "Rejected pipe client pid", "expected_pid", &format!("{} vs {}", pid, want));
        return Err(E::Access);
    }
    Ok(pid)
}

pub fn self_pid() -> u32 {
    unsafe { GetCurrentProcessId() }
}

/// Ghi một dòng JSON qua ống đã mở (dùng bởi host).
pub fn send(mut stream: impl Write, payload: &str) -> Res<()> {
    stream
        .write_all(payload.as_bytes())
        .and_then(|_| stream.flush())
        .map_err(|_| E::Access)
}

pub fn recv(mut stream: impl Read, buf: &mut [u8]) -> Res<usize> {
    stream.read(buf).map_err(|_| E::Access)
}
