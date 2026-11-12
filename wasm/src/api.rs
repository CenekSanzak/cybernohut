//! The wasm boundary.
//!
//! Two kinds of data cross it, by two different routes:
//!
//! - Control data (the catalog, configs, errors) is small and structured, so it goes
//!   through serde and gets generated TypeScript types.
//! - Payloads stay in wasm memory. JavaScript holds a numeric handle and copies a value
//!   out only when it has to show it. Chained operations never copy across the boundary.

use std::cell::RefCell;
use std::rc::Rc;

use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::core::{ErrorCode, OpError, OpMeta, Value, ValueType};
use crate::ops;

#[derive(Default)]
struct Store {
    slots: Vec<Option<Rc<Value>>>,
    free: Vec<u32>,
}

impl Store {
    fn insert(&mut self, value: Value) -> u32 {
        let value = Some(Rc::new(value));
        match self.free.pop() {
            Some(handle) => {
                self.slots[handle as usize] = value;
                handle
            }
            None => {
                self.slots.push(value);
                (self.slots.len() - 1) as u32
            }
        }
    }

    fn get(&self, handle: u32) -> Result<Rc<Value>, OpError> {
        self.slots.get(handle as usize).and_then(Clone::clone).ok_or_else(|| {
            OpError::new(ErrorCode::InvalidHandle, format!("No value behind handle {handle}"))
        })
    }

    fn remove(&mut self, handle: u32) {
        if let Some(slot) = self.slots.get_mut(handle as usize) {
            if slot.take().is_some() {
                self.free.push(handle);
            }
        }
    }
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::default());
}

fn insert(value: Value) -> u32 {
    STORE.with(|store| store.borrow_mut().insert(value))
}

fn get(handle: u32) -> Result<Rc<Value>, OpError> {
    STORE.with(|store| store.borrow().get(handle))
}

/// Errors are thrown to JavaScript as plain `OpError` objects.
fn throw(error: OpError) -> JsValue {
    match Ts::from_rust(&error) {
        Ok(ts) => ts.into(),
        Err(_) => JsValue::from_str(&error.message),
    }
}

fn run_handles(op_id: &str, config: &serde_json::Value, inputs: &[u32]) -> Result<Vec<u32>, OpError> {
    let entry = ops::find(op_id).ok_or_else(|| OpError::unknown_op(op_id))?;
    let values = inputs.iter().map(|handle| get(*handle)).collect::<Result<Vec<_>, _>>()?;
    let refs: Vec<&Value> = values.iter().map(Rc::as_ref).collect();
    let outputs = (entry.run)(config, &refs)?;
    Ok(outputs.into_iter().map(insert).collect())
}

fn config_from_js(config: JsValue) -> Result<serde_json::Value, OpError> {
    if config.is_undefined() || config.is_null() {
        return Ok(serde_json::Value::Null);
    }
    let json = js_sys::JSON::stringify(&config)
        .ok()
        .and_then(|text| text.as_string())
        .ok_or_else(|| OpError::invalid_config("Config is not serializable"))?;
    serde_json::from_str(&json).map_err(|e| OpError::invalid_config(e.to_string()))
}

/// Every available operation, described for the UI.
#[wasm_bindgen]
pub fn catalog() -> Result<Vec<Ts<OpMeta>>, JsValue> {
    ops::registry()
        .iter()
        .map(|entry| Ts::from_rust(&(entry.meta)()).map_err(|e| JsValue::from_str(&e.to_string())))
        .collect()
}

/// Runs one operation on stored values and stores its outputs. Throws an `OpError`.
#[wasm_bindgen]
pub fn run(op_id: &str, config: JsValue, inputs: &[u32]) -> Result<Vec<u32>, JsValue> {
    let config = config_from_js(config).map_err(throw)?;
    run_handles(op_id, &config, inputs).map_err(throw)
}

#[wasm_bindgen]
pub fn put_text(text: String) -> u32 {
    insert(Value::Text(text))
}

#[wasm_bindgen]
pub fn put_bytes(bytes: Vec<u8>) -> u32 {
    insert(Value::Bytes(bytes))
}

#[wasm_bindgen]
pub fn put_number(number: f64) -> u32 {
    insert(Value::Number(number))
}

#[wasm_bindgen]
pub fn put_text_list(items: Vec<String>) -> u32 {
    insert(Value::TextList(items))
}

#[wasm_bindgen]
pub fn put_number_list(items: Vec<f64>) -> u32 {
    insert(Value::NumberList(items))
}

#[wasm_bindgen]
pub fn value_type(handle: u32) -> Result<Ts<ValueType>, JsValue> {
    let value = get(handle).map_err(throw)?;
    Ts::from_rust(&value.value_type()).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn wrong_type(expected: ValueType, value: &Value) -> JsValue {
    throw(OpError::type_mismatch(0, expected, value.value_type()))
}

#[wasm_bindgen]
pub fn get_text(handle: u32) -> Result<String, JsValue> {
    match get(handle).map_err(throw)?.as_ref() {
        Value::Text(text) => Ok(text.clone()),
        other => Err(wrong_type(ValueType::Text, other)),
    }
}

#[wasm_bindgen]
pub fn get_bytes(handle: u32) -> Result<Vec<u8>, JsValue> {
    match get(handle).map_err(throw)?.as_ref() {
        Value::Bytes(bytes) => Ok(bytes.clone()),
        other => Err(wrong_type(ValueType::Bytes, other)),
    }
}

#[wasm_bindgen]
pub fn get_number(handle: u32) -> Result<f64, JsValue> {
    match get(handle).map_err(throw)?.as_ref() {
        Value::Number(number) => Ok(*number),
        other => Err(wrong_type(ValueType::Number, other)),
    }
}

#[wasm_bindgen]
pub fn get_text_list(handle: u32) -> Result<Vec<String>, JsValue> {
    match get(handle).map_err(throw)?.as_ref() {
        Value::TextList(items) => Ok(items.clone()),
        other => Err(wrong_type(ValueType::TextList, other)),
    }
}

#[wasm_bindgen]
pub fn get_number_list(handle: u32) -> Result<Vec<f64>, JsValue> {
    match get(handle).map_err(throw)?.as_ref() {
        Value::NumberList(items) => Ok(items.clone()),
        other => Err(wrong_type(ValueType::NumberList, other)),
    }
}

/// Releases a stored value. Releasing an unknown handle is a no-op.
#[wasm_bindgen]
pub fn release(handle: u32) {
    STORE.with(|store| store.borrow_mut().remove(handle));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_chain_without_leaving_the_store() {
        let input = insert(Value::Text("hello".into()));
        let encoded = run_handles("base64-standard-encode", &serde_json::Value::Null, &[input]).unwrap();
        let decoded = run_handles("base64-standard-decode", &serde_json::Value::Null, &encoded).unwrap();
        assert_eq!(*get(decoded[0]).unwrap(), Value::Bytes(b"hello".to_vec()));
    }

    #[test]
    fn released_handles_are_invalid_and_reused() {
        let handle = insert(Value::Number(1.0));
        release(handle);
        assert_eq!(get(handle).unwrap_err().code, ErrorCode::InvalidHandle);
        assert_eq!(insert(Value::Number(2.0)), handle);
    }

    #[test]
    fn unknown_operation() {
        let error = run_handles("nope", &serde_json::Value::Null, &[]).unwrap_err();
        assert_eq!(error.code, ErrorCode::UnknownOp);
    }
}
