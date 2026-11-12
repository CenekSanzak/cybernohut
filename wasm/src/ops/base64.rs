use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::engine::GeneralPurpose;
use base64::Engine as _;

use crate::core::{NoConfig, Op, OpError, Tag};

fn decode(engine: &GeneralPurpose, input: &str) -> Result<Vec<u8>, OpError> {
    // Encoded data is routinely wrapped or padded with whitespace; it carries no meaning.
    let compact: String = input.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    engine
        .decode(compact)
        .map_err(|e| OpError::invalid_input(format!("Not valid Base64: {e}")))
}

macro_rules! base64_ops {
    ($encode:ident, $decode:ident, $engine:expr, $id:literal, $label:literal) => {
        pub struct $encode;

        impl Op for $encode {
            const ID: &'static str = concat!($id, "-encode");
            const NAME: &'static str = concat!($label, " Encode");
            const DESCRIPTION: &'static str = concat!("Encodes the input to ", $label);
            const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Base64");
            const TAGS: &'static [Tag] = &[Tag::Encoding];

            type Config = NoConfig;
            type In<'a> = &'a [u8];
            type Out = String;

            fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
                Ok($engine.encode(input))
            }
        }

        pub struct $decode;

        impl Op for $decode {
            const ID: &'static str = concat!($id, "-decode");
            const NAME: &'static str = concat!($label, " Decode");
            const DESCRIPTION: &'static str = concat!("Decodes ", $label, " into raw bytes");
            const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Base64");
            const TAGS: &'static [Tag] = &[Tag::Encoding];

            type Config = NoConfig;
            type In<'a> = &'a str;
            type Out = Vec<u8>;

            fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
                decode(&$engine, input)
            }
        }
    };
}

base64_ops!(Base64StandardEncode, Base64StandardDecode, STANDARD, "base64-standard", "Base64 (Standard)");
base64_ops!(Base64UrlEncode, Base64UrlDecode, URL_SAFE, "base64-url", "Base64 (URL Safe)");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    #[test]
    fn binary_survives_a_round_trip() {
        let bytes: Vec<u8> = (0..=255).collect();
        let encoded = Base64StandardEncode::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(Base64StandardDecode::run(&NoConfig {}, &encoded).unwrap(), bytes);
    }

    #[test]
    fn multi_line_input_is_encoded_as_a_whole() {
        assert_eq!(Base64StandardEncode::run(&NoConfig {}, b"a\nb").unwrap(), "YQpi");
    }

    #[test]
    fn whitespace_in_encoded_input_is_ignored() {
        assert_eq!(Base64StandardDecode::run(&NoConfig {}, "aGVs\nbG8=\n").unwrap(), b"hello");
    }

    #[test]
    fn invalid_input_is_an_error_not_an_empty_result() {
        let error = Base64StandardDecode::run(&NoConfig {}, "!!!not base64").unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn url_alphabet_differs_from_standard() {
        assert_eq!(Base64StandardEncode::run(&NoConfig {}, &[0xfb, 0xff]).unwrap(), "+/8=");
        assert_eq!(Base64UrlEncode::run(&NoConfig {}, &[0xfb, 0xff]).unwrap(), "-_8=");
    }
}
