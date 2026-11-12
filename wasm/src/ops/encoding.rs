use std::borrow::Cow;

use data_encoding::{Encoding, BASE32};
use hex_simd::AsciiCase;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Suggestion, Tag};

/// Encoded data is routinely wrapped or padded with whitespace; it carries no meaning.
/// Borrows the input unless there is whitespace to strip.
fn compact(input: &str) -> Cow<'_, str> {
    if input.bytes().any(|byte| byte.is_ascii_whitespace()) {
        Cow::Owned(input.chars().filter(|c| !c.is_ascii_whitespace()).collect())
    } else {
        Cow::Borrowed(input)
    }
}

/// `data-encoding` asserts that the input is shorter than `usize::MAX / 512`, which on
/// 32-bit wasm is about 8 MB, and panics past it. Feeding it block-aligned chunks keeps
/// every call far below the limit without changing the result.
const CHUNK_BLOCKS: usize = 1 << 16;

/// `block` is the number of input bytes that encode to a whole number of symbols.
fn encode_chunked(encoding: &Encoding, block: usize, input: &[u8]) -> String {
    let mut output = String::with_capacity(encoding.encode_len(input.len().min(1 << 20)));
    for chunk in input.chunks(block * CHUNK_BLOCKS) {
        encoding.encode_append(chunk, &mut output);
    }
    output
}

/// `block` is the number of symbols that decode to a whole number of bytes.
fn decode_chunked(encoding: &Encoding, block: usize, input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    for (index, chunk) in input.chunks(block * CHUNK_BLOCKS).enumerate() {
        let decoded = encoding.decode(chunk).map_err(|e| {
            format!("{} at position {}", e.kind, index * block * CHUNK_BLOCKS + e.position)
        })?;
        output.extend_from_slice(&decoded);
    }
    Ok(output)
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct HexEncodeConfig {
    pub uppercase: bool,
    /// Put between bytes, as in `de ad be ef` or `de:ad:be:ef`. Empty for none.
    pub separator: String,
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
        vec![
            ParamSpec::new("uppercase", "Uppercase", ParamKind::Bool),
            ParamSpec::new("separator", "Separator", ParamKind::Text),
        ]
    }

    fn run(config: &HexEncodeConfig, input: &[u8]) -> Result<String, OpError> {
        let case = if config.uppercase { AsciiCase::Upper } else { AsciiCase::Lower };
        let digits = hex_simd::encode_to_string(input, case);
        if config.separator.is_empty() {
            return Ok(digits);
        }
        let mut output = String::with_capacity(digits.len() / 2 * (2 + config.separator.len()));
        for (index, pair) in digits.as_bytes().chunks(2).enumerate() {
            if index > 0 {
                output.push_str(&config.separator);
            }
            output.push_str(std::str::from_utf8(pair).expect("hex digits are ASCII"));
        }
        Ok(output)
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

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let compact = compact(input);
        (compact.len() >= 4
            && compact.len() % 2 == 0
            && compact.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| Suggestion::new("Only hexadecimal digits, an even number of them"))
    }

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        // Hex is often written `de:ad:be:ef`, `de-ad` or `de,ad`.
        if input.bytes().any(|byte| matches!(byte, b':' | b'-' | b',')) {
            let digits: Vec<u8> = input
                .bytes()
                .filter(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b':' | b'-' | b','))
                .collect();
            return hex_simd::decode_to_vec(digits)
                .map_err(|_| OpError::invalid_input("Not valid hexadecimal"));
        }
        hex_simd::decode_to_vec(compact(input).as_bytes())
            .map_err(|_| OpError::invalid_input("Not valid hexadecimal"))
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
        Ok(encode_chunked(&BASE32, 5, input))
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

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let compact = compact(input);
        let data = compact.trim_end_matches('=');
        (compact.len() >= 8
            && compact.len() % 8 == 0
            && compact.len() - data.len() <= 6
            && data.bytes().all(|byte| matches!(byte, b'A'..=b'Z' | b'2'..=b'7')))
        .then(|| Suggestion::new("Only A-Z and 2-7, in groups of eight: the Base32 alphabet"))
    }

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        decode_chunked(&BASE32, 8, compact(input).as_bytes())
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
        let lower = Base16Encode::run(&HexEncodeConfig { uppercase: false, separator: String::new() }, &bytes).unwrap();
        let upper = Base16Encode::run(&HexEncodeConfig { uppercase: true, separator: String::new() }, &bytes).unwrap();
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
    fn chunking_does_not_change_the_result() {
        // Longer than one Base32 chunk, and not block-aligned.
        let bytes: Vec<u8> = (0..5 * CHUNK_BLOCKS + 7).map(|i| (i % 251) as u8).collect();
        let hex = Base16Encode::run(&HexEncodeConfig::default(), &bytes).unwrap();
        assert_eq!(hex.len(), bytes.len() * 2);
        assert_eq!(Base16Decode::run(&NoConfig {}, &hex).unwrap(), bytes);
        let b32 = Base32Encode::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(b32, BASE32.encode(&bytes));
        assert_eq!(Base32Decode::run(&NoConfig {}, &b32).unwrap(), bytes);
    }

    #[test]
    fn detection_follows_each_alphabet() {
        assert!(Base16Decode::detect("68656c6c6f").is_some());
        assert!(Base16Decode::detect("68 65 6C 6c").is_some());
        assert!(Base16Decode::detect("68656c6c6").is_none(), "odd length");
        assert!(Base16Decode::detect("hello!").is_none());
        assert!(Base32Decode::detect("NBSWY3DPEB3W64TMMQ======").is_some());
        assert!(Base32Decode::detect("nbswy3dpeb3w64tm").is_none(), "lowercase is not Base32");
        assert!(Base32Decode::detect("NBSWY3DP1").is_none());
    }

    #[test]
    fn invalid_input_is_an_error() {
        assert_eq!(Base16Decode::run(&NoConfig {}, "xyz").unwrap_err().code, ErrorCode::InvalidInput);
        assert_eq!(Base32Decode::run(&NoConfig {}, "!!!").unwrap_err().code, ErrorCode::InvalidInput);
    }

    #[test]
    fn hex_can_be_written_and_read_with_separators() {
        let spaced = HexEncodeConfig { uppercase: false, separator: " ".into() };
        assert_eq!(Base16Encode::run(&spaced, &[0xde, 0xad, 0xbe]).unwrap(), "de ad be");
        let colons = HexEncodeConfig { uppercase: true, separator: ":".into() };
        assert_eq!(Base16Encode::run(&colons, &[0xde, 0xad]).unwrap(), "DE:AD");
        for written in ["de ad be", "DE:AD:BE", "de-ad-be", "de,ad,be"] {
            assert_eq!(Base16Decode::run(&NoConfig {}, written).unwrap(), [0xde, 0xad, 0xbe]);
        }
    }
}
