# Đối chiếu: Hepi v0.20.9 (legacy) ↔ Hepi v0.0.1 (bản dựng lại)

Tham chiếu: `C:\Users\HoangManh\Downloads\Hepi_0.20.9_x64-portable` (legacy, chạy được với SEB 3.10.2).
Nguồn: log runtime legacy `docs/raw/runtime/logs/Hepi_2026-11-06_14-41-08.log` + strings `docs/raw/strings-hepi.txt`.

## 1. Cơ chế chống SEB (đã đối chiếu byte-for-byte)

| Cơ chế | Legacy v0.20.9 | v0.0.1 hiện tại | Trạng thái |
|---|---|---|---|
| Clipboard | Layer A `user32!EmptyClipboard` + Layer B `Monitoring.Keyboard.KeyboardInterceptor.KeyboardHookCallback` → false | `clipboard_bypass.rs` — cùng 2 lớp, cùng target | ✅ |
| Khoá màn hình | `Coordinator.RequestSessionLock` → false | `lockscreen_bypass.rs` — cùng target, fallback `ClientResponsibility.ShowLockScreen` | ✅ |
| Chặn log | `Requests.LogRequest.TryExecute` + `Requests.LockScreenRequest.TryExecute` → luôn TRUE (6 byte) | `log_filter.rs` — cùng 2 method, cùng IL `ldc.i4.1; ret` | ✅ |
| Terminate process | `kernel32!TerminateProcess` → `B8 01 00 00 00 C3` | `patch_enforcement.rs` — cùng byte | ✅ |
| **Đóng cửa sổ lạ** | Không có (0.20.9 dựa vào ShowWindow filter + sidekick) | `patch_enforcement.rs` — patch `ApplicationMonitor.BelongsToSafeExamBrowser` → TRUE. Đây là hàm SEB dùng để quyết định đóng cửa sổ (`belongs to not whitelisted process 'hepi.exe'`). Target lấy từ bản 0.30.0, đã **kiểm chứng trên SEB 3.10.2 thật**: `SafeExamBrowser.Monitoring.dll` → `SafeExamBrowser.Monitoring.Applications.ApplicationMonitor` (37 method), `BelongsToSafeExamBrowser` rva=0x4e04 il_rva=0x4e10 size=114 | ✅ |
| Desktop cách ly | Hook `user32!SwitchDesktop` + `OpenInputDesktop` trong tiến trình SEB | `companion.rs` — spawn UI phụ trên desktop SEB (mô hình `sidekick.exe` của legacy) | ✅ |
| ShowWindow filter | Chặn `SW_HIDE` cho cửa sổ không phải SEB | `window_guard.rs` — chặn `WM_CLOSE` + re-assert topmost mỗi 1 s | ⚠️ khác cơ chế, đều giữ cửa sổ |
| topmost keeper | Có (`topmost_keeper`) | `window_guard.rs` — gộp vào cùng thread | ✅ |
| WDA bypass | `wda_bypass.rs` | `wda_bypass.rs` (giữ nguyên) | ✅ |
| Dọn log | `seb_log_cleaner` — marker theo log SEB thật | `log_cleaner.rs` — marker `ApplicationMonitor`/`WindowGuard`/`KeyboardInterceptor`/`Clipboard`/`LockScreen` | ✅ |
| Log filter monitor | `LogFilterMonitor` tự tắt sau 10 lần poll rỗng | `runtime.rs` — vòng monitor 1 s, áp lại idempotent | ✅ |

## 2. Bền vững engine (đã sửa trong bản dựng lại)

| Vấn đề | Nguyên nhân | Đã sửa |
|---|---|---|
| `target_processes` còn tên SEB **2.x** (`seb.exe`/`xul.exe`) trong `settings.json` cũ ⇒ app **không nhận diện** SEB 3.x | `load_or_default` chỉ thay khi rỗng | `settings.rs` — tự migrate khi còn tên SEB 2.x + 3 unit test |
| Engine ghi đè byte đã patch ⇒ `orig` bị ghi đè ⇒ rollback hỏng | `patch::apply` không kiểm tra idempotent | `patch.rs` — bỏ qua nếu byte đích đã khớp |
| SEB 3 tạo desktop riêng (`CreateNewDesktop`) ⇒ cửa sổ app trên 'Default' **không hiển thị** ("app lỏ") | SEB chuyển desktop | `companion.rs` — spawn `hepi.exe --companion --desktop <name>` gắn vào desktop SEB trước khi dựng cửa sổ |

