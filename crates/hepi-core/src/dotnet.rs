//! Đọc .NET metadata để tìm managed method trong PE của tiến trình khác.
//!
//! SEB 3.x (CefSharp) là managed: các hàm cần chặn (`RequestSessionLock`,
//! `LogRequest.TryExecute`, `KeyboardInterceptor.KeyboardHookCallback`) không nằm
//! trong export table nên `GetProcAddress` vô dụng. Phải đọc metadata
//! (stream `#~`, bảng `TypeDef`/`MethodDef`, heap `#Strings`) rồi lấy RVA.
//!
//! Không sửa file trên đĩa (R2): mọi thao tác đi qua bộ nhớ tiến trình đích.

use std::ffi::c_void;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};

use crate::{log, E, Res};

// ---- Image abstraction -------------------------------------------------------

/// Vùng đọc được: bộ nhớ tiến trình khác (remote) hoặc file trên đĩa (local).
pub trait Image {
    /// Đọc `len` byte tại RVA (relative image base).
    fn read_rva(&self, rva: u32, buf: &mut [u8]) -> Res<()>;
    /// Đọc `len` byte tại offset thô từ đầu image (PE header nằm ngoài section).
    fn read_at(&self, off: u32, buf: &mut [u8]) -> Res<()>;
}

// ---- PE helpers --------------------------------------------------------------

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// Section header của PE.
#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub va: u32,
    pub vsize: u32,
    pub raw_ptr: u32,
    pub raw_size: u32,
}

/// Đọc section table từ PE header.
pub fn parse_sections(pe: &[u8]) -> Res<Vec<Section>> {
    if pe.len() < 0x40 || &pe[0..2] != b"MZ" {
        return Err(E::Unsupported);
    }
    let lfanew = u32_at(pe, 0x3C) as usize;
    if lfanew + 24 > pe.len() || &pe[lfanew..lfanew + 4] != b"PE\0\0" {
        return Err(E::Unsupported);
    }
    let coff = lfanew + 4;
    let n_sections = u16_at(pe, coff + 2) as usize;
    let opt_size = u16_at(pe, coff + 16) as usize;
    let opt = coff + 20;
    let sect_off = opt + opt_size;

    let mut out = Vec::with_capacity(n_sections);
    for i in 0..n_sections {
        let s = sect_off + i * 40;
        if s + 40 > pe.len() {
            break;
        }
        out.push(Section {
            vsize: u32_at(pe, s + 8),
            va: u32_at(pe, s + 12),
            raw_size: u32_at(pe, s + 16),
            raw_ptr: u32_at(pe, s + 20),
        });
    }
    Ok(out)
}

/// Data directory index 14 = CLR Runtime Header.
pub const DIR_CLR: usize = 14;

/// RVA của data directory `idx` (0 nếu không có).
pub fn data_dir_rva(pe: &[u8], idx: usize) -> u32 {
    let lfanew = u32_at(pe, 0x3C) as usize;
    let coff = lfanew + 4;
    let opt = coff + 20;
    let magic = u16_at(pe, opt);
    let dd_off = opt + if magic == 0x20B { 112 } else { 96 };
    let o = dd_off + idx * 8;
    if o + 8 > pe.len() {
        return 0;
    }
    u32_at(pe, o)
}

/// Image base của PE (chỉ để hiển thị; tính toán dùng RVA).
pub fn image_base(pe: &[u8]) -> u64 {
    let lfanew = u32_at(pe, 0x3C) as usize;
    let opt = lfanew + 4 + 20;
    let magic = u16_at(pe, opt);
    let off = opt + if magic == 0x20B { 24 } else { 28 };
    if off + 8 > pe.len() {
        return 0;
    }
    u64::from(u32_at(pe, off))
}

// ---- .NET metadata -----------------------------------------------------------

/// Vị trí một managed method trong module.
#[derive(Debug, Clone)]
pub struct ManagedMethod {
    pub type_name: String,
    pub method_name: String,
    /// RVA của method (theo MethodDef) — trỏ vào method header.
    pub rva: u32,
    /// RVA của IL code (sau header).
    pub il_rva: u32,
    /// Kích thước IL code.
    pub il_size: u32,
    /// true nếu method dùng tiny header (1 byte).
    pub tiny: bool,
    /// true nếu RVA trỏ thẳng vào IL (không có header).
    pub raw_il: bool,
}

