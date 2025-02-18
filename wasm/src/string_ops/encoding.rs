use wasm_bindgen::prelude::*;
use base64::{Engine as _, engine::general_purpose};
use data_encoding::{HEXLOWER as BASE16, BASE32, HEXUPPER};
use base85::{encode, decode};
use crate::utils::process_lines;
use serde_json::Value;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
pub fn encode_base16(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| BASE16.encode(line.as_bytes()))]
}

#[wasm_bindgen]
pub fn build_base16_encoder(config: &str) -> Result<js_sys::Function, JsValue> {
    let config: Value = serde_json::from_str(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config JSON: {}", e)))?;
    
    let uppercase = config.get("uppercase")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let encoder = if uppercase { HEXUPPER } else { BASE16 };
    
    let closure: Closure<dyn Fn(String) -> Vec<String>> = Closure::new(move |s: String| {
        vec![process_lines(&s, |line| encoder.encode(line.as_bytes()))]
    });
    
    Ok(closure.into_js_value().unchecked_into::<js_sys::Function>())
}

#[wasm_bindgen]
pub fn decode_base16(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| {
        BASE16.decode(line.as_bytes())
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })]
}

#[wasm_bindgen]
pub fn encode_base32(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| BASE32.encode(line.as_bytes()))]
}

#[wasm_bindgen]
pub fn decode_base32(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| {
        BASE32.decode(line.as_bytes())
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })]
}

#[wasm_bindgen]
pub fn encode_base85(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| encode(line.as_bytes()))]
}

#[wasm_bindgen]
pub fn decode_base85(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| {
        decode(line).map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })]
}

#[wasm_bindgen]
pub fn encode_base64_standard(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| general_purpose::STANDARD.encode(line.as_bytes()))]
}

#[wasm_bindgen]
pub fn decode_base64_standard(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| {
        general_purpose::STANDARD.decode(line)
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })]
}

#[wasm_bindgen]
pub fn encode_base64_url(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| general_purpose::URL_SAFE.encode(line.as_bytes()))]
}

#[wasm_bindgen]
pub fn decode_base64_url(s: &str) -> Vec<String> {
    vec![process_lines(s, |line| {
        general_purpose::URL_SAFE.decode(line)
            .map_or(String::new(), |v| String::from_utf8_lossy(&v).into_owned())
    })]
} 