## 3. Tính năng người dùng

| Tính năng | Legacy v0.20.9 | v0.0.1 |
|---|---|---|
| **Chụp màn hình** | Có (nút chụp, region selector) | **Có** — nút "Chụp màn hình" (overlay kéo chọn vùng) + `capture_screen`/`capture_rect`/`capture_preview` |
| **Gemini vision** (ảnh + câu hỏi) | Có (model hỗ trợ image) | **Có** — `gemini.rs::chat_image` (`inline_data`) + IPC `quiz_ask_image` |
| Chat AI (text) | Có, nhiều model qua entitlement | Có — Gemini API, khoá trong `.env` (`GEMINI_API_KEY`) |
| ChatGPT web trong app | Không rõ | **Có** — `quiz_open_chatgpt` (webview riêng) + đọc cookie phiên |
| Đổi desktop SEB ↔ Default | Có (`CrossDesktop` + `DESKTOP_TOGGLE`) | Có (`switch_to_seb_desktop` / `switch_to_default_desktop`) |
| Launcher / hotkey / text grabber / stealth | Có, nhiều | **Chưa** — bản v0.0.1 cố tình gọn còn 3 panel |
| Bảo vệ nội dung (WDA) | Có | Có (toggle `contentProtection`) |

## 4. UI v0.0.1 — 3 panel

1. **Quiz** — Gemini API (text + ảnh chụp màn hình), ChatGPT web (mở/đóng, trạng thái đăng nhập).
2. **Private** — 11 toggle: chặn SEB, né giám sát, chặn log, dọn log, clipboard, khoá màn hình, đổi desktop, chặn quay màn hình, WDA, chạy cùng Windows, giữ tray + nút thao tác nhanh (áp dụng ngay, dọn log, đổi desktop, thoát khẩn).
3. **Status** — trạng thái AI (khoá Gemini, model, đăng nhập ChatGPT), trạng thái SEB (3 vai trò tiến trình, PID đã patch, handle mở, bảo vệ cửa sổ, thư mục log), engine (admin, tiến trình sống, route), cập nhật (kiểm tra/cài bản mới).

## 5. Updater (tự host)

- Khoá minisign mới: `C:\Users\HoangManh\.tauri\hepii-updater.key` (private) + `.pub`.
- `tauri.conf.json`: `plugins.updater.pubkey` = base64 pubkey, `endpoints` = `https://github.com/MaxxAlan/hepii/releases/latest/download/latest.json`, `bundle.createUpdaterArtifacts = true`.
- CI: `.github/workflows/publish-release.yml` — build NSIS, ký, sinh `latest.json`, phát hành GitHub Release.
- Secrets cần đặt trong repo: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## 6. Bản 0.30.0 (bản bán lậu) — xem `docs/RECON-0.30.0.md`

Kết luận quan trọng: **0.30.0 là bản SEB 2.x** (target `SafeExamBrowser.Client`/`SafeExamBrowser.Monitoring`),
*lùi* so với 0.20.9 vốn đã chạy được với SEB 3.10.2. Không dùng 0.30.0 làm chuẩn cho SEB 3.x.

Nhưng 0.30.0 có 2 cơ chế đáng để lấy (đã port sang v0.0.1):

| Cơ chế | Nguồn | Đã port? |
|---|---|---|
| `ApplicationMonitor.BelongsToSafeExamBrowser` → TRUE (chặn SEB đóng cửa sổ ta) | 0.30.0 clipboard bypass | ✅ `patch_enforcement.rs` |
| `PatchEnfMonitor` phát hiện & restore cửa sổ bị SEB ẩn | 0.30.0 | ⚠️ tương đương `window_guard.rs` (re-assert mỗi 1 s) |
| `SEB_VERIFY_*` (13 mã) đo trạng thái cửa sổ | 0.30.0 | ❌ chưa có — đề xuất bổ sung |
