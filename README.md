# Hepi v0.20.8 — bản dựng lại theo FLOW SẠCH

Bản dựng lại từ tài liệu `HEPI_FLOWS.md` + `HEPI_FLOW_SACH.md`. Mọi engine tuân theo khuôn
`start / verify / patch / rollback / stop` (§4) và mười bất biến R1–R10 (§1).

## Cấu trúc

```
crates/hepi-core        engine layer (Win32, chỉ Windows)
crates/hepi-app         Tauri 2 host: UI + IPC + ống dẫn có PID guard
crates/hepi-sidekick    cửa sổ phụ chạy trong desktop đích
crates/hepi-uninstall   trình gỡ 32-bit
ui/                     frontend tĩnh, không cần bundler
```

## Build

```bat
cargo build --workspace            :: kiểm tra toàn bộ crate
cargo build --release
cargo tauri build                  :: cần cargo-tauri 2.x, sinh bộ cài NSIS
```

Bộ icon đã có sẵn ở `crates/hepi-app/icons/`. Nếu thay icon nguồn:
`cargo tauri icon assets\icon.png`.

Yêu cầu: Rust stable, MSVC toolchain, WebView2 runtime, `cargo-tauri` 2.x.
Chạy dev: `cargo tauri dev` (frontend tĩnh nên không cần npm).

## Ánh xạ spec → mã nguồn

| § spec | Tệp |
|---|---|
| R1 handle khả nghịch | `core/src/patch.rs` |
| R3 khai báo unsigned | `core/src/app_identity.rs` (`signature_state`) |
| §5.1 thứ tự khởi động | `app/src/main.rs::boot` |
| §6.1 quan sát tiến trình | `core/src/process_monitor.rs` |
| §6.2 desktop | `core/src/desktop.rs` |
| §6.3 khuôn patch chéo | `core/src/inject.rs` |
| §6.4 WDA | `core/src/wda_bypass.rs` |
| §6.5 log filter | `core/src/log_filter.rs` |
| §6.6 clipboard / lock | `core/src/clipboard_bypass.rs`, `lockscreen_bypass.rs` |
| §6.7 dọn log | `core/src/log_cleaner.rs` |
| §6.8 route | `core/src/network_guard.rs` |
| §6.9 định danh | `core/src/pe_patch.rs`, `app_identity.rs` |
| §9 assert | `core/src/assertions.rs` |
| §10 ngưỡng | `core/src/settings.rs` (`TUNING`) |

## Trạng thái hiện thực

Đã có: quan sát tiến trình, desktop switch + hook, inject khuôn, WDA shellcode, log filter,
clipboard, lockscreen, dọn log, network guard, định danh PE, launcher, diagnostics, updater,
ống dẫn PID guard, uninstall.

Chưa hiện thực (trả `E_UNSUPPORTED`): WASAPI loopback (`audio.rs`), text grabber dùng
`SendInput` cho tổ hợp Ctrl+C (có khung, chưa bind), region selector chụp màn hình.

## Ghi chú

Binary không ký Authenticode (R3): trạng thái `unsigned` được ghi vào state, không giả.
Không sửa tệp của bên thứ ba (R2); mọi thay đổi chéo tiến trình đều nằm trong RAM.
