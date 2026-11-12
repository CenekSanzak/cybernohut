use base64_simd::{Base64, STANDARD, URL_SAFE};

use crate::core::{NoConfig, Op, OpError, Suggestion, Tag};

fn decode(engine: &Base64, input: &str) -> Result<Vec<u8>, OpError> {
    // Encoded data is routinely wrapped or padded with whitespace; it carries no meaning.
    // Copying the input to strip it is only worth it when there is some.
    let decoded = if input.bytes().any(|byte| byte.is_ascii_whitespace()) {
        let compact: Vec<u8> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
        engine.decode_to_vec(compact)
    } else {
        engine.decode_to_vec(input)
    };
    decoded.map_err(|_| OpError::invalid_input("Not valid Base64"))
}

/// Whether `input` is shaped like Base64 in the given alphabet: at least two groups
/// of four, nothing outside the alphabet, and padding only at the very end.
fn looks_like_base64(input: &str, extra: [u8; 2]) -> bool {
    let compact: Vec<u8> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    let data_len = compact.iter().rposition(|byte| *byte != b'=').map_or(0, |last| last + 1);
    compact.len() >= 8
        && compact.len() % 4 == 0
        && compact.len() - data_len <= 2
        && compact[..data_len].iter().all(|byte| byte.is_ascii_alphanumeric() || extra.contains(byte))
}

macro_rules! base64_ops {
    ($encode:ident, $decode:ident, $engine:expr, $id:literal, $label:literal, $detect:expr) => {
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
                Ok($engine.encode_to_string(input))
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

            fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
                let detect: fn(&str) -> Option<&'static str> = $detect;
                detect(input).map(Suggestion::new)
            }

            fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
                decode(&$engine, input)
            }
        }
    };
}

base64_ops!(
    Base64StandardEncode,
    Base64StandardDecode,
    STANDARD,
    "base64-standard",
    "Base64 (Standard)",
    |input| looks_like_base64(input, *b"+/")
        .then_some("Only Base64 characters, in groups of four")
);
base64_ops!(
    Base64UrlEncode,
    Base64UrlDecode,
    URL_SAFE,
    "base64-url",
    "Base64 (URL Safe)",
    // Without - or _ the standard alphabet already covers it.
    |input| (looks_like_base64(input, *b"-_") && input.contains(['-', '_']))
        .then_some("Base64 characters including - or _, the URL-safe alphabet")
);

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
    fn detection_requires_the_shape_of_base64() {
        let standard = |input| Base64StandardDecode::detect(input).is_some();
        assert!(standard("aGVsbG8gd29ybGQ="));
        assert!(standard("aGVs\nbG8g\nd29y\nbGQ="));
        assert!(!standard("aGVsbG8"), "not a multiple of four");
        assert!(!standard("aGV="), "too short");
        assert!(!standard("aGVs=G8gd29ybGQ="), "padding in the middle");
        assert!(!standard("hello, world"), "characters outside the alphabet");
        assert!(Base64UrlDecode::detect("-_-_-_8=").is_some());
        assert!(Base64UrlDecode::detect("aGVsbG8gd29ybGQ=").is_none());
    }

    #[test]
    fn url_alphabet_differs_from_standard() {
        assert_eq!(Base64StandardEncode::run(&NoConfig {}, &[0xfb, 0xff]).unwrap(), "+/8=");
        assert_eq!(Base64UrlEncode::run(&NoConfig {}, &[0xfb, 0xff]).unwrap(), "-_8=");
    }
}
