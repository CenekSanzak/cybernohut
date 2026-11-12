//! The operation contract. Everything the UI knows about an operation is derived from
//! an implementation of [`Op`]; nothing is declared a second time in TypeScript.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::error::OpError;
use super::value::{Inputs, Outputs, Value, ValueType};

pub trait Op: 'static {
    const ID: &'static str;
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const LINK: Option<&'static str> = None;
    const TAGS: &'static [Tag];
    const INPUTS: &'static [&'static str] = &["input"];
    const OUTPUTS: &'static [&'static str] = &["output"];

    type Config: DeserializeOwned + Serialize + Default;
    /// Port types come from these two, so the declaration cannot drift from the code.
    type In<'a>: Inputs<'a>;
    type Out: Outputs;

    /// How the UI should render each config field.
    fn params() -> Vec<ParamSpec> {
        Vec::new()
    }

    fn run(config: &Self::Config, input: Self::In<'_>) -> Result<Self::Out, OpError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub enum Tag {
    Text,
    #[serde(rename = "Basic Encryption")]
    BasicEncryption,
    Encoding,
    Hashing,
    #[serde(rename = "Number Base")]
    NumberBase,
    #[serde(rename = "I/O")]
    Io,
    #[serde(rename = "Substitution Cipher")]
    SubstitutionCipher,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct Port {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ValueType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct ParamSpec {
    pub key: String,
    pub label: String,
    pub kind: ParamKind,
}

impl ParamSpec {
    pub fn new(key: &str, label: &str, kind: ParamKind) -> Self {
        ParamSpec { key: key.into(), label: label.into(), kind }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParamKind {
    Int { min: Option<i32>, max: Option<i32> },
    Bool,
    Text,
    Choice { options: Vec<String> },
}

/// The description of one operation, as shipped to the UI in the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct OpMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub link: Option<String>,
    pub tags: Vec<Tag>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub params: Vec<ParamSpec>,
    /// Whether the node can be switched to run on each line separately.
    pub supports_per_line: bool,
    #[tsify(type = "Record<string, string | number | boolean>")]
    pub default_config: serde_json::Value,
}

/// A type-erased operation, as stored in the registry.
pub struct OpEntry {
    pub id: &'static str,
    pub meta: fn() -> OpMeta,
    pub run: fn(&serde_json::Value, &[&Value]) -> Result<Vec<Value>, OpError>,
}

pub fn entry<T: Op>() -> OpEntry {
    OpEntry { id: T::ID, meta: meta_of::<T>, run: run_erased::<T> }
}

fn ports(names: &[&str], types: Vec<ValueType>) -> Vec<Port> {
    assert_eq!(names.len(), types.len(), "port names and port types differ in count");
    names.iter().zip(types).map(|(name, ty)| Port { name: (*name).into(), ty }).collect()
}

fn meta_of<T: Op>() -> OpMeta {
    let inputs = ports(T::INPUTS, <T::In<'static> as Inputs>::types());
    let outputs = ports(T::OUTPUTS, <T::Out as Outputs>::types());
    let supports_per_line = inputs.len() == 1
        && inputs[0].ty.accepts(ValueType::Text)
        && outputs.len() == 1
        && outputs[0].ty == ValueType::Text;
    OpMeta {
        id: T::ID.into(),
        name: T::NAME.into(),
        description: T::DESCRIPTION.into(),
        link: T::LINK.map(Into::into),
        tags: T::TAGS.to_vec(),
        inputs,
        outputs,
        params: T::params(),
        supports_per_line,
        default_config: serde_json::to_value(T::Config::default())
            .expect("a default config always serializes"),
    }
}

fn run_erased<'a, T: Op>(
    config: &serde_json::Value,
    inputs: &[&'a Value],
) -> Result<Vec<Value>, OpError> {
    let config: T::Config = if config.is_null() {
        T::Config::default()
    } else {
        serde_json::from_value(config.clone())
            .map_err(|e| OpError::invalid_config(format!("Invalid config for {}: {e}", T::NAME)))?
    };
    let input = <T::In<'a> as Inputs<'a>>::extract(inputs)?;
    Ok(T::run(&config, input)?.into_values())
}

/// Config type for operations that have nothing to configure.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
pub struct NoConfig {}
