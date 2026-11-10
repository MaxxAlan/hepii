//! R9: một dòng ASCII, tiền tố module, không i18n.

pub fn line(module: &str, msg: &str) {
    println!("[{}] {}", module, msg);
}

pub fn err(module: &str, code: crate::E, detail: &str) {
    eprintln!("[{}] {} {}", module, code.code(), detail);
}

pub fn kv(module: &str, msg: &str, k: &str, v: &str) {
    println!("[{}] {} {}={}", module, msg, k, v);
}
