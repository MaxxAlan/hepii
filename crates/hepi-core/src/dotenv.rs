//! Nạp `.env` tối giản: `KEY=VALUE`, bỏ qua dòng trống và `#`.
//!
//! Không ghi đè biến môi trường đã có (môi trường thật thắng `.env`).
//! Không log giá trị — chỉ log số khoá đã nạp.

use std::path::{Path, PathBuf};

fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join(".env"));
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        for _ in 0..4 {
            let Some(d) = dir else { break };
            out.push(d.join(".env"));
            dir = d.parent().map(Path::to_path_buf);
        }
    }
    out
}

fn parse(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let k = k.trim();
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
        out.push((k.to_string(), v));
    }
    out
}

/// Nạp `.env` từ cwd hoặc thư mục exe (tối đa 4 cấp lên). Trả số khoá mới.
pub fn load() -> usize {
    for path in candidates() {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let mut n = 0usize;
        for (k, v) in parse(&text) {
            if std::env::var_os(&k).is_none() {
                std::env::set_var(&k, v);
                n += 1;
            }
        }
        return n;
    }
    0
}

/// Đọc biến, nạp `.env` một lần nếu chưa có.
pub fn var(key: &str) -> Option<String> {
    if let Ok(v) = std::env::var(key) {
        if !v.trim().is_empty() {
            return Some(v);
        }
    }
    load();
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_basic_lines() {
        let got = parse("# c\n\nA=1\nB = \"two\"\nbad-line\nC='x'\n");
        assert_eq!(
            got,
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "two".to_string()),
                ("C".to_string(), "x".to_string()),
            ]
        );
    }
}
