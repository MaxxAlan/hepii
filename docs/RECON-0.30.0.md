# RECON: Hepi v0.30.0 (bản bán lậu) — đối chiếu với v0.20.9

Binary: `D:\project\hepii\Hepi_0.30.0_x64-portable\{Hepi.exe, sidekick.exe}`
Đối chiếu: `C:\Users\HoangManh\Downloads\Hepi_0.20.9_x64-portable\{Hepi.exe, sidekick.exe}`

| File | 0.20.9 | 0.30.0 | SHA256 (0.30.0) |
|---|---|---|---|
| `Hepi.exe` | 39 869 440 | 39 929 344 | `617F8085…DAF52` |
| `sidekick.exe` | 68 544 000 | 68 589 568 | `3F7DA7A5…48BCD` |

Cả hai đều là Tauri 2 + Rust (`easy_assist_lib`), build bằng `cargo` trên `runneradmin`
(đường dẫn `C:\Users\runneradmin\.cargo\registry\…` còn sót trong binary).

## 1. Stack & dependency (0.30.0)

| Thành phần | Version |
|---|---|
| tauri | 2.10.3 |
| tauri-plugin-clipboard-manager | 2.3.2 |
| tauri-plugin-store / updater / dialog / autostart / global-shortcut / http / opener / os / path / process / shell / menu / tray / image / resources | có |
| rusqlite | 0.32.1 (history.db) |
| tokio | 1.50.0 |
| rav1e | 0.8.1 (WASM codec) |
| crossbeam-channel | 0.5.15 |

## 2. Module Rust (0.30.0)

Giống 0.20.9 về danh sách module, **cộng 2 module mới**:

| Module mới | Vai trò (suy ra từ chuỗi log) |
|---|---|
| `src\modules\seb_integrity_probe.rs` | Theo dõi log SEB để phát hiện vi phạm integrity của SEB: `runtime integrity is compromised`, `runtime component violation #`, `runtime behavior violation:` → `attempting to shutdown due to runtime integrity violation`. Đọc log ở `LOCALAPPDATA\SafeExamBrowser\Logs\{runtime,client}`. |
| `src\modules\seb_protection_window.rs` | Module "cửa sổ bảo vệ" — chưa rõ chuỗi log gắn trực tiếp (file nằm giữa các block khác). |

Các module cũ giữ nguyên: `app_identity`, `cross_desktop`, `device_signals`, `history`,
`keyboard_hook`, `launcher`, `network_guard`, `seb_log_cleaner`, `single_instance`,
`topmost_keeper`, `clipboard_bypass`, `kiosk_bypass`, `kiosk_guardian`, `lockscreen_bypass`,
`log_filter`, `patch_enforcement`, `text_grabber`, `wda_bypass`.

## 3. Cơ chế chống SEB — KHÔNG đổi so với 0.20.9

Cả hai bản dùng **cùng bộ target SEB 2.x-style** (module name, không phải tên tiến trình):

```
SafeExamBrowser.exe            (runtime host)
SafeExamBrowser.Client.exe     (client)
SafeExamBrowser.Monitoring     (clipboard layer B: KeyboardInterceptor.KeyboardHookCallback)
SafeExamBrowser.Server.dll     (log filter: LogRequest/LockScreenRequest.TryExecute)
SafeExamBrowser.Browser        (clipboard layer B: RenderProcessMessageHandler.OnContextCreated)
CefSharp.BrowserSubprocess.exe
```

Dấu vết còn trong 0.30.0:
- `[ClipboardBypass] Found 'BelongsToSafeExamBrowser' at …` → tên hàm thực tế trong
  `SafeExamBrowser.Monitoring` khác so với `KeyboardHookCallback` của 0.20.9.
- `[ClipboardBypass] Found RenderProcessMessageHandler.OnContextCreated` → thêm một entry
  clipboard nữa (trong `SafeExamBrowser.Browser`).
- `[LockscreenBypass] Found Client module '…'`, `RequestSessionLock`, `ShowLockScreen` → y hệt 0.20.9.
- `seb-windowsService.exe` → mới: nhắm cả service của SEB.

