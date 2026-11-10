//! Venice AI — client OpenAI-compatible chạy trên WinHTTP.
//!
//! Không thêm crate: `windows` + `serde_json` là đủ. Khoá đọc từ `.env`
//! (`VENICE_API_KEY`), không bao giờ hardcode và không bao giờ ghi log.
//!
//! Mọi ngưỡng thời gian lấy từ `settings::TUNING`.

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
use crate::{dotenv, log, E, Res};

pub const DEFAULT_BASE_URL: &str = "https://api.venice.ai/api/v1";
pub const DEFAULT_MODEL: &str = "zai-org-glm-5-2";

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

#[derive(Debug, Clone, PartialEq)]
struct Endpoint {
    secure: bool,
    host: String,
    port: u16,
    /// Không có `/` cuối, ví dụ `/api/v1`.
    path: String,
}

fn parse_base(base: &str) -> Res<Endpoint> {
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
    if authority.is_empty() {
        return Err(E::Unsupported);
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| E::Unsupported)?),
        None => (authority, if secure { 443 } else { 80 }),
    };
    if host.is_empty() {
        return Err(E::Unsupported);
    }
    Ok(Endpoint { secure, host: host.to_string(), port, path })
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
        let api_key = dotenv::var("VENICE_API_KEY").ok_or(E::NoTarget)?;
        let base_url = dotenv::var("VENICE_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let model = dotenv::var("VENICE_MODEL").unwrap_or_else(|| DEFAULT_MODEL.to_string());
        parse_base(&base_url)?;
        Ok(Self { api_key, base_url, model, timeout_ms: TUNING.http_timeout_ms })
    }

    /// `POST {base}/chat/completions`, trả `choices[0].message.content`.
    pub fn chat(&self, system: Option<&str>, user: &str) -> Res<String> {
        let mut messages = Vec::with_capacity(2);
        if let Some(s) = system.filter(|s| !s.trim().is_empty()) {
            messages.push(json!({ "role": "system", "content": s }));
        }
        messages.push(json!({ "role": "user", "content": user }));
        let body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.2,
        })
        .to_string();

        let raw = self.post("/chat/completions", &body)?;
        extract_content(&raw)
    }

    fn post(&self, suffix: &str, body: &str) -> Res<String> {
        let ep = parse_base(&self.base_url)?;
        let path = format!("{}{}", ep.path, suffix);

        unsafe {
            let agent = to_wide("Hepi/0.20.8");
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

            let host = to_wide(&ep.host);
            let conn = Handle(WinHttpConnect(session.0, PCWSTR(host.as_ptr()), ep.port, 0));
            if conn.0.is_null() {
                return Err(E::Access);
            }

            let verb = to_wide("POST");
            let obj = to_wide(&path);
            let flags = if ep.secure {
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

            let headers = format!(
                "Content-Type: application/json\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\n",
                self.api_key
            );
            let hw = to_wide(&headers);
            WinHttpAddRequestHeaders(
                req.0,
                &hw[..hw.len() - 1],
                WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE,
            )
            .map_err(|_| E::Access)?;

            let bytes = body.as_bytes();
            WinHttpSendRequest(
                req.0,
                None,
                Some(bytes.as_ptr() as *const c_void),
                bytes.len() as u32,
                bytes.len() as u32,
                0,
            )
            .map_err(|_| E::Access)?;
            WinHttpReceiveResponse(req.0, std::ptr::null_mut()).map_err(|_| E::Access)?;

            let mut status = 0u32;
            let mut len = std::mem::size_of::<u32>() as u32;
            let _ = WinHttpQueryHeaders(
                req.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(&mut status as *mut _ as *mut c_void),
                &mut len,
                std::ptr::null_mut(),
            );

            let mut raw: Vec<u8> = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let mut read = 0u32;
                WinHttpReadData(
                    req.0,
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
                log::kv("Venice", "HTTP error", "status", &status.to_string());
                log::kv("Venice", "response", "body", &truncate(&text, 300));
                return Err(E::Access);
            }
            log::kv("Venice", "chat ok", "status", &status.to_string());
            log::kv("Venice", "bytes", "n", &raw.len().to_string());
            Ok(text)
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn extract_content(raw: &str) -> Res<String> {
    let v: Value = serde_json::from_str(raw).map_err(|_| E::State)?;
    if let Some(c) = v["choices"][0]["message"]["content"].as_str() {
        if !c.trim().is_empty() {
            return Ok(c.to_string());
        }
    }
    log::kv("Venice", "no content", "body", &truncate(raw, 300));
    Err(E::State)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_base() {
        let ep = parse_base(DEFAULT_BASE_URL).unwrap();
        assert!(ep.secure);
        assert_eq!(ep.host, "api.venice.ai");
        assert_eq!(ep.port, 443);
        assert_eq!(ep.path, "/api/v1");
    }

    #[test]
    fn parses_plain_host_and_port() {
        let ep = parse_base("http://127.0.0.1:8080").unwrap();
        assert!(!ep.secure);
        assert_eq!(ep.host, "127.0.0.1");
        assert_eq!(ep.port, 8080);
        assert_eq!(ep.path, "");
    }

    #[test]
    fn rejects_bad_base() {
        assert_eq!(parse_base("api.venice.ai"), Err(E::Unsupported));
        assert_eq!(parse_base("https://"), Err(E::Unsupported));
        assert_eq!(parse_base("https://host:bad"), Err(E::Unsupported));
    }

    #[test]
    fn extracts_content() {
        let raw = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        assert_eq!(extract_content(raw), Ok("ok".to_string()));
        assert_eq!(extract_content("{}"), Err(E::State));
        assert_eq!(extract_content("not json"), Err(E::State));
    }
}
