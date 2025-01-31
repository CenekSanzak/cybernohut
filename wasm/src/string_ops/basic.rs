use wasm_bindgen::prelude::*;
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