/// Một hàng TypeDef.
#[derive(Debug, Clone)]
pub struct TypeRow {
    pub namespace: String,
    pub name: String,
    /// Chỉ số hàng MethodDef đầu tiên của type (1-based).
    pub method_first: u32,
    /// Số method của type.
    pub method_count: u32,
}

/// Một hàng MethodDef.
#[derive(Debug, Clone)]
pub struct MethodRow {
    pub name: String,
    pub rva: u32,
}

/// Bảng metadata đã parse.
pub struct Index {
    pub types: Vec<TypeRow>,
    pub methods: Vec<MethodRow>,
}

fn heap_string(img: &dyn Image, off: u32, size: u32, index: u32) -> Res<String> {
    if index >= size {
        return Err(E::NoTarget);
    }
    let want = (size - index).min(512) as usize;
    let mut buf = vec![0u8; want];
    img.read_rva(off + index, &mut buf)?;
    let end = buf.iter().position(|&c| c == 0).unwrap_or(want);
    Ok(String::from_utf8_lossy(&buf[..end]).to_string())
}

/// Header của stream `#~`.
struct TablesHeader {
    heap_sizes: u8,
    valid: u64,
    rows: [u32; 64],
    data_off: usize,
    strings_off: u32,
    strings_size: u32,
    tables_off: u32,
}

fn tbl_idx(rows: &[u32; 64], t: usize) -> usize {
    if rows[t] < 0x1_0000 {
        2
    } else {
        4
    }
}

fn coded_idx(rows: &[u32; 64], tables: &[usize], bits: usize) -> usize {
    let max = tables.iter().map(|&t| rows[t]).max().unwrap_or(0);
    if (max as u64) < (1u64 << (16 - bits)) {
        2
    } else {
        4
    }
}

/// Kích thước một hàng của bảng `t` (chỉ các bảng cần tới).
fn row_size(t: usize, th: &TablesHeader) -> usize {
    let rows = &th.rows;
    let str_s = if th.heap_sizes & 1 != 0 { 4 } else { 2 };
    let guid_s = if th.heap_sizes & 2 != 0 { 4 } else { 2 };
    let blob_s = if th.heap_sizes & 4 != 0 { 4 } else { 2 };
    let typedef_or_ref = coded_idx(rows, &[0, 1, 27], 2);
    let resolution_scope = coded_idx(rows, &[0, 26, 35, 1], 2);

    match t {
        0x00 => 2 + str_s + guid_s * 3,                    // Module
        0x01 => resolution_scope + str_s * 2,               // TypeRef
        0x02 => 4 + str_s * 2 + typedef_or_ref + tbl_idx(rows, 4) + tbl_idx(rows, 6), // TypeDef
        0x03 => tbl_idx(rows, 4),                           // FieldPtr
        0x04 => 2 + str_s + blob_s,                         // Field
        0x05 => tbl_idx(rows, 6),                           // MethodPtr
        0x06 => 4 + 2 + 2 + str_s + blob_s + tbl_idx(rows, 8), // MethodDef
        _ => 0,
    }
}

/// Offset (byte) của bảng `t` trong stream `#~`.
fn table_offset(t: usize, th: &TablesHeader) -> usize {
    let mut off = th.data_off;
    for i in 0..t {
        if th.valid & (1u64 << i) != 0 {
            off += row_size(i, th) * th.rows[i] as usize;
        }
    }
    off
}

