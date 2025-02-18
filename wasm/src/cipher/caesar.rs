use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsValue;
use serde_json::Value;
use crate::utils::process_lines;

fn shift_char(c: char, shift: i32) -> char {
    if !c.is_ascii_alphabetic() {
        return c;
    }
    
    let base = if c.is_ascii_uppercase() { 'A' } else { 'a' } as i32;
    let shifted = (((c as i32 - base + shift) % 26 + 26) % 26 + base) as u8;
    shifted as char
}

fn try_all_shifts(text: &str) -> Vec<(i32, String)> {
    (0..26).map(|shift| {
        let shifted = text.chars()
            .map(|c| shift_char(c, shift as i32))
            .collect();
        (shift, shifted)
    }).collect()
}

#[wasm_bindgen]
pub fn build_caesar_cipher(config: &str) -> Result<js_sys::Function, JsValue> {
    let config: Value = serde_json::from_str(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;
    
    let shift = config.get("shift")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
    
    let known_part = config.get("knownPart")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let closure: Closure<dyn Fn(String) -> Vec<String>> = Closure::new(move |s: String| {
        match (shift, &known_part) {
            (Some(n), _) if n != 0 => {
                vec![process_lines(&s, |line| {
                    line.chars()
                        .map(|c| shift_char(c, n))
                        .collect()
                })]
            }
            (None, Some(known)) | (Some(0), Some(known)) => {
                vec![process_lines(&s, |line| {
                    try_all_shifts(line)
                        .into_iter()
                        .filter(|(_, text)| text.to_lowercase().contains(known))
                        .map(|(shift, text)| format!("Shift {}: {}", shift, text))
                        .collect::<Vec<_>>()
                        .join("\n")
                })]
            }
            _ => {
                vec![process_lines(&s, |line| {
                    try_all_shifts(line)
                        .into_iter()
                        .map(|(shift, text)| format!("Shift {}: {}", shift, text))
                        .collect::<Vec<_>>()
                        .join("\n")
                })]
            }
        }
    });

    Ok(closure.into_js_value().unchecked_into::<js_sys::Function>())
} 