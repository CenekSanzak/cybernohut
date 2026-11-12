use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum KeyFormat {
    #[default]
    Text,
    Hex,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct XorConfig {
    pub key: String,
    /// Whether `key` is literal text or hexadecimal bytes.
    pub key_format: KeyFormat,
}

fn key_bytes(config: &XorConfig) -> Result<Vec<u8>, OpError> {
    let key = match config.key_format {
        KeyFormat::Text => config.key.as_bytes().to_vec(),
        KeyFormat::Hex => {
            let compact: String = config.key.chars().filter(|c| !c.is_ascii_whitespace()).collect();
            hex_simd::decode_to_vec(compact.as_bytes())
                .map_err(|_| OpError::invalid_config("The key is not valid hexadecimal"))?
        }
    };
    if key.is_empty() {
        return Err(OpError::invalid_config("Enter a key"));
    }
    Ok(key)
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

    fn run(config: &XorConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let key = key_bytes(config)?;
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
    fn empty_and_malformed_keys_are_config_errors() {
        let empty = Xor::run(&config("", KeyFormat::Text), b"x").unwrap_err();
        assert_eq!(empty.code, ErrorCode::InvalidConfig);
        let bad_hex = Xor::run(&config("zz", KeyFormat::Hex), b"x").unwrap_err();
        assert_eq!(bad_hex.code, ErrorCode::InvalidConfig);
    }
}
