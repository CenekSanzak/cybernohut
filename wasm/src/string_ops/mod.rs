use wasm_bindgen::prelude::*;
use base64::{Engine as _, engine::general_purpose};
use data_encoding::{HEXLOWER as BASE16, BASE32};
use base85::{encode, decode};
use crate::utils::process_lines;

#[wasm_bindgen]
pub fn reverse_string(s: &str) -> String {
    process_lines(s, |line| line.chars().rev().collect())
}

#[wasm_bindgen]
pub fn count_vowels(s: &str) -> u32 {
    s.chars()
        .filter(|c| "aeiou".contains(*c))
        .count() as u32
}

#[wasm_bindgen]
pub fn lowercase_string(s: &str) -> String {
    process_lines(s, |line| line.to_ascii_lowercase())
}

#[wasm_bindgen]
pub fn uppercase_string(s: &str) -> String {
    process_lines(s, |line| line.to_ascii_uppercase())
}

#[wasm_bindgen]
pub fn remove_whitespace(s: &str) -> String {
    process_lines(s, |line| line.chars().filter(|c| !c.is_whitespace()).collect())
}

#[wasm_bindgen]
pub fn encode_base16(s: &str) -> String {
    process_lines(s, |line| BASE16.encode(line.as_bytes()))
}

#[wasm_bindgen]
pub fn decode_base16(s: &str) -> String {
    process_lines(s, |line| {
        BASE16.decode(line.as_bytes())
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })
}

#[wasm_bindgen]
pub fn encode_base32(s: &str) -> String {
    process_lines(s, |line| BASE32.encode(line.as_bytes()))
}

#[wasm_bindgen]
pub fn decode_base32(s: &str) -> String {
    process_lines(s, |line| {
        BASE32.decode(line.as_bytes())
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })
}

#[wasm_bindgen]
pub fn encode_base85(s: &str) -> String {
    process_lines(s, |line| encode(line.as_bytes()))
}

#[wasm_bindgen]
pub fn decode_base85(s: &str) -> String {
    process_lines(s, |line| {
        decode(line).map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })
}

#[wasm_bindgen]
pub fn encode_base64_standard(s: &str) -> String {
    process_lines(s, |line| general_purpose::STANDARD.encode(line.as_bytes()))
}

#[wasm_bindgen]
pub fn decode_base64_standard(s: &str) -> String {
    process_lines(s, |line| {
        general_purpose::STANDARD.decode(line)
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })
}

#[wasm_bindgen]
pub fn encode_base64_url(s: &str) -> String {
    process_lines(s, |line| general_purpose::URL_SAFE.encode(line.as_bytes()))
}

#[wasm_bindgen]
pub fn decode_base64_url(s: &str) -> String {
    process_lines(s, |line| {
        general_purpose::URL_SAFE.decode(line)
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })
}