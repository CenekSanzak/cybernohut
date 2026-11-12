//! Keys as the user types them: literal text or hexadecimal bytes.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::OpError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum KeyFormat {
    #[default]
    Text,
    Hex,
}

impl KeyFormat {
    pub fn options() -> Vec<String> {
        vec!["text".into(), "hex".into()]
    }
}

/// Hexadecimal with optional whitespace between bytes. `what` names the field in the
/// error message.
pub fn parse_hex(value: &str, what: &str) -> Result<Vec<u8>, OpError> {
    let compact: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    hex_simd::decode_to_vec(compact.as_bytes())
        .map_err(|_| OpError::invalid_config(format!("The {what} is not valid hexadecimal")))
}

/// The bytes of a key. An empty key is an error: no operation that takes one has a
/// meaningful result without it.
pub fn parse_key(key: &str, format: KeyFormat) -> Result<Vec<u8>, OpError> {
    let bytes = match format {
        KeyFormat::Text => key.as_bytes().to_vec(),
        KeyFormat::Hex => parse_hex(key, "key")?,
    };
    if bytes.is_empty() {
        return Err(OpError::invalid_config("Enter a key"));
    }
    Ok(bytes)
}
