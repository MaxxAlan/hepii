//! Gemini API — client OpenAI-compatible trên WinHTTP (không thêm crate).
//!
//! Endpoint: `POST {base}/models/{model}:generateContent?key=API_KEY`
//! Body: `{"contents":[{"parts":[{"text":"..."}]}],"systemInstruction":{...}}`
//! Response: `candidates[0].content.parts[*].text`
//!
//! Khoá đọc từ `.env` (`GEMINI_API_KEY`), không log giá trị, không ghi log khoá.

use std::ffi::c_void;

use serde_json::{json, Value};
use windows::core::PCWSTR;
use windows::Win32::Networking::WinHttp::{
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_ADDREQ_FLAG_ADD,
    WINHTTP_ADDREQ_FLAG_REPLACE, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
};

use crate::settings::TUNING;
use crate::util::to_wide;
use crate::{log, E, Res};

pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";

/// Một handle WinHTTP; đóng khi rơi khỏi scope, kể cả đường lỗi.
struct Handle(*mut c_void);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

/// Tách `scheme://host[:port]/path` thành (secure, host, port, path).
fn parse_base(base: &str) -> Res<(bool, String, u16, String)> {
    let base = base.trim().trim_end_matches('/');
    let (secure, rest) = if let Some(r) = base.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = base.strip_prefix("http://") {
        (false, r)
    } else {
        return Err(E::Unsupported);
    };
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, format!("/{p}")),
        None => (rest, String::new()),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, p.parse().map_err(|_| E::Unsupported)?),
        None => (authority, if secure { 443 } else { 80 }),
    };
    if host.is_empty() {
        return Err(E::Unsupported);
    }
    Ok((secure, host.to_string(), port, path))
}

pub struct Client {
    api_key: String,
    base_url: String,
    pub model: String,
    timeout_ms: u32,
}

