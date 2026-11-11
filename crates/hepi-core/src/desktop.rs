//! §6.2 — desktop cách ly: tìm, hook bàn phím trên desktop mặc định, chuyển, khôi phục.

use std::sync::Mutex;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, EnumDesktopWindows, EnumDesktopsW, GetProcessWindowStation, OpenDesktopW,
    SetThreadDesktop, SwitchDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_ENUMERATE, DESKTOP_READOBJECTS,
    DESKTOP_SWITCHDESKTOP, DESKTOP_WRITEOBJECTS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_SHIFT, VK_V};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, IsWindowVisible,
    SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, HHOOK,
};

use crate::util::from_wide;
use crate::{log, E, Res};

#[derive(Default, Clone)]
struct State {
    desktop: String,
    hook: isize,
}

static STATE: Mutex<State> = Mutex::new(State { desktop: String::new(), hook: 0 });

fn access() -> u32 {
    DESKTOP_READOBJECTS.0
        | DESKTOP_SWITCHDESKTOP.0
        | DESKTOP_ENUMERATE.0
        | DESKTOP_WRITEOBJECTS.0
}

unsafe extern "system" fn desktop_cb(name: PCWSTR, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<String>);
    // Find null terminator instead of hard-coded 256 to prevent buffer overread
    let mut len = 0usize;
    while *name.0.add(len) != 0 {
        len += 1;
        if len > 512 { break; }  // Guard against infinite loop
    }
    let slice = std::slice::from_raw_parts(name.0, len);
    out.push(from_wide(slice));
    BOOL::from(true)
}

unsafe extern "system" fn wnd_cb(hwnd: windows::Win32::Foundation::HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<u32>);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    out.push(pid);
    BOOL::from(true)
}

/// Toàn bộ desktop không phải mặc định.
pub fn list_desktops() -> Res<Vec<String>> {
    unsafe {
        let winsta = GetProcessWindowStation().map_err(|_| E::Unsupported)?;
        let mut v: Vec<String> = Vec::new();
        EnumDesktopsW(winsta, Some(desktop_cb), LPARAM(&mut v as *mut _ as isize))
            .map_err(|_| E::Unsupported)?;
        v.retain(|d| !d.is_empty() && !d.eq_ignore_ascii_case("Default") && !d.eq_ignore_ascii_case("WinSta0"));
        Ok(v)
    }
}

/// Desktop chứa nhiều cửa sổ của pid nhất (§10 khi nhiều khớp).
pub fn desktop_of_pid(pid: u32) -> Res<String> {
    let mut best = (String::new(), 0usize);
    for name in list_desktops()? {
        let w = crate::util::to_wide(&name);
        let hd = unsafe {
            OpenDesktopW(PCWSTR(w.as_ptr()), DESKTOP_CONTROL_FLAGS(0), false, access())
                .map_err(|_| E::Access)?
        };
        let mut pids: Vec<u32> = Vec::new();
        unsafe {
            let _ = EnumDesktopWindows(hd, Some(wnd_cb), LPARAM(&mut pids as *mut _ as isize));
            let _ = CloseDesktop(hd);
        }
        let n = pids.iter().filter(|p| **p == pid).count();
        if n > best.1 {
            best = (name, n);
        }
    }
    if best.0.is_empty() {
        return Err(E::NoTarget);
    }
    Ok(best.0)
}

pub fn windows_on(desktop: &str) -> Res<Vec<windows::Win32::Foundation::HWND>> {
    let w = crate::util::to_wide(desktop);
    unsafe {
        let hd = OpenDesktopW(PCWSTR(w.as_ptr()), DESKTOP_CONTROL_FLAGS(0), false, access())
            .map_err(|_| E::Access)?;
        let mut out: Vec<windows::Win32::Foundation::HWND> = Vec::new();
        let _ = EnumDesktopWindows(
            hd,
            Some(collect_hwnd),
            LPARAM(&mut out as *mut _ as isize),
        );
        let _ = CloseDesktop(hd);
        Ok(out)
    }
}

