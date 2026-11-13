//! Chế độ headless `--selftest`: chạy engine thật, in JSON ra stdout.
//!
//! Dùng cho E2E / CI: xác nhận engine chống SEB 3.x thực sự patch được
//! (đọc byte trong bộ nhớ tiến trình đích sau khi patch), log cleaner có
//! đổi file, Gemini có khoá, v.v.
//!
//! Không mở cửa sổ, không chạy monitor ⇒ chạy được khi SEB đang chiếm màn hình.

use serde_json::json;

use hepi_core::{log_cleaner, patch, seb3, settings, util};

/// Kết quả self-test.
pub fn run() -> serde_json::Value {
    let cfg = settings::Settings::load_or_default(
        std::fs::read_to_string(settings_path()).ok().as_deref(),
    );

    let running = seb3::running();
    let procs: Vec<serde_json::Value> = running
        .iter()
        .map(|(role, pid, name)| {
            json!({ "role": format!("{role:?}").to_lowercase(), "pid": pid, "name": name })
        })
        .collect();

    // --- Áp engine ---
    let log_filter = hepi_core::log_filter::apply_now();
    let clipboard = hepi_core::clipboard_bypass::apply_now();
    let lockscreen = hepi_core::lockscreen_bypass::apply_now();
    let enforcement = hepi_core::patch_enforcement::apply_now();

    // --- Xác minh byte đã patch trong bộ nhớ tiến trình đích ---
    let verify = verify_patches();

    // --- Log cleaner ---
    let log_dir = log_cleaner::default_dir();
    let log_clean = match &log_dir {
        Some(d) => {
            let before = count_marker_hits(d);
            let changed = log_cleaner::clean_dir(d).unwrap_or(0);
            let after = count_marker_hits(d);
            json!({
                "dir": d.to_string_lossy(),
                "filesChanged": changed,
                "markerHitsBefore": before,
                "markerHitsAfter": after,
            })
        }
        None => json!({ "dir": serde_json::Value::Null, "filesChanged": 0 }),
    };

    // --- Gemini ---
    let gemini = json!({
        "keyPresent": hepi_core::gemini::Client::key_present(),
        "model": hepi_core::gemini::DEFAULT_MODEL,
    });

    json!({
        "admin": util::is_admin(),
        "version": env!("CARGO_PKG_VERSION"),
        "sebProcesses": procs,
        "patchedPids": patch::patched_pids(),
        "openPatches": patch::open_count(),
        "engines": {
            "logFilter": res_bool(&log_filter),
            "clipboard": res_bool(&clipboard),
            "lockscreen": res_bool(&lockscreen),
            "patchEnforcement": res_bool(&enforcement),
        },
        "verify": verify,
        "logClean": log_clean,
        "gemini": gemini,
        "windowGuard": true,
        "targetProcesses": cfg.target_processes,
    })
}

/// Bọc `Res<bool>` thành JSON (kèm mã lỗi nếu thất bại).
fn res_bool(r: &hepi_core::Res<bool>) -> serde_json::Value {
    match r {
        Ok(v) => json!({ "ok": true, "value": v }),
        Err(e) => json!({ "ok": false, "error": e.code() }),
    }
}

/// Đếm số dòng log SEB chứa marker (đo trước/sau khi dọn).
fn count_marker_hits(dir: &std::path::Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut hits = 0usize;
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        hits += text
            .lines()
            .filter(|l| log_cleaner::MARKERS.iter().any(|m| l.contains(m)))
            .count();
    }
    hits
}

/// Đọc byte đầu của các managed method đã patch để xác nhận trong RAM.
///
/// Không đụng file trên đĩa (R2) — chỉ đọc bộ nhớ tiến trình đích.
fn verify_patches() -> serde_json::Value {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_ALL_ACCESS};

    use hepi_core::dotnet::{self, RemoteImage};

    let mut out = serde_json::Map::new();

    let checks: Vec<(&str, &str, &str, &str)> = vec![
        (
            "logFilter",
            seb3::SERVER_DLL,
            seb3::TYPE_LOG_REQUEST,
            seb3::METHOD_TRY_EXECUTE,
        ),
        (
            "lockscreen",
            seb3::CLIENT_EXE,
            seb3::TYPE_REQUEST_SESSION_LOCK,
            seb3::METHOD_REQUEST_SESSION_LOCK,
        ),
        (
            "clipboard",
            seb3::MONITORING_DLL,
            seb3::TYPE_KEYBOARD_INTERCEPTOR,
            seb3::METHOD_KEYBOARD_HOOK_CALLBACK,
        ),
        (
            "belongsToSeb",
            seb3::MONITORING_APPS_DLL,
            seb3::TYPE_APPLICATION_MONITOR,
            seb3::METHOD_BELONGS_TO_SEB,
        ),
    ];

    for (label, module, type_name, method_name) in checks {
        let mut entry = serde_json::Map::new();
        let mut found = false;
        for (role, pid, _) in seb3::running() {
            if role == seb3::Role::Browser {
                continue;
            }
            let Ok(base) = hepi_core::inject::module_base_addr(pid, module) else {
                continue;
            };
            let Ok(handle) = (unsafe {
                OpenProcess(PROCESS_ALL_ACCESS, false, pid)
            }) else {
                continue;
            };
            let img = RemoteImage::new(handle, base);
            let Ok(m) = dotnet::find_method(&img, type_name, method_name) else {
                unsafe {
                    let _ = CloseHandle(handle);
                }
                continue;
            };
            // Đọc 6 byte đầu body (IL hoặc native).
            let mut buf = vec![0u8; 6];
            let code_abs = base + m.il_rva as u64;
            let ok = dotnet::read_abs(handle, code_abs, &mut buf).is_ok();
            unsafe {
                let _ = CloseHandle(handle);
            }
            if ok {
                found = true;
                entry.insert(
                    format!("{role:?}").to_lowercase(),
                    json!({
                        "pid": pid,
                        "ilRva": format!("0x{:x}", m.il_rva),
                        "head": patch::hex(&buf),
                        "returnsTrue": buf[0] == 0x17,   // ldc.i4.1
                        "returnsFalse": buf[0] == 0x16,  // ldc.i4.0
                        "nativeTrue": buf[0] == 0xB8 && buf[1] == 0x01,
                        "nativeFalse": buf[0] == 0xB8 && buf[1] == 0x00,
                    }),
                );
            }
        }
        entry.insert("found".into(), json!(found));
        out.insert(label.into(), serde_json::Value::Object(entry));
    }
    serde_json::Value::Object(out)
}

fn settings_path() -> std::path::PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(base)
        .join("com.easyapp.hepi")
        .join("settings.json")
}

/// Entry cho `main`: nếu argv có `--selftest` thì chạy headless rồi thoát.
pub fn maybe_run_cli() -> bool {
    if !std::env::args().any(|a| a == "--selftest") {
        return false;
    }
    // Log của app đi vào file; stdout dành cho JSON kết quả.
    let out = run();
    let text = serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".into());
    println!("{text}");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    std::process::exit(0);
}
