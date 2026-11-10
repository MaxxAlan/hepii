# GAPS — điều chưa xác định

Mỗi mục nêu: câu hỏi, vì sao chưa trả lời được, cách kiểm chứng, ảnh hưởng tới bản dựng lại.
Không mục nào được lấp bằng suy đoán.

## GAP-01 — Release v0.18.0 / v0.14.0 / v0.1.x không tồn tại công khai

- **Thiếu gì:** binary của các tag được yêu cầu trong D2.
- **Bằng chứng:** `docs/raw/releases.json` có 47 tag; danh sách đầy đủ nằm trong
  `docs/raw/evidence/versions.tsv` và mục lục `docs/raw/analysis/`.
  Tag sớm nhất là `v0.2.0`; không có `v0.18.0`, `v0.14.0`, `v0.1.x`.
- **Cách kiểm chứng:** `gh api repos/trfi/hepi-releases/releases --paginate` trả đúng 47 mục;
  `git ls-remote --tags https://github.com/trfi/hepi-releases` không có các tag đó.
- **Ảnh hưởng:** phân tích tiến hoá dùng các mốc thay thế: v0.2.0, v0.5.0, v0.6.0, v0.7.0,
  v0.8.0, v0.9.0, v0.10.0, v0.10.9, v0.20.0, v0.20.8.

## GAP-02 — Tham số và kiểu trả về đầy đủ của 85 IPC command

- **Thiếu gì:** với phần lớn command, frontend chỉ cho thấy tên và đôi khi một object literal;
  kiểu trả về bị dùng ngay trong biểu thức minified nên không suy ra được schema.
- **Bằng chứng:** `docs/raw/ipc-calls.tsv` có 85/85 command kèm snippet gọi (86 candidate ban đầu,
  `network_error` bị loại: không có lời gọi `invoke`, chỉ khớp chuỗi `ENTITLEMENT_NETWORK_ERROR`).
