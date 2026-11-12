use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Suggestion, Tag};
use crate::score::text_score;

pub use super::key::KeyFormat;
use super::key::parse_key;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct XorConfig {
    pub key: String,
    /// Whether `key` is literal text or hexadecimal bytes.
    pub key_format: KeyFormat,
}

/// XOR with a repeating key. Applying it twice with the same key undoes it.
pub struct Xor;

impl Op for Xor {
    const ID: &'static str = "xor";
    const NAME: &'static str = "XOR";
    const DESCRIPTION: &'static str = "XORs every byte with a repeating key";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/XOR_cipher");
    const TAGS: &'static [Tag] = &[Tag::BasicEncryption];

    type Config = XorConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("key", "Key", ParamKind::Text),
            ParamSpec::new(
                "keyFormat",
                "Key format",
                ParamKind::Choice { options: vec!["text".into(), "hex".into()] },
            ),
        ]
    }

    /// Tries every single-byte key on the start of the input and offers the one that
    /// turns it into clearly readable text. Longer keys are not searched for.
    fn detect(input: &[u8]) -> Option<Suggestion<XorConfig>> {
        let sample = &input[..input.len().min(2048)];
        if sample.len() < 8 {
            return None;
        }
        let before = std::str::from_utf8(sample).map_or(0.0, text_score);
        let (key, after) = (1..=255u8)
            .filter_map(|key| {
                let decoded: Vec<u8> = sample.iter().map(|byte| byte ^ key).collect();
                Some((key, text_score(std::str::from_utf8(&decoded).ok()?)))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))?;
        (after >= 0.7 && after >= before + 0.15).then(|| Suggestion {
            config: XorConfig { key: format!("{key:02x}"), key_format: KeyFormat::Hex },
            reason: format!(
                "XOR with the single byte 0x{key:02x} gives readable text \
                 (readability goes from {before:.2} to {after:.2})"
            ),
        })
    }

    fn run(config: &XorConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let key = parse_key(&config.key, config.key_format)?;
        Ok(input.iter().zip(key.iter().cycle()).map(|(byte, key)| byte ^ key).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    fn config(key: &str, key_format: KeyFormat) -> XorConfig {
        XorConfig { key: key.into(), key_format }
    }

    #[test]
    fn repeating_key_xor_matches_a_known_vector() {
        // Cryptopals set 1, challenge 5, first bytes.
        let output = Xor::run(&config("ICE", KeyFormat::Text), b"Burning 'em").unwrap();
        assert_eq!(hex_simd::encode_to_string(&output, hex_simd::AsciiCase::Lower), "0b3637272a2b2e63622c2e");
    }

    #[test]
    fn applying_it_twice_restores_the_input() {
        let config = config("de ad BE ef", KeyFormat::Hex);
        let once = Xor::run(&config, b"nohut").unwrap();
        assert_ne!(once, b"nohut");
        assert_eq!(Xor::run(&config, &once).unwrap(), b"nohut");
    }

    #[test]
    fn detection_recovers_a_single_byte_key() {
        let prose = b"The secret message is hidden in the last file that you will find here.";
        let encrypted = Xor::run(&config("5a", KeyFormat::Hex), prose).unwrap();
        let suggestion = Xor::detect(&encrypted).expect("a key is suggested");
        assert_eq!(suggestion.config, config("5a", KeyFormat::Hex));
        assert!(Xor::detect(prose).is_none(), "plain text needs no key");
        assert!(Xor::detect(&[0x13, 0x37]).is_none(), "too short to judge");
    }

    #[test]
    fn empty_and_malformed_keys_are_config_errors() {
        let empty = Xor::run(&config("", KeyFormat::Text), b"x").unwrap_err();
        assert_eq!(empty.code, ErrorCode::InvalidConfig);
        let bad_hex = Xor::run(&config("zz", KeyFormat::Hex), b"x").unwrap_err();
        assert_eq!(bad_hex.code, ErrorCode::InvalidConfig);
    }
}
