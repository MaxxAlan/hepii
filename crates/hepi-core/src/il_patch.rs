//! Patch IL của managed method trong tiến trình khác (SEB 3.x).
//!
//! Hai dạng body .NET:
//! - **JIT'd native**: RVA trỏ vào code x86/native đã compile → patch `mov eax,0; ret`.
//! - **Raw IL**: RVA trỏ thẳng vào IL bytecode → patch `ldc.i4.0; ret`.
//!
//! Cả hai đều trả `false` (bool) — dùng cho `RequestSessionLock` và
//! `KeyboardHookCallback`.

use windows::Win32::Foundation::HANDLE;

use crate::dotnet::{self, ManagedMethod};
use crate::{log, E, Res};

/// `mov eax, 0; ret` — trả về false cho hàm native đã JIT.
pub const NATIVE_FALSE: [u8; 6] = [0xB8, 0x00, 0x00, 0x00, 0x00, 0xC3];

/// `mov eax, 1; ret` — trả về true cho hàm native đã JIT.
pub const NATIVE_TRUE: [u8; 6] = [0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3];

/// IL: `ldc.i4.0; ret` — trả về false cho method IL thuần.
pub const IL_FALSE: [u8; 2] = [0x16, 0x2A];

/// IL: `ldc.i4.1; ret` — trả về true.
pub const IL_TRUE: [u8; 2] = [0x17, 0x2A];

/// Dấu hiệu code x86 đã JIT (không phải IL).
fn looks_native(code: &[u8]) -> bool {
    // IL hợp lệ bắt đầu bằng opcode đơn; native x64 thường bắt đầu bằng
    // 0x48/0x40/0x55/0x56/0x57 (REX / push rbp / push rsi ...).
    match code.first() {
        Some(&b) => !matches!(b, 0x00..=0x2A | 0x2C..=0x6F | 0x70..=0x8C | 0x8D..=0xFF),
        None => false,
    }
}

/// Patch managed method để trả về `false`.
///
/// - Nếu body là IL thuần → ghi `ldc.i4.0; ret` vào đầu IL.
/// - Nếu body đã JIT thành native → ghi `mov eax,0; ret` vào đầu code.
pub fn patch_return_false(
    handle: HANDLE,
    base: u64,
    m: &ManagedMethod,
) -> Res<()> {
    let mut head = vec![0u8; 16];
    dotnet::read_abs(handle, base + m.rva as u64, &mut head)?;

    // Đọc IL header để biết code bắt đầu ở đâu và dài bao nhiêu.
    let (code_abs, code_len) = if m.raw_il {
        (base + m.rva as u64, 0u32)
    } else if m.tiny {
        (base + m.il_rva as u64, m.il_size)
    } else {
        (base + m.il_rva as u64, m.il_size)
    };

    let mut code = vec![0u8; 16.min(code_len.max(16) as usize)];
    dotnet::read_abs(handle, code_abs, &mut code)?;

    let native = if m.raw_il {
        // RVA trỏ thẳng IL: kiểm tra byte đầu có phải IL hợp lệ không.
        !looks_native(&code)
    } else {
        // Có header: đọc byte IL đầu để quyết định.
        !looks_native(&code)
    };

    if native {
        let patch = NATIVE_FALSE;
        log::kv(
            "IlPatch",
            "Native (JIT) body detected",
            "addr",
            &format!("0x{:x}", code_abs),
        );
        dotnet::write_abs(handle, code_abs, &patch)?;
        log::kv(
            "IlPatch",
            "Patched to return false (native)",
            "method",
            &format!("{}.{}", m.type_name, m.method_name),
        );
    } else {
        let patch = IL_FALSE;
        log::kv(
            "IlPatch",
            "Raw IL body detected",
            "addr",
            &format!("0x{:x}", code_abs),
        );
        dotnet::write_abs(handle, code_abs, &patch)?;
        log::kv(
            "IlPatch",
            "Patched to return false (IL)",
            "method",
            &format!("{}.{}", m.type_name, m.method_name),
        );
    }
    Ok(())
}

/// Patch managed method để trả về `true`.
///
/// Giống `patch_return_false`: xử lý cả IL thuần lẫn native đã JIT.
pub fn patch_return_true(handle: HANDLE, base: u64, m: &ManagedMethod) -> Res<()> {
    let mut head = vec![0u8; 16];
    dotnet::read_abs(handle, base + m.il_rva as u64, &mut head)?;

    // `ldc.i4.1` = 0x17, `ret` = 0x2A, `mov eax,1` = 0xB8 0x01 …
    let native = head[0] == 0xB8 && head[1] == 0x01;
    if native {
        log::kv(
            "IlPatch",
            "Native (JIT) body detected",
            "addr",
            &format!("0x{:x}", base + m.il_rva as u64),
        );
        dotnet::write_abs(handle, base + m.il_rva as u64, &NATIVE_TRUE)?;
        log::kv(
            "IlPatch",
            "Patched to return true (native)",
            "method",
            &format!("{}.{}", m.type_name, m.method_name),
        );
    } else {
        dotnet::write_abs(handle, base + m.il_rva as u64, &IL_TRUE)?;
        log::kv(
            "IlPatch",
            "Patched to return true (IL)",
            "method",
            &format!("{}.{}", m.type_name, m.method_name),
        );
    }
    Ok(())
}

/// Đọc byte đầu method (để log đối chiếu).
pub fn peek(handle: HANDLE, base: u64, m: &ManagedMethod, n: usize) -> Res<Vec<u8>> {
    let mut buf = vec![0u8; n];
    dotnet::read_abs(handle, base + m.rva as u64, &mut buf)?;
    Ok(buf)
}

/// Tìm module base trong tiến trình theo tên (dùng Toolhelp).
pub fn module_base(pid: u32, module: &str) -> Res<u64> {
    crate::inject::module_base_addr(pid, module)
}

/// Quét tất cả module của tiến trình, trả về (tên, base).
pub fn list_modules(pid: u32) -> Vec<(String, u64)> {
    crate::inject::list_modules(pid)
}

/// Tìm managed method trong một module đã nạp (dùng base + size từ Toolhelp).
pub fn find_in_module(
    handle: HANDLE,
    base: u64,
    type_name: &str,
    method_name: &str,
) -> Res<ManagedMethod> {
    let img = dotnet::RemoteImage::new(handle, base);
    dotnet::find_method(&img, type_name, method_name)
}

pub use dotnet::ManagedMethod as Method;

/// Helper: tìm method theo tên method bất kỳ type (khi chưa biết type).
pub fn find_any(handle: HANDLE, base: u64, method_name: &str) -> Res<ManagedMethod> {
    let img = dotnet::RemoteImage::new(handle, base);
    dotnet::find_method_any_type(&img, method_name)
}

/// Trả về Ok nếu không tìm thấy (không phải lỗi nghiêm trọng).
pub fn find_optional(
    handle: HANDLE,
    base: u64,
    type_name: &str,
    method_name: &str,
) -> Option<ManagedMethod> {
    find_in_module(handle, base, type_name, method_name).ok()
}

/// Lỗi chuẩn khi không tìm thấy.
pub fn not_found(what: &str) -> E {
    log::kv("IlPatch", "Method not found", "target", what);
    E::NoTarget
}