> **Kết luận quan trọng cho bản dựng lại:** 0.30.0 vẫn là bản SEB 2.x. Bản 0.20.9 đã chạy được
> với SEB **3.10.2** (đã xác minh trong log `Hepi_2026-11-06_*.log`), nghĩa là các sửa đổi
> hỗ trợ SEB 3.x của 0.20.9 **không có** trong 0.30.0. Đừng dùng 0.30.0 làm chuẩn cho SEB 3.x.

## 4. Tính năng mới trong 0.30.0

### 4.1 Hệ thống "SEB verify" (self-check toàn diện)

0.30.0 thêm 13 mã lỗi kiểm tra:

```
SEB_VERIFY_PATCH_ENFORCEMENT_TIMEOUT
SEB_VERIFY_CLICK_THROUGH_STUCK
SEB_VERIFY_MAIN_WINDOW_MISSING
SEB_VERIFY_MAIN_WINDOW_HIDDEN
SEB_VERIFY_MAIN_WINDOW_INVALID_BOUNDS
SEB_VERIFY_MAIN_WINDOW_OCCLUDED_BY_SEB
SEB_VERIFY_KIOSK_BYPASS_TIMEOUT
SEB_VERIFY_SIDEKICK_MISSING_OR_QUARANTINED
SEB_VERIFY_SIDEKICK_PROCESS_DEAD
SEB_VERIFY_SIDEKICK_PIPE_DISCONNECTED
SEB_VERIFY_SIDEKICK_WINDOW_MISSING
SEB_VERIFY_SIDEKICK_WINDOW_HIDDEN
SEB_VERIFY_SIDEKICK_WINDOW_INVALID_BOUNDS
SEB_VERIFY_SIDEKICK_WINDOW_OCCLUDED_BY_SEB
```

⇒ 0.30.0 tự **đo** xem cửa sổ có thấy được không (bị SEB che/ẩn/không hợp lệ),
thay vì chỉ "tin" rằng đã patch. Đây là cải tiến lớn so với 0.20.9.

### 4.2 `PatchEnfMonitor` — tự khôi phục cửa sổ bị SEB ẩn

```
[PatchEnfMonitor] Window '<title>' was hidden by SEB … restoring
```

0.30.0 phát hiện cửa sổ của chính nó bị SEB `ShowWindow(SW_HIDE)` và **restore lại**.
Bản 0.20.9 chỉ có `topmost_keeper` (giữ HWND_TOPMOST) + ShowWindow filter.

### 4.3 Sidekick: giao thức native + quarantine

- `sidekick_native_protocol_version`, `sidekick_native_hotkey`, `sidekick_focused_hotkey`,
  `sidekick_emergency_exit_hotkey` — sidekick là tiến trình native riêng, giao tiếp host
  qua named pipe có version + capability token.
- `SEB_VERIFY_SIDEKICK_QUARANTINED` — sidekick bị SEB "cách ly" (kill) thì host phát hiện
  và khởi động lại.
- `[KioskMonitor] Sidekick connected to pipe`, `[KioskMonitor] JWT sent to sidekick via pipe`.

⇒ Xác nhận kiến trúc 2 tiến trình (host + sidekick trên desktop SEB) là chuẩn của dòng
0.2x/0.3x — bản dựng lại v0.0.1 đã dùng đúng mô hình này (`companion.rs`).

### 4.4 Capture / region selector / quiz answer overlay

Lệnh IPC mới (0.30.0):
```
preload_region_selector   show_region_selector   hide_persistent_overlay
capture_persistent_region capture_region          capture_key_hold_region
close_region_selector     handle_region_selection  sync_help_overlay
show_quiz_answer_overlay  hide_quiz_answer_overlay  reset_quiz_answer_overlay_position
start_quiz_answer_overlay_reposition  finish_quiz_answer_overlay_reposition
start_key_hold_capture    complete_key_hold_capture  cancel_key_hold_capture
start_text_grabber        stop_text_grabber         update_text_grabber_settings
get_text_grabber_settings select_question_number    register_question_selection_shortcuts
```
Kèm frontend events: `quiz-answer-overlay:update`, `region-capture-failed`,
`region-selection-cancelled`, `region-processing-started`, `fullscreen-overlay`,
`region-overlay`, `region-selector`.

⇒ Bản 0.30.0 có **luồng chụp vùng + overlay đáp án** đầy đủ (bản v0.0.1 mới chỉ có nút
chụp vùng cơ bản).

