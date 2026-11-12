use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Tag};

const LINK: &str = "https://en.wikipedia.org/wiki/Percent-encoding";
const HEX: &[u8; 16] = b"0123456789ABCDEF";

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
}

/// Percent-encodes everything except unreserved characters, like `encodeURIComponent`
/// but without that function's extra exceptions.
pub struct UrlEncode;

impl Op for UrlEncode {
    const ID: &'static str = "url-encode";
    const NAME: &'static str = "URL Encode";
    const DESCRIPTION: &'static str =
        "Percent-encodes every byte except letters, digits and - _ . ~";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        let mut output = Vec::with_capacity(input.len());
        for &byte in input {
            if is_unreserved(byte) {
                output.push(byte);
            } else {
                output.extend([b'%', HEX[(byte >> 4) as usize], HEX[(byte & 0xf) as usize]]);
            }
        }
        Ok(String::from_utf8(output).expect("percent-encoding produces only ASCII"))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct UrlDecodeConfig {
    /// Treat `+` as a space, as HTML form encoding does.
    pub plus_as_space: bool,
}

fn hex_digit(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|digit| digit as u8)
}

pub struct UrlDecode;

impl Op for UrlDecode {
    const ID: &'static str = "url-decode";
    const NAME: &'static str = "URL Decode";
    const DESCRIPTION: &'static str = "Decodes percent-encoded text into raw bytes";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = UrlDecodeConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new("plusAsSpace", "Treat + as space", ParamKind::Bool)]
    }

    fn run(config: &UrlDecodeConfig, input: &str) -> Result<Vec<u8>, OpError> {
        let bytes = input.as_bytes();
        let mut output = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'%' => {
                    let digits = bytes.get(index + 1).zip(bytes.get(index + 2));
                    let value = digits.and_then(|(high, low)| {
                        Some(hex_digit(*high)? << 4 | hex_digit(*low)?)
                    });
                    let Some(value) = value else {
                        return Err(OpError::invalid_input(format!(
                            "\"%\" at position {index} is not followed by two hex digits"
                        )));
                    };
                    output.push(value);
                    index += 3;
                }
                b'+' if config.plus_as_space => {
                    output.push(b' ');
                    index += 1;
                }
                byte => {
                    output.push(byte);
                    index += 1;
                }
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    #[test]
    fn reserved_and_non_ascii_bytes_are_escaped() {
        let encoded = UrlEncode::run(&NoConfig {}, "a b&c=ç/~".as_bytes()).unwrap();
        assert_eq!(encoded, "a%20b%26c%3D%C3%A7%2F~");
    }

    #[test]
    fn every_byte_survives_a_round_trip() {
        let bytes: Vec<u8> = (0..=255).collect();
        let encoded = UrlEncode::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(UrlDecode::run(&UrlDecodeConfig::default(), &encoded).unwrap(), bytes);
    }

    #[test]
    fn plus_is_a_space_only_when_asked() {
        let decode = |plus_as_space| UrlDecode::run(&UrlDecodeConfig { plus_as_space }, "a+b%2B").unwrap();
        assert_eq!(decode(false), b"a+b+");
        assert_eq!(decode(true), b"a b+");
    }

    #[test]
    fn a_dangling_percent_is_an_error() {
        for input in ["100%", "%4", "%zz"] {
            let error = UrlDecode::run(&UrlDecodeConfig::default(), input).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
        }
    }
}
