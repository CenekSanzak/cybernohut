//! JSON, and the token format built on it.

use serde::{Deserialize, Serialize};
use serde_json::ser::{PrettyFormatter, Serializer};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Suggestion, Tag};

const LINK: &str = "https://en.wikipedia.org/wiki/JSON";

/// Key order and the exact digits of numbers are kept: this reformats, it does not
/// reinterpret.
fn parse(input: &str) -> Result<serde_json::Value, OpError> {
    serde_json::from_str(input).map_err(|error| OpError::invalid_input(format!("Not valid JSON: {error}")))
}

fn pretty(value: &serde_json::Value, indent: usize) -> String {
    let indent = " ".repeat(indent);
    let mut output = Vec::new();
    let mut serializer =
        Serializer::with_formatter(&mut output, PrettyFormatter::with_indent(indent.as_bytes()));
    value.serialize(&mut serializer).expect("a parsed value serializes");
    String::from_utf8(output).expect("JSON is UTF-8")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct JsonFormatConfig {
    /// Spaces per nesting level.
    pub indent: u8,
}

impl Default for JsonFormatConfig {
    fn default() -> Self {
        JsonFormatConfig { indent: 2 }
    }
}

pub struct JsonFormat;

impl Op for JsonFormat {
    const ID: &'static str = "json-format";
    const NAME: &'static str = "JSON Format";
    const DESCRIPTION: &'static str = "Lays JSON out with one value per line, indented";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::DataFormat];

    type Config = JsonFormatConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new("indent", "Indent", ParamKind::Int { min: Some(0), max: Some(8) })]
    }

    fn run(config: &JsonFormatConfig, input: &str) -> Result<String, OpError> {
        Ok(pretty(&parse(input)?, usize::from(config.indent.min(8))))
    }
}

pub struct JsonMinify;

impl Op for JsonMinify {
    const ID: &'static str = "json-minify";
    const NAME: &'static str = "JSON Minify";
    const DESCRIPTION: &'static str = "Removes all whitespace between JSON values";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::DataFormat];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(parse(input)?.to_string())
    }
}

/// The header and payload of a token, or why it is not one.
fn jwt_parts(token: &str) -> Result<(serde_json::Value, serde_json::Value, &str), OpError> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    let [header, payload, signature] = parts[..] else {
        return Err(OpError::invalid_input(format!(
            "A JSON Web Token has three parts separated by dots; this has {}",
            parts.len()
        )));
    };
    let decode = |part: &str, name: &str| {
        let bytes = base64_simd::URL_SAFE_NO_PAD
            .decode_to_vec(part.trim_end_matches('='))
            .map_err(|_| OpError::invalid_input(format!("The {name} is not valid Base64URL")))?;
        serde_json::from_slice::<serde_json::Value>(&bytes)
            .map_err(|error| OpError::invalid_input(format!("The {name} is not valid JSON: {error}")))
    };
    Ok((decode(header, "header")?, decode(payload, "payload")?, signature))
}

/// Reads a token; it does not check the signature, which needs the key.
pub struct JwtDecode;

impl Op for JwtDecode {
    const ID: &'static str = "jwt-decode";
    const NAME: &'static str = "JWT Decode";
    const DESCRIPTION: &'static str =
        "Shows the header and payload of a JSON Web Token. The signature is not verified";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/JSON_Web_Token");
    const TAGS: &'static [Tag] = &[Tag::DataFormat];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        // Every JSON object starts with `{"`, which Base64 turns into `eyJ`.
        let (header, _, _) = input.starts_with("eyJ").then(|| jwt_parts(input).ok())??;
        let algorithm = header.get("alg").and_then(|alg| alg.as_str()).unwrap_or("unknown");
        Some(Suggestion::new(format!(
            "Three Base64URL parts with a JSON header: a JSON Web Token signed with {algorithm}"
        )))
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let (header, payload, signature) = jwt_parts(input)?;
        let mut token = serde_json::Map::new();
        token.insert("header".into(), header);
        token.insert("payload".into(), payload);
        token.insert("signature".into(), signature.into());
        Ok(pretty(&serde_json::Value::Object(token), 2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    // The example token from jwt.io.
    const TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
        eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
        SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[test]
    fn formatting_keeps_key_order_and_number_digits() {
        let input = r#"{"z":1,"a":[1.50,12345678901234567890],"m":{}}"#;
        let formatted = JsonFormat::run(&JsonFormatConfig::default(), input).unwrap();
        assert_eq!(
            formatted,
            "{\n  \"z\": 1,\n  \"a\": [\n    1.50,\n    12345678901234567890\n  ],\n  \"m\": {}\n}"
        );
        assert_eq!(JsonMinify::run(&NoConfig {}, &formatted).unwrap(), input);
    }

    #[test]
    fn indent_is_configurable() {
        let four = JsonFormat::run(&JsonFormatConfig { indent: 4 }, "[1]").unwrap();
        assert_eq!(four, "[\n    1\n]");
        let none = JsonFormat::run(&JsonFormatConfig { indent: 0 }, "[1]").unwrap();
        assert_eq!(none, "[\n1\n]");
    }

    #[test]
    fn invalid_json_says_where() {
        let error = JsonFormat::run(&JsonFormatConfig::default(), "{\"a\": }").unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error.message.contains("column"), "{}", error.message);
    }

    #[test]
    fn a_token_is_split_into_header_payload_and_signature() {
        let decoded = JwtDecode::run(&NoConfig {}, TOKEN).unwrap();
        let value: serde_json::Value = serde_json::from_str(&decoded).unwrap();
        assert_eq!(value["header"]["alg"], "HS256");
        assert_eq!(value["payload"]["name"], "John Doe");
        assert_eq!(value["signature"], "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c");
    }

    #[test]
    fn tokens_are_detected_and_other_text_is_not() {
        let suggestion = JwtDecode::detect(TOKEN).expect("a token is recognized");
        assert!(suggestion.reason.contains("HS256"));
        assert!(JwtDecode::detect("eyJub3QiOiJhIHRva2VuIn0").is_none());
        assert!(JwtDecode::detect("a.b.c").is_none());
        let error = JwtDecode::run(&NoConfig {}, "a.b").unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
}
