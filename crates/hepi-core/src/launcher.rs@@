//! §7.11 — liệt kê ứng dụng đã cài (Start Menu) và chạy trong desktop đích.

use std::path::PathBuf;

use serde::Serialize;

use crate::{log, E, Res};

#[derive(Serialize, Clone)]
pub struct Entry {
    pub name: String,
    pub path: String,
}

pub fn scopes() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(a) = std::env::var("APPDATA") {
        v.push(format!("{}\\Microsoft\\Windows\\Start Menu\\Programs", a));
    }
    v.push(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs".to_string());
    v
}

pub fn rebuild() -> Res<Vec<Entry>> {
    let mut out = Vec::new();
    for root in scopes() {
        walk(PathBuf::from(root).as_path(), &mut out, 0);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.path == b.path);
    log::kv("Launcher", "Catalog rebuilt", "entries", &out.len().to_string());
    Ok(out)
}

fn walk(dir: &std::path::Path, out: &mut Vec<Entry>, depth: u32) {
    if depth > 3 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out, depth + 1);
        } else if p.extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false) {
            let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
            out.push(Entry { name, path: p.to_string_lossy().to_string() });
        }
    }
}

pub fn launch(path: &str) -> Res<()> {
    std::process::Command::new("cmd")
        .args(["/c", "start", "", path])
        .spawn()
        .map_err(|_| E::Unsupported)?;
    log::kv("Launcher", "Launched", "path", path);
    Ok(())
}

pub fn assert_launchable(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
