use wasm_bindgen::prelude::*;
use crate::utils::process_lines;
use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
#[wasm_bindgen]
pub fn reverse_string(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| line.chars().rev().collect())]
}

#[wasm_bindgen]
pub fn count_vowels(s: &str) -> Vec<String> {
    vec![s.chars()
        .filter(|c| "aeiou".contains(*c))
        .count()
        .to_string()]
}

#[wasm_bindgen]
pub fn lowercase_string(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| line.to_ascii_lowercase())]
}

#[wasm_bindgen]
pub fn uppercase_string(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| line.to_ascii_uppercase())]
}

#[wasm_bindgen]
pub fn remove_whitespace(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| line.chars().filter(|c| !c.is_whitespace()).collect())]
}

#[wasm_bindgen]
pub fn build_slice_text(config: &str) -> Result<js_sys::Function, JsValue> {
    let config: Value = serde_json::from_str(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;
    
    let start = config.get("start")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
    
    let end = config.get("end")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let closure: Closure<dyn Fn(String) -> Vec<String>> = Closure::new(move |s: String| -> Vec<String> {
        vec![process_lines(&s, |line| {
            let len = line.chars().count() as i32;
            
            let start_idx = match start {
                Some(s) if s < 0 => (len + s).max(0) as usize,
                Some(s) => s.max(0) as usize,
                None => 0,
            };

            let end_idx = match end {
                Some(e) if e < 0 => (len + e).max(0) as usize,
                Some(e) if e == 0 => len as usize,
                Some(e) => e.max(0) as usize,
                None => len as usize,
            };

            line.chars()
                .skip(start_idx)
                .take(end_idx.saturating_sub(start_idx))
                .collect()
        })]
    });

    Ok(closure.into_js_value().unchecked_into::<js_sys::Function>())
} 