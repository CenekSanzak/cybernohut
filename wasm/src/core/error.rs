use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::value::ValueType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    UnknownOp,
    InvalidConfig,
    InvalidInput,
    TypeMismatch,
    InvalidHandle,
}

/// The single error shape that crosses the wasm boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct OpError {
    pub code: ErrorCode,
    pub message: String,
}

impl OpError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        OpError { code, message: message.into() }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidConfig, message)
    }

    pub fn unknown_op(id: &str) -> Self {
        Self::new(ErrorCode::UnknownOp, format!("Unknown operation \"{id}\""))
    }

    pub fn type_mismatch(index: usize, expected: ValueType, got: ValueType) -> Self {
        Self::new(
            ErrorCode::TypeMismatch,
            format!("Input {index}: expected {expected:?}, got {got:?}"),
        )
    }

    pub fn input_count(expected: usize, got: usize) -> Self {
        Self::new(ErrorCode::TypeMismatch, format!("Expected {expected} inputs, got {got}"))
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OpError {}