fn read_tables_header(img: &dyn Image) -> Res<TablesHeader> {
    let mut hdr = vec![0u8; 4096];
    img.read_at(0, &mut hdr)?;

    let cli_rva = data_dir_rva(&hdr, DIR_CLR);
    if cli_rva == 0 {
        return Err(E::Unsupported);
    }
    let mut cli = vec![0u8; 72];
    img.read_rva(cli_rva, &mut cli)?;
    let md_rva = u32_at(&cli, 8);
    let md_size = u32_at(&cli, 12);
    if md_rva == 0 || md_size == 0 {
        return Err(E::Unsupported);
    }

    let root_len = (md_size as usize).min(4096);
    let mut root = vec![0u8; root_len];
    img.read_rva(md_rva, &mut root)?;
    if root.len() < 20 || &root[0..4] != b"BSJB" {
        return Err(E::Unsupported);
    }
    let ver_len = u32_at(&root, 12) as usize;
    let mut off = 16 + ver_len + 2; // + flags
    let n_streams = u16_at(&root, off) as usize;
    off += 2;

    let mut strings_off = 0;
    let mut strings_size = 0;
    let mut tables_off = 0;
    for _ in 0..n_streams {
        if off + 8 > root.len() {
            break;
        }
        let s_off = u32_at(&root, off);
        let s_size = u32_at(&root, off + 4);
        off += 8;
        let start = off;
        while off < root.len() && root[off] != 0 {
            off += 1;
        }
        let name = String::from_utf8_lossy(&root[start..off]).to_string();
        off = (off + 1 + 3) & !3;

        match name.as_str() {
            "#Strings" => {
                strings_off = md_rva + s_off;
                strings_size = s_size;
            }
            "#~" | "#-" => {
                tables_off = md_rva + s_off;
            }
            _ => {}
        }
    }
    if strings_off == 0 || tables_off == 0 {
        return Err(E::Unsupported);
    }

    let mut buf = vec![0u8; 4096];
    img.read_rva(tables_off, &mut buf)?;
    if buf.len() < 24 {
        return Err(E::Unsupported);
    }
    let heap_sizes = buf[6];
    let valid = u64::from_le_bytes(buf[8..16].try_into().unwrap());
    let mut rows = [0u32; 64];
    let mut o = 24;
    for i in 0..64 {
        if valid & (1u64 << i) != 0 {
            if o + 4 > buf.len() {
                return Err(E::Unsupported);
            }
            rows[i] = u32_at(&buf, o);
            o += 4;
        }
    }
    Ok(TablesHeader {
        heap_sizes,
        valid,
        rows,
        data_off: o,
        strings_off,
        strings_size,
        tables_off,
    })
}

/// Parse toàn bộ bảng TypeDef + MethodDef.
pub fn parse_index(img: &dyn Image) -> Res<Index> {
    let th = read_tables_header(img)?;
    if th.valid & (1u64 << 2) == 0 || th.valid & (1u64 << 6) == 0 {
        return Err(E::Unsupported);
    }
    let str_s = if th.heap_sizes & 1 != 0 { 4 } else { 2 };
    let type_row = row_size(0x02, &th);
    let method_row = row_size(0x06, &th);
    let type_off = table_offset(0x02, &th);
    let method_off = table_offset(0x06, &th);
    let type_rows = th.rows[2] as usize;
    let method_rows = th.rows[6] as usize;

    let need = (method_off + method_row * method_rows).max(type_off + type_row * type_rows);
    let mut tbl = vec![0u8; need];
    img.read_rva(th.tables_off, &mut tbl)?;

    let str_idx = |off: usize| -> u32 {
        if str_s == 2 {
            u16_at(&tbl, off) as u32
        } else {
            u32_at(&tbl, off)
        }
    };

    // MethodDef: RVA(4) | ImplFlags(2) | Flags(2) | Name(str) | Sig(blob) | ParamList
    let mut methods = Vec::with_capacity(method_rows);
    for i in 0..method_rows {
        let o = method_off + i * method_row;
        if o + method_row > tbl.len() {
            break;
        }
        methods.push(MethodRow {
            name: heap_string(img, th.strings_off, th.strings_size, str_idx(o + 8))?,
            rva: u32_at(&tbl, o),
        });
    }

    // TypeDef: Flags(4) | Name(str) | Namespace(str) | Extends(coded) | FieldList | MethodList
    let typedef_or_ref = coded_idx(&th.rows, &[0, 1, 27], 2);
    let ml_rel = 4 + str_s * 2 + typedef_or_ref + tbl_idx(&th.rows, 4);
    let mut types = Vec::with_capacity(type_rows);
    for i in 0..type_rows {
        let o = type_off + i * type_row;
        if o + type_row > tbl.len() {
            break;
        }
        let name = heap_string(img, th.strings_off, th.strings_size, str_idx(o + 4))?;
        let ns = heap_string(img, th.strings_off, th.strings_size, str_idx(o + 4 + str_s))?;
        let first = if tbl_idx(&th.rows, 6) == 2 {
            u16_at(&tbl, o + ml_rel) as u32
        } else {
            u32_at(&tbl, o + ml_rel)
        };
        let last = if i + 1 < type_rows {
            let no = type_off + (i + 1) * type_row;
            if tbl_idx(&th.rows, 6) == 2 {
                u16_at(&tbl, no + ml_rel) as u32
            } else {
                u32_at(&tbl, no + ml_rel)
            }
        } else {
            method_rows as u32 + 1
        };
        types.push(TypeRow {
            namespace: ns,
            name,
            method_first: first,
            method_count: last.saturating_sub(first),
        });
    }

    Ok(Index { types, methods })
}

