use wasm_bindgen::prelude::*;

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