impl Client {
    /// Khoá thiếu ⇒ `E_NO_TARGET`; không đoán, không dùng khoá rỗng.
    pub fn from_env() -> Res<Self> {
        let api_key = crate::dotenv::var("GEMINI_API_KEY").ok_or(E::NoTarget)?;
        let base_url = crate::dotenv::var("GEMINI_BASE_URL")
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let model =
            crate::dotenv::var("GEMINI_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());
        parse_base(&base_url)?;
        Ok(Self {
            api_key,
            base_url,
            model,
            timeout_ms: TUNING.http_timeout_ms,
        })
    }

    /// Có khoá trong `.env` không (không gọi mạng).
    pub fn key_present() -> bool {
        crate::dotenv::var("GEMINI_API_KEY").is_some()
    }

    /// `POST {base}/models/{model}:generateContent`.
    pub fn chat(&self, system: Option<&str>, user: &str) -> Res<String> {
        let mut parts = vec![json!({ "text": user })];
        if let Some(s) = system.filter(|s| !s.trim().is_empty()) {
            parts.insert(0, json!({ "text": s }));
        }
        self.chat_parts(system, parts)
    }

    /// `POST` với `parts` tuỳ biến (dùng cho vision: `inline_data`).
    pub fn chat_parts(&self, system: Option<&str>, mut parts: Vec<serde_json::Value>) -> Res<String> {
        if let Some(s) = system.filter(|s| !s.trim().is_empty()) {
            parts.insert(0, json!({ "text": s }));
        }
        let body = json!({
            "contents": [{ "role": "user", "parts": parts }],
            "generationConfig": { "temperature": 0.2 },
        })
        .to_string();

        let (secure, host, port, path) = parse_base(&self.base_url)?;
        let suffix = format!("/models/{}:generateContent", self.model);
        let raw = self.post(&path, &suffix, &body, secure, &host, port)?;
        extract_text(&raw)
    }

    /// Gửi ảnh (PNG base64) kèm câu hỏi — Gemini vision.
    pub fn chat_image(&self, system: Option<&str>, user: &str, png_b64: &str) -> Res<String> {
        let parts = vec![
            json!({ "text": user }),
            json!({ "inline_data": { "mime_type": "image/png", "data": png_b64 } }),
        ];
        self.chat_parts(system, parts)
    }

    fn post(
        &self,
        base_path: &str,
        suffix: &str,
        body: &str,
        secure: bool,
        host: &str,
        port: u16,
    ) -> Res<String> {
        unsafe {
            let agent = to_wide(concat!("Hepi/", env!("CARGO_PKG_VERSION")));
            let session = Handle(WinHttpOpen(
                PCWSTR(agent.as_ptr()),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            ));
            if session.0.is_null() {
                return Err(E::Access);
            }
            let t = self.timeout_ms as i32;
            WinHttpSetTimeouts(
                session.0,
                t,
                TUNING.http_connect_timeout_ms as i32,
                t,
                t,
            )
            .map_err(|_| E::Access)?;

            let host_w = to_wide(host);
            let conn = Handle(WinHttpConnect(session.0, PCWSTR(host_w.as_ptr()), port, 0));
            if conn.0.is_null() {
                return Err(E::Access);
            }

            let verb = to_wide("POST");
            let obj = to_wide(&format!("{base_path}{suffix}"));
            let flags = if secure {
                WINHTTP_FLAG_SECURE
            } else {
                WINHTTP_OPEN_REQUEST_FLAGS(0)
            };
            let req = Handle(WinHttpOpenRequest(
                conn.0,
                PCWSTR(verb.as_ptr()),
                PCWSTR(obj.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                flags,
            ));
            if req.0.is_null() {
                return Err(E::Access);
            }

            // Key trong query string (Gemini dùng ?key=), không phải header.
            let obj_keyed = to_wide(&format!("{base_path}{suffix}?key={}", self.api_key));
            let verb2 = to_wide("POST");
            let req2 = Handle(WinHttpOpenRequest(
                conn.0,
                PCWSTR(verb2.as_ptr()),
                PCWSTR(obj_keyed.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                flags,
            ));
            if req2.0.is_null() {
                return Err(E::Access);
            }

            let headers = format!("Content-Type: application/json\r\nAccept: application/json\r\n");
            let hw = to_wide(&headers);
            WinHttpAddRequestHeaders(
                req2.0,
                &hw[..hw.len() - 1],
                WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
            )
            .map_err(|_| E::Access)?;

            let bytes = body.as_bytes();
            WinHttpSendRequest(
                req2.0,
                None,
                Some(bytes.as_ptr() as *const c_void),
                bytes.len() as u32,
                bytes.len() as u32,
                0,
            )
            .map_err(|_| E::Access)?;
            WinHttpReceiveResponse(req2.0, std::ptr::null_mut()).map_err(|_| E::Access)?;

            let mut status = 0u32;
            let mut len = std::mem::size_of::<u32>() as u32;
            let _ = WinHttpQueryHeaders(
                req2.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(&mut status as *mut _ as *mut c_void),
                &mut len as *mut u32,
                std::ptr::null_mut(),
            );

            let mut raw: Vec<u8> = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let mut read = 0u32;
                WinHttpReadData(
                    req2.0,
                    buf.as_mut_ptr() as *mut c_void,
                    buf.len() as u32,
                    &mut read,
                )
                .map_err(|_| E::Access)?;
                if read == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..read as usize]);
            }
            let text = String::from_utf8_lossy(&raw).to_string();

            if !(200..300).contains(&status) {
                log::kv("Gemini", "HTTP error", "status", &status.to_string());
                log::kv("Gemini", "response", "body", &truncate(&text, 300));
                return Err(E::Access);
            }
            log::kv("Gemini", "chat ok", "status", &status.to_string());
            Ok(text)
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Gom text từ `candidates[0].content.parts[*].text`.
fn extract_text(raw: &str) -> Res<String> {
    let v: Value = serde_json::from_str(raw).map_err(|_| E::State)?;
    let parts = v["candidates"][0]["content"]["parts"]
        .as_array()
        .ok_or(E::State)?;
    let mut out = String::new();
    for p in parts {
        if let Some(t) = p["text"].as_str() {
            out.push_str(t);
        }
    }
    if out.trim().is_empty() {
        log::kv("Gemini", "no content", "body", &truncate(raw, 300));
        return Err(E::State);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_base() {
        let (s, h, p, path) = parse_base(DEFAULT_BASE_URL).unwrap();
        assert!(s);
        assert_eq!(h, "generativelanguage.googleapis.com");
        assert_eq!(p, 443);
        assert_eq!(path, "/v1beta");
    }

    #[test]
    fn rejects_bad_base() {
        assert!(parse_base("generativelanguage.googleapis.com").is_err());
        assert!(parse_base("https://").is_err());
    }

    #[test]
    fn extracts_parts() {
        let raw = r#"{"candidates":[{"content":{"parts":[{"text":"a"},{"text":"b"}]}}]}"#;
        assert_eq!(extract_text(raw).unwrap(), "ab");
        assert_eq!(extract_text("{}"), Err(E::State));
    }
}
