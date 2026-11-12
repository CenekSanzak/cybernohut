use data_encoding::{BASE32, HEXLOWER, HEXLOWER_PERMISSIVE, HEXUPPER};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Tag};

/// Encoded data is routinely wrapped or padded with whitespace; it carries no meaning.
fn compact(input: &str) -> String {
    input.chars().filter(|c| !c.is_ascii_whitespace()).collect()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct HexEncodeConfig {
    pub uppercase: bool,
}

pub struct Base16Encode;

impl Op for Base16Encode {
    const ID: &'static str = "base16-encode";
    const NAME: &'static str = "Hexadecimal Encode (Base16)";
    const DESCRIPTION: &'static str = "Encodes the input as hexadecimal";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Hexadecimal");
    const TAGS: &'static [Tag] = &[Tag::NumberBase, Tag::Encoding];

    type Config = HexEncodeConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new("uppercase", "Uppercase", ParamKind::Bool)]
    }

    fn run(config: &HexEncodeConfig, input: &[u8]) -> Result<String, OpError> {
        Ok(if config.uppercase { HEXUPPER.encode(input) } else { HEXLOWER.encode(input) })
    }
}

pub struct Base16Decode;

impl Op for Base16Decode {
    const ID: &'static str = "base16-decode";
    const NAME: &'static str = "Hexadecimal Decode (Base16)";
    const DESCRIPTION: &'static str = "Decodes hexadecimal into raw bytes";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Hexadecimal");
    const TAGS: &'static [Tag] = &[Tag::NumberBase, Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        HEXLOWER_PERMISSIVE
            .decode(compact(input).as_bytes())
            .map_err(|e| OpError::invalid_input(format!("Not valid hexadecimal: {e}")))
    }
}

pub struct Base32Encode;

impl Op for Base32Encode {
    const ID: &'static str = "base32-encode";
    const NAME: &'static str = "Base32 Encode";
    const DESCRIPTION: &'static str = "Encodes the input to Base32";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Base32");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        Ok(BASE32.encode(input))
    }
}

pub struct Base32Decode;

impl Op for Base32Decode {
    const ID: &'static str = "base32-decode";
    const NAME: &'static str = "Base32 Decode";
    const DESCRIPTION: &'static str = "Decodes Base32 into raw bytes";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Base32");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        BASE32
            .decode(compact(input).as_bytes())
            .map_err(|e| OpError::invalid_input(format!("Not valid Base32: {e}")))
    }
}

pub struct Base85Encode;

impl Op for Base85Encode {
    const ID: &'static str = "base85-encode";
    const NAME: &'static str = "Base85 Encode";
    const DESCRIPTION: &'static str = "Encodes the input to Base85";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Ascii85");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        Ok(base85::encode(input))
    }
}

pub struct Base85Decode;

impl Op for Base85Decode {
    const ID: &'static str = "base85-decode";
    const NAME: &'static str = "Base85 Decode";
    const DESCRIPTION: &'static str = "Decodes Base85 into raw bytes";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Ascii85");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        base85::decode(&compact(input))
            .ok_or_else(|| OpError::invalid_input("Not valid Base85"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    #[test]
    fn hex_encodes_in_either_case_and_decodes_both() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        let lower = Base16Encode::run(&HexEncodeConfig { uppercase: false }, &bytes).unwrap();
        let upper = Base16Encode::run(&HexEncodeConfig { uppercase: true }, &bytes).unwrap();
        assert_eq!((lower.as_str(), upper.as_str()), ("deadbeef", "DEADBEEF"));
        assert_eq!(Base16Decode::run(&NoConfig {}, &lower).unwrap(), bytes);
        assert_eq!(Base16Decode::run(&NoConfig {}, "DE AD\nBE EF").unwrap(), bytes);
    }

    #[test]
    fn binary_survives_every_round_trip() {
        let bytes: Vec<u8> = (0..=255).collect();
        let b32 = Base32Encode::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(Base32Decode::run(&NoConfig {}, &b32).unwrap(), bytes);
        let b85 = Base85Encode::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(Base85Decode::run(&NoConfig {}, &b85).unwrap(), bytes);
    }

    #[test]
    fn invalid_input_is_an_error() {
        assert_eq!(Base16Decode::run(&NoConfig {}, "xyz").unwrap_err().code, ErrorCode::InvalidInput);
        assert_eq!(Base32Decode::run(&NoConfig {}, "!!!").unwrap_err().code, ErrorCode::InvalidInput);
    }
}
