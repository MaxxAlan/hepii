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
