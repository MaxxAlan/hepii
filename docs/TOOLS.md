# TOOLS — công cụ đã dùng cho recon

Ghi lại đúng lệnh. Không cài thêm gì ngoài bộ đã có trên máy.

| Mục đích | Công cụ | Lệnh / đường dẫn |
|---|---|---|
| Cây thư mục | `tree.exe` (Windows) | `cmd /c "tree /f /a D:\software\Hepi"` |
| Kiểm kê file | PowerShell 5.1 | `Get-ChildItem -Recurse \| Select FullName,Length,LastWriteTime,Attributes` |
| Hash | PowerShell 5.1 | `Get-FileHash -Algorithm SHA256` |
| PE headers/imports/exports/resources | `dumpbin.exe` (Visual Studio Build Tools) | `dumpbin /headers /imports /exports /resources <file>` |
| Chữ ký | `signtool.exe` (Windows SDK) | `signtool verify /pa /v <file>` |
| Chuỗi | `strings.exe` (Sysinternals) | `strings -n 6 <file>` (ASCII + Unicode, offset hex) |
| Trích frontend Tauri | script nội bộ | đọc bảng asset trong `.rdata`, giải brotli → `docs/raw/frontend/` |
| Release list | `gh` CLI | `gh api repos/trfi/hepi-releases/releases --paginate` |
| Tải asset | `gh release download` | `gh release download <tag> -R trfi/hepi-releases -p <asset>` |
| Giải nén NSIS | `7z.exe` | `7z x <setup.exe> -o<dir>` |
| Build / test | `cargo` (rustup toolchain) | `cargo build --workspace`, `cargo test -p hepi-core --lib` |
| Git | `git` 2.55.0.windows.2 | `git init`, `git add`, `git commit` |

## Ghi chú

- Không có `pdb`/`map`/`sourcemap` cho `hepi.exe`, `sidekick.exe`, `uninstall.exe`.
- `strings` xuất offset dạng hex ở đầu mỗi dòng; offset trong `docs/raw/analysis/<ver>/*-strings.txt` là offset file, không phải RVA.
- Mọi lệnh chỉ đọc `D:\software\Hepi`; không sửa, không đổi tên, không xoá.
