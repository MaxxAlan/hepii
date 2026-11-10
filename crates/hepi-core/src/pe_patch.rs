//! §6.9 bước 3 — thay resource icon/version trong bản sao.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HMODULE};
use windows::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, EnumResourceNamesW, FindResourceW, LoadLibraryExW,
    LoadResource, LockResource, SizeofResource, UpdateResourceW, LOAD_LIBRARY_AS_DATAFILE,
};

use crate::util::to_wide;
use crate::{log, E, Res};

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_STRING: u16 = 6;
#[allow(dead_code)]
const RT_MANIFEST: u16 = 24;

unsafe extern "system" fn enum_cb(
    _h: HMODULE,
    ty: PCWSTR,
    name: PCWSTR,
    lparam: isize,
) -> BOOL {
    let out = &mut *(lparam as *mut Vec<(usize, usize)>);
    out.push((ty.0 as usize, name.0 as usize));
    BOOL::from(true)
}

/// Sao chép toàn bộ icon + version từ `source` sang `clone`.
pub fn copy_visuals(source: &str, clone: &str) -> Res<()> {
    unsafe {
        let s = to_wide(source);
        let c = to_wide(clone);
        let module = LoadLibraryExW(PCWSTR(s.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE)
            .map_err(|_| E::NoTarget)?;
        let update = BeginUpdateResourceW(PCWSTR(c.as_ptr()), false).map_err(|_| E::Access)?;

        let mut ok = true;
        for ty in [RT_ICON, RT_GROUP_ICON, RT_VERSION] {
            let mut names: Vec<(usize, usize)> = Vec::new();
            let _ = EnumResourceNamesW(
                module,
                PCWSTR(ty as usize as *const u16),
                Some(enum_cb),
                &mut names as *mut _ as isize,
            );
            for (_, name_ptr) in names {
                let res = FindResourceW(
                    module,
                    PCWSTR(name_ptr as *const u16),
                    PCWSTR(ty as usize as *const u16),
                );
                if res.is_invalid() {
                    continue;
                }
                let size = SizeofResource(module, res);
                let Ok(data) = LoadResource(module, res) else { continue };
                let raw = LockResource(data);
                if raw.is_null() {
                    ok = false;
                    continue;
                }
                let bytes = std::slice::from_raw_parts(raw as *const u8, size as usize);
                if UpdateResourceW(
                    update,
                    PCWSTR(ty as usize as *const u16),
                    PCWSTR(name_ptr as *const u16),
                    0,
                    Some(bytes.as_ptr() as *const _),
                    size,
                )
                .is_err()
                {
                    log::kv("AppIdentity", "Failed to update resource type", "rt", &ty.to_string());
                    ok = false;
                }
            }
        }
        let _ = EndUpdateResourceW(update, false);
        if ok {
            Ok(())
        } else {
            Err(E::Unsupported)
        }
    }
}

/// Vá string table. `copy_visuals` không đụng RT_STRING nên phải gọi riêng.
/// Bảng string của PE chia block id 1..=16, mỗi block 16 entry length-prefix UTF-16;
/// copy nguyên block là đủ, không cần parse.
pub fn copy_strings(source: &str, clone: &str) -> Res<()> {
    unsafe {
        let s = to_wide(source);
        let c = to_wide(clone);
        let module = LoadLibraryExW(PCWSTR(s.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE)
            .map_err(|_| E::NoTarget)?;
        let update = BeginUpdateResourceW(PCWSTR(c.as_ptr()), false).map_err(|_| E::Access)?;

        let mut names: Vec<(usize, usize)> = Vec::new();
        let _ = EnumResourceNamesW(
            module,
            PCWSTR(RT_STRING as usize as *const u16),
            Some(enum_cb),
            &mut names as *mut _ as isize,
        );

        let mut copied = 0usize;
        for (_, name_ptr) in names {
            let res = FindResourceW(
                module,
                PCWSTR(name_ptr as *const u16),
                PCWSTR(RT_STRING as usize as *const u16),
            );
            if res.is_invalid() {
                continue;
            }
            let size = SizeofResource(module, res);
            let Ok(data) = LoadResource(module, res) else { continue };
            let raw = LockResource(data);
            if raw.is_null() {
                continue;
            }
            let bytes = std::slice::from_raw_parts(raw as *const u8, size as usize);
            if UpdateResourceW(
                update,
                PCWSTR(RT_STRING as usize as *const u16),
                PCWSTR(name_ptr as *const u16),
                0,
                Some(bytes.as_ptr() as *const _),
                size,
            )
            .is_ok()
            {
                copied += 1;
            }
        }
        let _ = EndUpdateResourceW(update, false);
        log::kv("AppIdentity", "String table copied", "blocks", &copied.to_string());
        if copied == 0 {
            return Err(E::Unsupported);
        }
        Ok(())
    }
}

/// assert_pe_1 — bản sao mang đúng version của nguồn.
pub fn assert_version_matches(clone: &str, source: &str) -> bool {
    fn read(path: &str) -> Option<String> {
        use windows::Win32::Storage::FileSystem::{
            GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
        };
        let w = to_wide(path);
        unsafe {
            let size = GetFileVersionInfoSizeW(PCWSTR(w.as_ptr()), None);
            if size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            if GetFileVersionInfoW(PCWSTR(w.as_ptr()), 0, size, buf.as_mut_ptr() as *mut _).is_err() {
                return None;
            }
            let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let mut len = 0u32;
            let sub = to_wide("\\");
            if !VerQueryValueW(buf.as_ptr() as *const _, PCWSTR(sub.as_ptr()), &mut ptr, &mut len).as_bool() {
                return None;
            }
            let fixed = std::slice::from_raw_parts(ptr as *const u32, 4);
            Some(fixed.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("."))
        }
    }
    match (read(clone), read(source)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}
