use wasm_bindgen::prelude::*;
use base64::{Engine as _, engine::general_purpose};
use data_encoding::{HEXLOWER as BASE16, BASE32};
use base85::{encode, decode};
use crate::utils::process_lines;
use md5::{Md5, Digest as Md5Digest};
use sha1::{Sha1, Digest as Sha1Digest};
use sha2::{Sha256, Digest as Sha256Digest};

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

#[wasm_bindgen]
pub fn calculate_md5(s: &str) -> String {
    process_lines(s, |line| {
        let mut hasher = Md5::new();
        hasher.update(line.as_bytes());
        format!("{:x}", hasher.finalize())
    })
}

#[wasm_bindgen]
pub fn calculate_sha1(s: &str) -> String {
    process_lines(s, |line| {
        let mut hasher = Sha1::new();
        hasher.update(line.as_bytes());
        format!("{:x}", hasher.finalize())
    })
}

#[wasm_bindgen]
pub fn calculate_sha256(s: &str) -> String {
    process_lines(s, |line| {
        let mut hasher = Sha256::new();
        hasher.update(line.as_bytes());
        format!("{:x}", hasher.finalize())
    })
}