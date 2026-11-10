//! §5.2 — SeDebugPrivilege. Fail ⇒ E_NO_ADMIN, không thử tiếp.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::util::to_wide;
use crate::{log, E, Res};

pub fn enable_se_debug() -> Res<()> {
    log::line("Privs", "Enabling SeDebugPrivilege...");
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .map_err(|_| E::NoAdmin)?;

        let mut luid = LUID::default();
        let name = to_wide("SeDebugPrivilege");
        LookupPrivilegeValueW(PCWSTR::null(), PCWSTR(name.as_ptr()), &mut luid)
            .map_err(|_| E::NoAdmin)?;

        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        let r = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None);
        let _ = CloseHandle(token);
        r.map_err(|_| E::NoAdmin)?;
        log::line("Privs", "SeDebugPrivilege enabled");
        Ok(())
    }
}
