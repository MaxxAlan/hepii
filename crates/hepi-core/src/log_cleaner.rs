//! §6.7 — dọn nội dung log trên đĩa, giữ nguyên mtime.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{log, E, Res};

pub const MARKERS: [&str; 4] = ["CBT", "Screenshot", "Clipboard", "LockScreen"];

pub fn default_dir() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let p = Path::new(&base).join("SafeExamBrowser").join("logs");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Rule §6.7 bước 2: bỏ các dòng chứa marker.
pub fn rewrite(content: &str) -> String {
    content
        .lines()
        .filter(|l| !MARKERS.iter().any(|m| l.contains(m)))
        .map(|l| format!("{}\r\n", l))
        .collect()
}

pub fn clean_dir(dir: &Path) -> Res<usize> {
    let entries = fs::read_dir(dir).map_err(|_| E::NoTarget)?;
    let mut changed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(meta) = fs::metadata(&path) else { continue };
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let next = rewrite(&text);
        if next == text {
            continue;
        }
        let tmp = path.with_extension("hepi.tmp");
        let wrote_tmp = fs::write(&tmp, next.as_bytes()).is_ok()
            && fs::rename(&tmp, &path).is_ok();
        if !wrote_tmp {
            let _ = fs::remove_file(&tmp);
            log::kv("SebLogCleaner", "Atomic replacement unavailable, retrying in place", "file", &path.display().to_string());
            if fs::write(&path, next.as_bytes()).is_err() {
                continue;
            }
        }
        let ft = filetime::FileTime::from_last_modification_time(&meta);
        let fa = filetime::FileTime::from_last_access_time(&meta);
        if filetime::set_file_times(&path, fa, ft).is_err() {
            log::line("SebLogCleaner", "SEB_LOG_CLEANER_TIMESTAMP_RESTORE_FAILED");
        }
        changed += 1;
    }
    log::kv("SebLogCleaner", "Cleaned files", "count", &changed.to_string());
    Ok(changed)
}
