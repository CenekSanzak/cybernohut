//! The values that flow along graph edges, and the typed views operations take of them.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::error::OpError;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bytes(Vec<u8>),
    Text(String),
    Number(f64),
    TextList(Vec<String>),
    NumberList(Vec<f64>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Bytes,
    Text,
    Number,
    TextList,
    NumberList,
}

impl ValueType {
    /// Whether a port of this type can be fed a value of type `from`.
    ///
    /// The only implicit conversion is text to bytes (its UTF-8 encoding), because it
    /// cannot fail or lose information. Everything else needs an explicit operation.
    pub fn accepts(self, from: ValueType) -> bool {
        self == from || (self == ValueType::Bytes && from == ValueType::Text)
    }
}

impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Bytes(_) => ValueType::Bytes,
            Value::Text(_) => ValueType::Text,
            Value::Number(_) => ValueType::Number,
            Value::TextList(_) => ValueType::TextList,
            Value::NumberList(_) => ValueType::NumberList,
        }
    }
}

/// A borrowed, typed view of a [`Value`], used as an operation input.
pub trait FromValue<'a>: Sized {
    const TYPE: ValueType;
    fn from_value(value: &'a Value) -> Option<Self>;
}

impl<'a> FromValue<'a> for &'a [u8] {
    const TYPE: ValueType = ValueType::Bytes;
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::Bytes(bytes) => Some(bytes),
            Value::Text(text) => Some(text.as_bytes()),
            _ => None,
        }
    }
}

impl<'a> FromValue<'a> for &'a str {
    const TYPE: ValueType = ValueType::Text;
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }
}

impl<'a> FromValue<'a> for f64 {
    const TYPE: ValueType = ValueType::Number;
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }
}

impl<'a> FromValue<'a> for &'a [String] {
    const TYPE: ValueType = ValueType::TextList;
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::TextList(items) => Some(items),
            _ => None,
        }
    }
}

impl<'a> FromValue<'a> for &'a [f64] {
    const TYPE: ValueType = ValueType::NumberList;
    fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::NumberList(items) => Some(items),
            _ => None,
        }
    }
}

/// An owned value an operation can produce.
pub trait IntoValue {
    const TYPE: ValueType;
    fn into_value(self) -> Value;
}

macro_rules! into_value {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
        impl IntoValue for $ty {
            const TYPE: ValueType = ValueType::$variant;
            fn into_value(self) -> Value {
                Value::$variant(self)
            }
        }
    )*};
}

into_value! {
    Vec<u8> => Bytes,
    String => Text,
    f64 => Number,
    Vec<String> => TextList,
    Vec<f64> => NumberList,
}

/// The full input list of an operation: a single typed view, or a tuple of them.
pub trait Inputs<'a>: Sized {
    fn types() -> Vec<ValueType>;
    fn extract(values: &[&'a Value]) -> Result<Self, OpError>;
}

fn extract_one<'a, T: FromValue<'a>>(values: &[&'a Value], index: usize) -> Result<T, OpError> {
    let value = values[index];
    T::from_value(value).ok_or_else(|| OpError::type_mismatch(index, T::TYPE, value.value_type()))
}

fn check_count(values: &[&Value], expected: usize) -> Result<(), OpError> {
    if values.len() == expected {
        Ok(())
    } else {
        Err(OpError::input_count(expected, values.len()))
    }
}

impl<'a, A: FromValue<'a>> Inputs<'a> for A {
    fn types() -> Vec<ValueType> {
        vec![A::TYPE]
    }
    fn extract(values: &[&'a Value]) -> Result<Self, OpError> {
        check_count(values, 1)?;
        extract_one(values, 0)
    }
}

impl<'a, A: FromValue<'a>, B: FromValue<'a>> Inputs<'a> for (A, B) {
    fn types() -> Vec<ValueType> {
        vec![A::TYPE, B::TYPE]
    }
    fn extract(values: &[&'a Value]) -> Result<Self, OpError> {
        check_count(values, 2)?;
        Ok((extract_one(values, 0)?, extract_one(values, 1)?))
    }
}

/// The full output list of an operation: a single value, or a tuple of them.
pub trait Outputs {
    fn types() -> Vec<ValueType>;
    fn into_values(self) -> Vec<Value>;
}

impl<A: IntoValue> Outputs for A {
    fn types() -> Vec<ValueType> {
        vec![A::TYPE]
    }
    fn into_values(self) -> Vec<Value> {
        vec![self.into_value()]
    }
}

impl<A: IntoValue, B: IntoValue> Outputs for (A, B) {
    fn types() -> Vec<ValueType> {
        vec![A::TYPE, B::TYPE]
    }
    fn into_values(self) -> Vec<Value> {
        vec![self.0.into_value(), self.1.into_value()]
    }
}