- **Cách kiểm chứng:** chạy bản cũ với WebView2 devtools
  (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`), hook
  `window.__TAURI_INTERNALS__.invoke`, gọi từng command và ghi request/response.
- **Ảnh hưởng:** `docs/IPC.md` đánh `UNKNOWN` ở cột kiểu trả về. Không được đổi kiểu đã
  xác nhận (ví dụ `apply_app_identity` nhận `{request}`).

## GAP-03 — Mẫu byte (signature) của hàm SEB

- **Thiếu gì:** không có mẫu byte nào cho `LogRequest.TryExecute`, `LockScreenRequest.TryExecute`,
  `KeyboardHookCallback`, `EmptyClipboard` wrapper của SEB.
- **Vì sao:** SEB 3.10.2 cài trên máy là assembly .NET; thân hàm bị JIT, không nằm trong file.
  Bản cũ xử lý bằng metadata/JIT scan (R-21..R-23), không dùng mẫu tĩnh đơn giản.
- **Cách kiểm chứng:** bật SEB, dump vùng JIT bằng `VirtualQueryEx` + `ReadProcessMemory` trên
  `SafeExamBrowser.Client.exe`/`SafeExamBrowser.Monitoring.dll`, lưu prologue đã mask theo build.
- **Ảnh hưởng:** mọi bước "quét mẫu byte" trong bản dựng lại phải trả `Pending` khi chưa có mẫu.

## GAP-04 — Public key entitlement và updater

- **Thiếu gì:** nội dung PEM đầy đủ và key id/fingerprint build-time.
- **Bằng chứng:** `strings-hepi.txt` offset `0x01c6bfc7` chỉ thấy `-----END PUBLIC KEY-----`;
  `0x01c4bb18` chỉ thấy tên field `pubkey`, không thấy giá trị.
- **Cách kiểm chứng:** tìm blob PEM trong `.rdata` (ASCII `-----BEGIN PUBLIC KEY-----`), đọc tiếp
  tới `-----END PUBLIC KEY-----`; đối chiếu với chữ ký receipt trong `runtime/appdata/entitlement.json`.
- **Ảnh hưởng:** bản dựng lại không thể xác thực receipt thật cho tới khi có key; hiện khai báo
  trạng thái và không giả chữ ký (R3).

## GAP-05 — Tiêu chí protected/fallback candidate của WdaBypass

- **Thiếu gì:** điều kiện để một cửa sổ vào danh sách "protected" hay "fallback".
- **Bằng chứng:** `strings-hepi.txt` offset `0x01c98d70` và `0x01c98db0` chỉ có tên danh sách.
- **Cách kiểm chứng:** chạy SEB, log danh sách cửa sổ + affinity + style tại thời điểm bypass,
  so sánh hai nhóm.
- **Ảnh hưởng:** bản dựng lại xếp hạng theo affinity/visibility (đã có trong `wda_bypass.rs`),
  đánh dấu là heuristic, không phải parity.

## GAP-06 — Cơ chế ClipboardBypass Layer C

- **Thiếu gì:** Layer C "Neutralized" làm gì trong chế độ Isolated.
- **Bằng chứng:** `strings-hepi.txt` offset `0x01c9a653`, `0x01c9a693`.
- **Cách kiểm chứng:** chạy SEB ở chế độ Isolated, hook `SetClipboardData`/`GetClipboardData`,
  xem Layer C vá vùng nào.
- **Ảnh hưởng:** bản dựng lại chỉ có Layer A/B; Layer C để `UNKNOWN`.

## GAP-07 — Thư mục log mặc định của LogCleaner

- **Thiếu gì:** đường dẫn gốc bản cũ dùng để quét log SEB.
- **Bằng chứng:** runtime thực tế có `C:\Users\HoangManh\AppData\Local\SafeExamBrowser\Logs`;
  binary có `seb_logs` (offset `0x002d2730`) và các mã `SEB_LOG_CLEANER_*`, nhưng không thấy
  chuỗi đường dẫn đầy đủ.
- **Cách kiểm chứng:** chạy bản cũ, gọi `get_seb_log_directory`, so với registry/`%LOCALAPPDATA%`.
- **Ảnh hưởng:** bản dựng lại dò registry → `%LOCALAPPDATA%\SafeExamBrowser\Logs` → scan;
  không hardcode một đường dẫn duy nhất.

## GAP-08 — Run value name và các key `Software\Easy Quiz`

- **Thiếu gì:** tên value Run key, tên key con, và việc app identity sửa `Uninstall` key nào.
- **Bằng chứng:** `strings-hepi.txt` offset `0x01c10b40`: `Software\Easy Quiz`,
  `Software\Microsoft\Windows\CurrentVersion\Run`, `...\Uninstall`; offset `0x01c29708`:
  `Programs`, `Tools`, `app_identity.json`.
- **Cách kiểm chứng:** snapshot `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` và
  `HKCU\Software\Easy Quiz` trước/sau khi gọi `apply_app_identity`, diff.
- **Ảnh hưởng:** bản dựng lại dùng `run_value_name` từ bảng profile; phần `Software\Easy Quiz`
  chưa dựng.

## GAP-09 — Ngưỡng thời gian/retry của bản cũ

- **Thiếu gì:** chu kỳ monitor, timeout remote thread, retry patch, thời gian chờ desktop.
- **Bằng chứng:** chỉ có tên mã lỗi `SEB_VERIFY_PATCH_ENFORCEMENT_TIMEOUT` (offset `0x01c44b88`);
  không thấy hằng số trong chuỗi.
- **Cách kiểm chứng:** đo bằng log runtime (`docs/raw/runtime/logs/*.log`) hoặc debugger.
- **Ảnh hưởng:** `TUNING` của bản dựng lại là giá trị thiết kế, không phải parity số học.

## GAP-10 — `_hepi_helper.exe` khác gì `hepi.exe`

- **Thiếu gì:** mục đích của `_hepi_helper.exe` trong portable v0.20.8.
- **Bằng chứng:** cùng kích thước 39.854.080 B; `analysis/v0.20.8/_hepi_helper-*` có
  import giống hepi.exe (WDA, desktop, hook, remote thread).
- **Cách kiểm chứng:** so hash, so resource/version, chạy với `--help`/không tham số trong VM.
- **Ảnh hưởng:** bản dựng lại chỉ giữ hai binary như yêu cầu; helper không dựng.

## GAP-11 — Schema capture/diagnostics

- **Thiếu gì:** định dạng ảnh trả về (data URL hay base64 thuần), kích thước tối đa, schema report.
- **Bằng chứng:** `capture_persistent_region` nhận `{x,y,width,height}`; `read_files_base64`;
  `diagnostics_save_js_crash_report` nhận `{report}` — chưa thấy kiểu trả về.
- **Cách kiểm chứng:** gọi command qua devtools, log response.
- **Ảnh hưởng:** IPC.md để `UNKNOWN`; bản dựng lại trả data URL PNG (đã có trong code) và ghi rõ.

## GAP-12 — Hành vi cài service của installer

- **Thiếu gì:** installer có tạo service/Windows service nào không, hay chỉ chạy RuntimeHelper.
- **Bằng chứng:** `docs/raw/nsis/` có 11 installer; `RuntimeHelper.dll` x86 nằm trong
  `$PLUGINSDIR`; chưa thấy script NSIS gốc.
- **Cách kiểm chứng:** giải nén NSIS, đọc `[nsExec]`/`nsis` script nếu có; hoặc chạy installer
  trong VM rồi `sc query`, snapshot registry trước/sau.
- **Ảnh hưởng:** bản dựng lại không cài service (đúng luật recon hiện tại); ghi là thiếu parity.

## GAP-13 — Hành vi 32-bit của RuntimeHelper

- **Thiếu gì:** RuntimeHelper làm gì khi tiêm vào tiến trình 32-bit.
- **Bằng chứng:** `RuntimeHelper-pe.txt`: `14C machine (x86)`, DLL, có export directory;
  `hepi.exe` import `IsWow64Process` từ v0.5.0.
- **Cách kiểm chứng:** load RuntimeHelper trong VM 32-bit, hook export; hoặc disassemble export
  table và so với shellcode x64 trong `hepi.exe`.
- **Ảnh hưởng:** nhánh Wow64 của bản dựng lại là thiết kế mới, không phải parity đã chứng minh.