unsafe extern "system" fn collect_hwnd(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: LPARAM,
) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<windows::Win32::Foundation::HWND>);
    if IsWindowVisible(hwnd).as_bool() {
        out.push(hwnd);
    }
    BOOL::from(true)
}

// ---- hotkey trên desktop mặc định -------------------------------------------

#[derive(Default, Clone, Copy)]
pub struct Hotkeys {
    pub capture: bool,
    pub hover: bool,
    pub typing: bool,
    pub toggle: bool,
}

static HOTKEYS: Mutex<Hotkeys> = Mutex::new(Hotkeys {
    capture: true,
    hover: true,
    typing: true,
    toggle: true,
});

pub fn set_hotkeys(h: Hotkeys) {
    *HOTKEYS.lock().unwrap() = h;
}

unsafe extern "system" fn hook_cb(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let down = wparam.0 == 0x0100;
        if down {
            let ctrl = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let shift = (windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
            let hk = *HOTKEYS.lock().unwrap();
            if kb.vkCode == VK_V.0 as u32 && ctrl && shift && hk.typing {
                crate::log::line("Kiosk", "hotkey triggerTyping");
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

/// Cài hook LL trên desktop mặc định. MUST chạy trên luồng giữ hook.
pub fn install_hook() -> Res<()> {
    unsafe {
        let h = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_cb), None, 0).map_err(|_| E::Access)?;
        STATE.lock().unwrap().hook = h.0 as isize;
        log::line("Kiosk", "default_desktop_ll_hook installed");
        Ok(())
    }
}

pub fn remove_hook() {
    let mut st = STATE.lock().unwrap();
    if st.hook != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(st.hook as *mut _));
        }
        st.hook = 0;
        log::line("Kiosk", "hook removed");
    }
}

// ---- chuyển desktop ---------------------------------------------------------

/// Gắn luồng hiện tại vào desktop đích rồi hiển thị nó.
pub fn switch_to(desktop: &str) -> Res<()> {
    let w = crate::util::to_wide(desktop);
    unsafe {
        let hd = OpenDesktopW(PCWSTR(w.as_ptr()), DESKTOP_CONTROL_FLAGS(0), false, access())
            .map_err(|_| E::NoTarget)?;
        SetThreadDesktop(hd).map_err(|_| E::Access)?;
        SwitchDesktop(hd).map_err(|_| E::Access)?;
    }
    STATE.lock().unwrap().desktop = desktop.to_string();
    log::kv("Kiosk", "Switched desktop", "name", desktop);
    Ok(())
}

pub fn switch_to_default() -> Res<()> {
    let w = crate::util::to_wide("Default");
    unsafe {
        let hd = OpenDesktopW(PCWSTR(w.as_ptr()), DESKTOP_CONTROL_FLAGS(0), false, access())
            .map_err(|_| E::NoTarget)?;
        let _ = SwitchDesktop(hd);
        let _ = CloseDesktop(hd);
    }
    log::line("Kiosk", "Switched desktop name=Default");
    Ok(())
}

pub fn assert_on(desktop: &str) -> bool {
    let st = STATE.lock().unwrap();
    st.desktop.eq_ignore_ascii_case(desktop)
}

pub fn foreground_in(desktop: &str) -> bool {
    let fg = unsafe { GetForegroundWindow() };
    windows_on(desktop)
        .map(|v| v.contains(&fg))
        .unwrap_or(false)
}

/// Đẩy cửa sổ của tiến trình đích ra khỏi tầm nhìn (không đóng).
pub fn minimize_windows(pids: &[u32]) {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
    for pid in pids {
        let mut data: Vec<windows::Win32::Foundation::HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(wnd_cb_collect), LPARAM(&mut data as *mut _ as isize));
        }
        for hwnd in data {
            let mut p = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut p)) };
            if p == *pid {
                unsafe { let _ = ShowWindow(hwnd, SW_MINIMIZE); }
            }
        }
    }
}

unsafe extern "system" fn wnd_cb_collect(
    hwnd: windows::Win32::Foundation::HWND,
    lparam: LPARAM,
) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<windows::Win32::Foundation::HWND>);
    out.push(hwnd);
    BOOL::from(true)
}
