use wasm_bindgen::prelude::*;
use base64::{Engine as _, engine::general_purpose};
use data_encoding::{HEXLOWER as BASE16, BASE32};
use base85::{encode, decode};

#[wasm_bindgen]
pub fn reverse_string(s: &str) -> String {
    s.chars().rev().collect()
}

#[wasm_bindgen]
pub fn count_vowels(s: &str) -> u32 {
    s.chars()
        .filter(|c| "aeiou".contains(*c))
        .count() as u32
}

#[wasm_bindgen]
pub fn lowercase_string(s: &str) -> String {
    s.to_ascii_lowercase()
}

#[wasm_bindgen]
pub fn uppercase_string(s: &str) -> String {
    s.to_ascii_uppercase()
}

#[wasm_bindgen]
pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace() || *c == '\n').collect()
}

#[wasm_bindgen]
pub fn encode_base64(s: &str) -> String {
    general_purpose::STANDARD.encode(s.as_bytes())
}

#[wasm_bindgen]
pub fn decode_base64(s: &str) -> String {
    match general_purpose::STANDARD.decode(s) {
        Ok(decoded) => String::from_utf8_lossy(&decoded).into_owned(),
        Err(_) => String::new()
    }
}

#[wasm_bindgen]
pub fn encode_base16(s: &str) -> String {
    BASE16.encode(s.as_bytes())
}

#[wasm_bindgen]
pub fn decode_base16(s: &str) -> String {
    BASE16.decode(s.as_bytes()).map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
}

#[wasm_bindgen]
pub fn encode_base32(s: &str) -> String {
    BASE32.encode(s.as_bytes())
}

#[wasm_bindgen]
pub fn decode_base32(s: &str) -> String {
    BASE32.decode(s.as_bytes()).map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
}

#[wasm_bindgen]
pub fn encode_base85(s: &str) -> String {
    encode(s.as_bytes())
}

#[wasm_bindgen]
pub fn decode_base85(s: &str) -> String {
    decode(s).map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
}