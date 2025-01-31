use wasm_bindgen::prelude::*;
use md5::{Md5, Digest};
use sha1::Sha1;
use sha2::Sha256;
use crate::utils::process_lines;

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