### 4.5 Điều khiển AI (window AI response)

```
open_ai_response_window   close_ai_response_window   sync_ai_response_window_position
focus_main_window          blur_window               move_window
```
Window "ai-response" là cửa sổ trả lời AI riêng (nhỏ, luôn trên cùng).

### 4.6 Launcher, history, diagnostics

- `launcher_search/recent/open/thumbnails`, `launcher_openid`, `cancel_seb_startup`,
  `launch_sidekick_dev`, `restore_sidekick`, `set_kiosk_allow_switch_desktop`.
- History lưu SQLite: `history_list_quiz_items`, `history_save_quiz_item`,
  `history_list_conversations`, `history_save_conversation`, `history_delete_*`.
- Diagnostics: `diagnostics_save_js_crash_report`, `diagnostics_list_pending_reports`,
  `diagnostics_mark_report_uploaded`, `diagnostics_delete_report`,
  `diagnostics_start_session`, `diagnostics_heartbeat`, `diagnostics_open_log_*`.

### 4.7 Stealth / identity

- `registry_disguise.rs` — tự ghi registry uninstall entry (`Software\Easy Quiz\Hepi`),
  tạo shortcut `_Hepi.lnk`, xoá dấu vết (`clean_uninstall`), `disguised_uninstall`.
- `toggle_ghost_mode`, `set_user_click_through`, `set_no_activate_mode`,
  `set_window_virtually_hidden`, `hide_from_task_switcher`.

### 4.8 Clipboard manager + cross-desktop capture

```
tauri-plugin-clipboard-manager-2.3.2
[KioskMonitor] Cross-desktop capture: tab '…'
cross-desktop-capture
```

## 5. Bảng so sánh nhanh

| Hạng mục | 0.20.9 | 0.30.0 | v0.0.1 (đang dựng) |
|---|---|---|---|
| Hỗ trợ SEB 3.10.2 | ✅ (đã verify) | ❌ (vẫn target SEB 2.x) | ✅ (`seb3.rs`, `dotnet.rs`) |
| Clipboard bypass 2 lớp | ✅ | ✅ (+ `BelongsToSafeExamBrowser`, `OnContextCreated`) | ✅ (2 lớp) |
| Lockscreen bypass | ✅ `RequestSessionLock` | ✅ (+ `ShowLockScreen`) | ✅ (+ fallback) |
| Log filter | ✅ | ✅ | ✅ |
| Patch enforcement | ✅ + ShowWindow filter | ✅ + `PatchEnfMonitor` restore | ✅ + window_guard |
| Sidekick 2 tiến trình | ✅ | ✅ + native protocol + quarantine | ✅ (`companion.rs`) |
| Self-verify (đo cửa sổ) | ❌ | ✅ 13 mã SEB_VERIFY_* | ❌ (nên thêm) |
| Chụp vùng + overlay đáp án | ✅ | ✅ (nhiều hơn) | ⚠️ mới có nút chụp cơ bản |
| Launcher / history SQLite | ✅ | ✅ | ❌ (cố tình bỏ) |
| Registry disguise | ✅ | ✅ | ❌ |
| Gemini API | ❌ (dùng backend riêng) | ❌ (dùng backend riêng) | ✅ (`.env` → `GEMINI_API_KEY`) |

## 6. Kết luận & hành động

1. **Không dùng 0.30.0 làm chuẩn cho SEB 3.x.** Bản đó *lùi* về SEB 2.x so với 0.20.9.
   Bản dựng lại v0.0.1 (dựa trên 0.20.9) là hướng đúng cho SEB 3.10.2.
2. **Có giá trị để lấy cơ chế** (không phải byte patch):
   - `PatchEnfMonitor` — phát hiện & restore cửa sổ bị SEB ẩn.
   - `SEB_VERIFY_*` — đo thật trạng thái cửa sổ (bị che/ẩn/không hợp lệ).
   - `BelongsToSafeExamBrowser` — tên hàm thật trong `SafeExamBrowser.Monitoring` của SEB 3.
   - `seb-windowsService.exe` — thêm target service.
3. Cần kiểm chứng `BelongsToSafeExamBrowser` trên SEB 3.10.2 thật (đọc #Strings/#US heap của
   `SafeExamBrowser.Monitoring.dll` cài đặt) trước khi dùng làm target layer B.