/// Tìm managed method theo tên type + tên method.
pub fn find_method(img: &dyn Image, type_name: &str, method_name: &str) -> Res<ManagedMethod> {
    let idx = parse_index(img)?;
    for t in &idx.types {
        let full = if t.namespace.is_empty() {
            t.name.clone()
        } else {
            format!("{}.{}", t.namespace, t.name)
        };
        if !t.name.eq_ignore_ascii_case(type_name) && !full.eq_ignore_ascii_case(type_name) {
            continue;
        }
        let start = t.method_first.saturating_sub(1) as usize;
        let end = (start + t.method_count as usize).min(idx.methods.len());
        for m in &idx.methods[start..end] {
            if m.name.eq_ignore_ascii_case(method_name) {
                let mut out = ManagedMethod {
                    type_name: full,
                    method_name: m.name.clone(),
                    rva: m.rva,
                    il_rva: 0,
                    il_size: 0,
                    tiny: false,
                    raw_il: false,
                };
                resolve_il(img, &mut out)?;
                return Ok(out);
            }
        }
        return Err(E::NoTarget);
    }
    Err(E::NoTarget)
}

/// Tìm method theo tên method trong mọi type (khi không biết tên type).
pub fn find_method_any_type(img: &dyn Image, method_name: &str) -> Res<ManagedMethod> {
    let idx = parse_index(img)?;
    for t in &idx.types {
        let full = if t.namespace.is_empty() {
            t.name.clone()
        } else {
            format!("{}.{}", t.namespace, t.name)
        };
        let start = t.method_first.saturating_sub(1) as usize;
        let end = (start + t.method_count as usize).min(idx.methods.len());
        for m in &idx.methods[start..end] {
            if m.name.eq_ignore_ascii_case(method_name) {
                let mut out = ManagedMethod {
                    type_name: full,
                    method_name: m.name.clone(),
                    rva: m.rva,
                    il_rva: 0,
                    il_size: 0,
                    tiny: false,
                    raw_il: false,
                };
                resolve_il(img, &mut out)?;
                return Ok(out);
            }
        }
    }
    Err(E::NoTarget)
}

/// Xác định vị trí IL code của method và kích thước.
fn resolve_il(img: &dyn Image, m: &mut ManagedMethod) -> Res<()> {
    if m.rva == 0 {
        return Err(E::NoTarget);
    }
    let mut hdr = vec![0u8; 16];
    img.read_rva(m.rva, &mut hdr)?;
    let first = hdr[0];
    if first & 0x3 == 0x2 {
        // tiny header: 1 byte, code size = first >> 2
        m.tiny = true;
        m.il_rva = m.rva + 1;
        m.il_size = (first >> 2) as u32;
        m.raw_il = false;
    } else if first & 0x3 == 0x3 {
        // fat header: 12 byte, code size ở offset 4
        m.tiny = false;
        let flags = u16_at(&hdr, 0);
        let hdr_size = ((flags >> 12) & 0xF) as u32 * 4;
        m.il_rva = m.rva + hdr_size;
        m.il_size = u32_at(&hdr, 4);
        m.raw_il = false;
    } else {
        // RVA trỏ thẳng vào IL (không có header)
        m.raw_il = true;
        m.tiny = false;
        m.il_rva = m.rva;
        m.il_size = 0;
    }
    Ok(())
}

