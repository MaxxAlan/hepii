//! Gọi thật một lượt `/chat/completions` để xác nhận khoá + model.
//!
//! `cargo run -p hepi-core --example venice_chat`

use hepi_core::venice::Client;

fn main() {
    let client = match Client::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("client init failed: {}", e.code());
            std::process::exit(2);
        }
    };
    println!("model: {}", client.model);
    match client.chat(
        Some("You are a terse assistant."),
        "Reply with exactly: VENICE_OK",
    ) {
        Ok(text) => println!("content: {}", text),
        Err(e) => {
            eprintln!("chat failed: {}", e.code());
            std::process::exit(1);
        }
    }
}
