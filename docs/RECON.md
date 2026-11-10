# RECON — bản đồ hành vi Hepi (bản cũ)

Phạm vi: `D:\software\Hepi` (hepi.exe 39.854.080 B, sidekick.exe 68.588.032 B, uninstall.exe 79.295 B),
các bản trong `docs/raw/bin/<ver>/`, installer trong `docs/raw/nsis/`, runtime capture trong
`docs/raw/runtime/`.

Quy ước bằng chứng: `file:offset` là offset file do `strings` in ra; `file:line` là dòng trong
artifact recon; tên API chỉ được nêu khi thấy trong import table.

| ID | Hành vi | Bằng chứng | Tin cậy | Ghi chú |
|---|---|---|---|---|
| R-01 | App là Tauri 2.10.3, crate Rust `easy_assist_lib` | `strings-hepi.txt` offset `0x019c0108`: `tauri-2.10.3\src\plugin.rs`; offset `0x01c10112`: `easy_assist_lib::modules::network_guard` | CONFIRMED | Không phải Electron/.NET/Go cho binary chính |
| R-02 | hepi.exe và sidekick.exe là PE32+ x64 | `analysis/v0.20.8/hepi-pe.txt`: `8664 machine (x64)`; `sidekick-pe.txt` cùng giá trị | CONFIRMED | uninstall.exe cũng x64 |
| R-03 | Toàn bộ binary không có chữ ký Authenticode | `analysis/v0.20.8/hepi-sig.txt`, `sidekick-sig.txt`, `uninstall-sig.txt`: `SignTool Error: No signature found` | CONFIRMED | R3: bản dựng lại khai báo unsigned |
| R-04 | Có sidecar `sidekick.exe`, giao tiếp qua pipe có kiểm pid | `strings-hepi.txt` offset `0x01c2b53d`: `--pipe`, `HEPI_SIDEKICK_SERVER`; offset `0x01c2a285`: `Ignoring exit request from stale sidekick PID=` | CONFIRMED | Sidecar chạy trong desktop đích |
| R-05 | Frontend là SvelteKit SPA nhúng trong Tauri asset table | `docs/raw/frontend/_manifest.json` (280 asset, 8.250.433 B); `index.html` 1.634 B dùng chung | CONFIRMED | 280 asset + `_manifest.json` = 281 file trên đĩa. Toàn bộ JS/CSS/HTML đã trích |
| R-06 | Plugin Tauri: updater 2.10.0, clipboard-manager, global-shortcut | `strings-hepi.txt` offset `0x01c2cfe0`: `tauri-plugin-updater-2.10.0\src\updater.rs`; offset `0x01c943e8`: `clipboard-manager`, `global-shortcut` | CONFIRMED | |
| R-07 | Khoá cấu hình runtime nằm dưới `%APPDATA%\com.easyapp.hepi` | `strings-hepi.txt` offset `0x01c29708`: `APPDATA`, `com.easyapp.hepi`, `app_identity.json`; `docs/raw/runtime/appdata/settings.json` | CONFIRMED | |
| R-08 | Settings có 3 khoá gốc `rustShortcut`, `selectableModels`, `hepiSettings`; `hepiSettings` có 8 nhóm | `docs/raw/runtime/appdata/settings.json` (8.030 B) | CONFIRMED | Nhóm: general, hotkeys, launcher, meeting, privacy, quiz, textGrabber, window |
| R-09 | Entitlement là receipt `entitlement-ed25519-v1.<payload>.<sig>`, plan `Free`, mọi feature flag false | `docs/raw/runtime/appdata/entitlement.json`; `strings-hepi.txt` offset `0x01c6bfc7`: `-----END PUBLIC KEY-----`, `entitlement-ed25519-v1`, `https://api.hepi.help` | CONFIRMED | Payload base64 JSON có `deviceFingerprint`, `iat`, `exp`, `jti` |
| R-10 | Host mạng: `api.hepi.help`, `api.easyquiz.cc`, `auth.easyquiz.cc`, `storage.easyquiz.cc` | `strings-hepi.txt` offset `0x01c2b53d`: `https://api.hepi.help`; offset `0x01c37d2a`: `https://storage.easyquiz.cc/sidekick.exe`; `business-strings.txt` mục URL | CONFIRMED | Không có endpoint nào bắt buộc trên đường nóng cục bộ |
| R-11 | Updater dùng `tauri-plugin-updater`, có endpoint + pubkey, từ chối release JSON/signature thiếu | `strings-hepi.txt` offset `0x01c4bb18`: `endpoints`, `pubkey`; offset `0x01c779e8`: `the 'signature' field was not set on the updater response` | CONFIRMED | Giá trị pubkey không trích ra ở đây |
| R-12 | Sidecar có thể được tải lại từ `https://storage.easyquiz.cc/sidekick.exe` khi thiếu/hỏng | `strings-hepi.txt` offset `0x01c37d2a`: URL; offset `0x01c37b90`: `Starting background download and restore of sidekick.exe` | CONFIRMED | |
| R-13 | IPC frontend gọi 85 command Tauri (tên xác nhận trong cả JS và chuỗi binary) | `docs/raw/ipc-confirmed.txt` (85 dòng), `docs/raw/ipc-calls.tsv` (85 snippet) | CONFIRMED | 86 candidate ban đầu; `network_error` bị loại vì không có snippet invoke (chỉ khớp `ENTITLEMENT_NETWORK_ERROR`). Tham số/kiểu trả về chỉ xác nhận được một phần — xem `IPC.md` |
| R-14 | WDA bypass: resolve `user32!SetWindowDisplayAffinity`, gọi từ xa bằng shellcode + remote thread | `strings-hepi.txt` offset `0x01c9892a`: `SetWindowDisplayAffinity at 0x`; offset `0x01c989c4`: `WriteProcessMemory (shellcode) failed`; offset `0x01c98a28`: `CreateRemoteThread failed` | CONFIRMED | Không thấy `NtCreateThreadEx` trong import |
| R-15 | WDA bypass có hai danh sách cửa sổ: protected candidates và fallback candidates | `strings-hepi.txt` offset `0x01c98d70`: `Using protected candidates`; offset `0x01c98db0`: `Using fallback candidates` | CONFIRMED | Tiêu chí phân loại: UNKNOWN (GAP-05) |
| R-16 | Clipboard bypass Layer A vá `EmptyClipboard` trong tiến trình SEB | `strings-hepi.txt` offset `0x01c9a55f`: `Layer A: EmptyClipboard patched`; offset `0x01c9a844`: `resolve_api_address(EmptyClipboard)` | CONFIRMED | |
| R-17 | Clipboard bypass Layer B vá `KeyboardHookCallback` trong assembly `SafeExamBrowser.Monitoring` | `strings-hepi.txt` offset `0x01c9a5d9`: `Layer B: KeyboardHookCallback patched`; offset `0x01c9abb8`: `BelongsToSafeExamBrowserKeyboardInterceptor` | CONFIRMED | Patch cả IL tiny/fat và JIT native (offset `0x01c9ad82`) |
| R-18 | Clipboard bypass Layer C xử lý chế độ Isolated, có thể không vá được | `strings-hepi.txt` offset `0x01c9a653`: `Layer C: Neutralized`; offset `0x01c9a693`: `Layer C failed (Isolated mode may still intercept)` | CONFIRMED | Cơ chế Layer C: UNKNOWN (GAP-06) |
| R-19 | Lockscreen bypass ưu tiên `Coordinator.RequestSessionLock`, fallback `ClientResponsibility.ShowLockScreen` | `strings-hepi.txt` offset `0x01c641e9`: `Could not find Coordinator.RequestSessionLock via metadata`; `Trying fallback: ClientResponsibility.ShowLockScreen` | CONFIRMED | |
| R-20 | Lockscreen bypass vá IL hoặc JIT native; stub x86 `mov eax,0; ret` | `strings-hepi.txt` offset `0x01c63e31`: `Method body is JIT'd native`; offset `0x01c63e63`: `using x86 patch (mov eax,0; ret)` | CONFIRMED | |
| R-21 | Log filter có Strategy 1 = parse .NET metadata | `strings-hepi.txt` offset `0x01c64c10`: `Strategy 1: .NET Metadata Parsing` | CONFIRMED | |
| R-22 | Log filter Strategy 2 = quét section `.text` tìm prologue | `strings-hepi.txt` offset `0x01c64ef8`: `Strategy 2: .text section scan`; offset `0x01c64fb0`: `Found method prolog at .text+` | CONFIRMED | |
| R-23 | Log filter có bước quét heap bằng `VirtualQueryEx` và cache RVA | `strings-hepi.txt` offset `0x01c658cf`: `VirtualQueryEx scan of PID`; offset `0x01c65a19`: `Using cached RVA for LockScreenRequest.TryExecute` | CONFIRMED | |
| R-24 | Kiosk bypass Method 1 vá `user32!SwitchDesktop` trong PID SEB | `strings-hepi.txt` offset `0x01c99100`: `user32!SwitchDesktop @`; offset `0x01c99145`: `Method 1: SwitchDesktop hooked in PID=` | CONFIRMED | Có unhook/restore (offset `0x01c9933b`) |
| R-25 | Kiosk bypass Method 2 vá `user32!OpenInputDesktop` | `strings-hepi.txt` offset `0x01c993e8`: `user32!OpenInputDesktop @`; offset `0x01c9942c`: `Method 2: OpenInputDesktop hooked in PID=` | CONFIRMED | Fail cả hai ⇒ log `Both methods failed` |
| R-26 | Desktop đích được tìm bằng enum desktop + cửa sổ SEB, sidekick spawn vào desktop đó | `strings-hepi.txt` offset `0x01c2b53d`: `Sidekick launched on desktop`; `analysis/*/hepi-pe.txt` imports `OpenInputDesktop`, `SwitchDesktop`, `EnumDesktopsW`, `OpenDesktopW`, `SetThreadDesktop` | CONFIRMED | |
| R-27 | Có tiến trình `--kiosk-guardian <parent_pid> <desktop_name>`, tự gắn desktop, WER auto-restart | `strings-hepi.txt` offset `0x01c99f58`: `Usage: --kiosk-guardian <parent_pid> <desktop_name>`; offset `0x01c9a126`: `Registered for WER auto-restart` | CONFIRMED | |
| R-28 | NetworkGuard yêu cầu Administrator, thêm route chia đôi qua default gateway, teardown xoá route đã thêm | `strings-hepi.txt` offset `0x01c10080`: `Route defense requires elevation`; offset `0x01c10112`: `restoring via`; `network_guard.rs` trong bản dựng lại dựa trên các chuỗi này | CONFIRMED | Có đọc gateway từ registry (offset `0x01c101a8`) |
| R-29 | Log cleaner tác động lên log SEB, có backup, kiểm session active và giới hạn kích thước file | `strings-hepi.txt` offset `0x01c14070`: `SEB_LOG_CLEANER_IO_ERROR`; offset `0x01c1ff6c`: `SEB_LOG_CLEANER_SESSION_ACTIVE`; offset `0x01c274a8`: `SEB_LOG_CLEANER_FILE_TOO_LARGE` | CONFIRMED | Thư mục mặc định: GAP-07 |
| R-30 | App identity: copy binary vào `%LOCALAPPDATA%\Programs\Tools`, ghi Run key, có cờ `--identity-uninstall` | `strings-hepi.txt` offset `0x01c29708`: `LOCALAPPDATA`, `Programs`, `Tools`, `app_identity.json`; offset `0x01c10b40`: `Software\Microsoft\Windows\CurrentVersion\Run`; offset `0x01c10bd9`: `--identity-uninstall` | CONFIRMED | Tên value Run key cụ thể: GAP-08 |
| R-31 | App identity còn đụng `Software\Easy Quiz` và `Uninstall` key | `strings-hepi.txt` offset `0x01c10b40`: `Software\Easy Quiz`, `CurrentVersion\Uninstall` | CONFIRMED | Mục đích từng key: GAP-08 |
| R-32 | Tiến trình đích mặc định: `SafeExamBrowser.exe`, `SafeExamBrowser.Client.exe` | `strings-hepi.txt` offset `0x01c30b80`, offset `0x01c96d62` | CONFIRMED | Danh sách đầy đủ (xul/chrome) do config quyết định |
| R-33 | Có patch enforcement monitor và các verify code `SEB_VERIFY_*` | `strings-hepi.txt` offset `0x01c44b88`: `SEB_VERIFY_PATCH_ENFORCEMENT_TIMEOUT`, `SEB_VERIFY_KIOSK_BYPASS_TIMEOUT`, ... | CONFIRMED | Ngưỡng thời gian: UNKNOWN (GAP-09) |
| R-34 | RuntimeHelper x86 được cài riêng, là DLL 32-bit có export | `docs/raw/analysis/v0.20.8/RuntimeHelper-pe.txt`: `14C machine (x86)`, `DLL`, export directory `13AC0`; installer chứa `$PLUGINSDIR\RuntimeHelper.dll` | CONFIRMED | Vai trò chính xác (inject 32-bit?) `INFERRED` từ kiến trúc + tên |
| R-35 | Bản portable v0.20.8 gồm hepi.exe, sidekick.exe, README.txt; có thêm `_hepi_helper.exe` cùng kích thước hepi.exe | `docs/raw/bin/v0.20.8/`; `docs/raw/nsis/Hepi_0.20.8_x64-portable.zip` | CONFIRMED | `_hepi_helper.exe` khác gì: GAP-10 |
| R-36 | Release công khai có 47 tag, không có v0.18.0 và v0.14.0; sớm nhất là v0.2.0 | `docs/raw/releases.json` (47 mục) | CONFIRMED | Yêu cầu tải v0.18/v0.14 không thực hiện được — GAP-01 |
| R-37 | Tiến hoá API theo version: WDA có từ v0.2.0; Wow64/NtQuery từ v0.5.0; OpenClipboard v0.7.0; SetWindowsHookEx v0.8.0; desktop API + module log v0.9.0; sidekick + Clipboard/Lockscreen v0.10.0; RuntimeHelper x86 v0.10.9 | `docs/raw/evidence/versions.tsv`; `analysis/<ver>/hepi-pe.txt` | CONFIRMED | Chi tiết trong `EVOLUTION.md` (đang dựng) |
| R-38 | IPC `apply_app_identity` nhận **một object** `{ request: ... }`, không phải 3 tham số rời | `frontend/_app/immutable/chunks/*.js`: `s("apply_app_identity",{request:e})` | CONFIRMED | Bản dựng lại hiện tại đang lệch hợp đồng này |
| R-39 | Frontend có màn hình/overlay riêng: quiz answer overlay, persistent overlay, region selector, AI response, help panel, launcher | `ipc-confirmed.txt`: `show_quiz_answer_overlay`, `show_persistent_overlay`, `show_region_selector`, `open_ai_response_window`, `toggle_help_panel`, `launcher_rebuild` | CONFIRMED | Chi tiết cửa sổ: `IPC.md` |
| R-40 | Có capture vùng màn hình và persistent region, trả ảnh base64 | `ipc-calls.tsv`: `capture_persistent_region` với `{x,y,width,height}`; `read_files_base64` | CONFIRMED | Định dạng trả về: GAP-11 |

## UNKNOWN

Mọi mục dưới đây nằm trong `docs/GAPS.md`, không suy đoán:

- GAP-01: không có tag v0.18.0 / v0.14.0 / v0.1.x trong release công khai.
- GAP-02: tham số và kiểu trả về đầy đủ của 85 IPC command.
- GAP-03: mẫu byte cụ thể để signature-scan hàm SEB.
- GAP-04: giá trị public key entitlement và updater pubkey.
- GAP-05: tiêu chí phân loại protected/fallback candidate của WdaBypass.
- GAP-06: cơ chế Layer C của ClipboardBypass.
- GAP-07: thư mục log mặc định của LogCleaner.
- GAP-08: tên Run value và toàn bộ key `Software\Easy Quiz` bị sửa.
- GAP-09: toàn bộ ngưỡng thời gian/retry của bản cũ.
- GAP-10: khác biệt `_hepi_helper.exe` so với `hepi.exe`.
- GAP-11: schema JSON/return type của capture và diagnostics.
- GAP-12: hành vi cài service (nếu có) của installer.
- GAP-13: hành vi 32-bit của RuntimeHelper (chỉ suy ra từ kiến trúc x86).