// ---- Image implementations ---------------------------------------------------

/// PE nạp từ đĩa (dùng để tìm RVA offline và kiểm thử).
pub struct FileImage {
    pub bytes: Vec<u8>,
    pub sections: Vec<Section>,
}

impl FileImage {
    pub fn open(path: &str) -> Res<Self> {
        let bytes = std::fs::read(path).map_err(|_| E::NoTarget)?;
        let sections = parse_sections(&bytes)?;
        Ok(Self { bytes, sections })
    }
}

impl Image for FileImage {
    fn read_rva(&self, rva: u32, buf: &mut [u8]) -> Res<()> {
        for s in &self.sections {
            let start = s.va;
            let end = s.va + s.vsize.max(s.raw_size);
            if rva >= start && rva < end {
                let off = s.raw_ptr as usize + (rva - start) as usize;
                if off + buf.len() > self.bytes.len() {
                    return Err(E::Access);
                }
                buf.copy_from_slice(&self.bytes[off..off + buf.len()]);
                return Ok(());
            }
        }
        Err(E::NoTarget)
    }

    fn read_at(&self, off: u32, buf: &mut [u8]) -> Res<()> {
        let o = off as usize;
        if o + buf.len() > self.bytes.len() {
            return Err(E::Access);
        }
        buf.copy_from_slice(&self.bytes[o..o + buf.len()]);
        Ok(())
    }
}

/// PE trong bộ nhớ tiến trình khác.
pub struct RemoteImage {
    pub handle: HANDLE,
    pub base: u64,
}

impl RemoteImage {
    pub fn new(handle: HANDLE, base: u64) -> Self {
        Self { handle, base }
    }
}

impl Image for RemoteImage {
    fn read_rva(&self, rva: u32, buf: &mut [u8]) -> Res<()> {
        read_abs(self.handle, self.base + rva as u64, buf)
    }

    fn read_at(&self, off: u32, buf: &mut [u8]) -> Res<()> {
        read_abs(self.handle, self.base + off as u64, buf)
    }
}

/// Đọc byte tại địa chỉ tuyệt đối trong tiến trình khác.
pub fn read_abs(handle: HANDLE, addr: u64, buf: &mut [u8]) -> Res<()> {
    let mut read = 0usize;
    unsafe {
        ReadProcessMemory(
            handle,
            addr as *const c_void,
            buf.as_mut_ptr() as *mut c_void,
            buf.len(),
            Some(&mut read),
        )
        .map_err(|_| E::Access)?;
    }
    if read != buf.len() {
        return Err(E::Access);
    }
    Ok(())
}

/// Ghi byte vào tiến trình khác (dùng để patch IL).
pub fn write_abs(handle: HANDLE, addr: u64, data: &[u8]) -> Res<()> {
    let mut written = 0usize;
    unsafe {
        WriteProcessMemory(
            handle,
            addr as *mut c_void,
            data.as_ptr() as *const c_void,
            data.len(),
            Some(&mut written),
        )
        .map_err(|_| E::Access)?;
    }
    if written != data.len() {
        return Err(E::Access);
    }
    Ok(())
}

/// Ghi log tóm tắt (không log nội dung nhạy cảm).
pub fn log_found(m: &ManagedMethod) {
    log::kv(
        "Dotnet",
        "Managed method located",
        "method",
        &format!("{}.{}", m.type_name, m.method_name),
    );
    log::kv("Dotnet", "Method RVA", "rva", &format!("0x{:x}", m.rva));
    log::kv(
        "Dotnet",
        "IL body",
        "il_rva",
        &format!("0x{:x} size={} tiny={} raw={}", m.il_rva, m.il_size, m.tiny, m.raw_il),
    );
}
