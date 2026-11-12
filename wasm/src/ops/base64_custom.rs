//! Base64 with an alphabet of the user's choosing, as malware and CTF challenges use
//! to keep a standard decoder from working.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

const STANDARD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const LINK: &str = "https://en.wikipedia.org/wiki/Base64#Variants_summary_table";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct Base64CustomConfig {
    /// The 64 symbols, in order of the values 0 to 63.
    pub alphabet: String,
    /// Fill the last group up to four symbols with `=`.
    pub padding: bool,
}

impl Default for Base64CustomConfig {
    fn default() -> Self {
        Base64CustomConfig { alphabet: STANDARD.into(), padding: true }
    }
}

fn params() -> Vec<ParamSpec> {
    vec![
        ParamSpec::new("alphabet", "Alphabet", ParamKind::Text),
        ParamSpec::new("padding", "Padding", ParamKind::Bool),
    ]
}

fn alphabet(config: &Base64CustomConfig) -> Result<[u8; 64], OpError> {
    let symbols: [u8; 64] = config.alphabet.as_bytes().try_into().map_err(|_| {
        OpError::invalid_config(format!(
            "The alphabet has {} symbols; it needs exactly 64 one-byte characters",
            config.alphabet.chars().count()
        ))
    })?;
    let mut seen = [false; 256];
    for symbol in symbols {
        if symbol == b'=' || !symbol.is_ascii_graphic() || std::mem::replace(&mut seen[usize::from(symbol)], true) {
            return Err(OpError::invalid_config(format!(
                "\"{}\" cannot be used, or is used twice, in the alphabet",
                char::from(symbol)
            )));
        }
    }
    Ok(symbols)
}

pub struct Base64CustomEncode;

impl Op for Base64CustomEncode {
    const ID: &'static str = "base64-custom-encode";
    const NAME: &'static str = "Base64 (Custom Alphabet) Encode";
    const DESCRIPTION: &'static str = "Encodes to Base64 using the 64 symbols you give";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = Base64CustomConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        params()
    }

    fn run(config: &Base64CustomConfig, input: &[u8]) -> Result<String, OpError> {
        let symbols = alphabet(config)?;
        let mut output = Vec::with_capacity(input.len().div_ceil(3) * 4);
        for chunk in input.chunks(3) {
            let group = chunk.iter().fold(0u32, |group, byte| group << 8 | u32::from(*byte))
                << (8 * (3 - chunk.len()));
            // n bytes fill n + 1 symbols.
            for index in 0..=chunk.len() {
                output.push(symbols[(group >> (18 - 6 * index) & 63) as usize]);
            }
            if config.padding {
                output.extend(std::iter::repeat_n(b'=', 3 - chunk.len()));
            }
        }
        Ok(String::from_utf8(output).expect("the alphabet is ASCII"))
    }
}

pub struct Base64CustomDecode;

impl Op for Base64CustomDecode {
    const ID: &'static str = "base64-custom-decode";
    const NAME: &'static str = "Base64 (Custom Alphabet) Decode";
    const DESCRIPTION: &'static str = "Decodes Base64 written with the 64 symbols you give";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = Base64CustomConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        params()
    }

    fn run(config: &Base64CustomConfig, input: &str) -> Result<Vec<u8>, OpError> {
        let symbols = alphabet(config)?;
        let mut values = [u8::MAX; 256];
        for (value, symbol) in symbols.iter().enumerate() {
            values[usize::from(*symbol)] = value as u8;
        }
        // Padding and whitespace carry no data, with or without the padding setting.
        let data: Vec<u8> =
            input.bytes().filter(|byte| !byte.is_ascii_whitespace() && *byte != b'=').collect();
        if data.len() % 4 == 1 {
            return Err(OpError::invalid_input("The input is one symbol too long or too short"));
        }
        let mut output = Vec::with_capacity(data.len() / 4 * 3 + 2);
        for chunk in data.chunks(4) {
            let mut group = 0u32;
            for symbol in chunk {
                let value = values[usize::from(*symbol)];
                if value == u8::MAX {
                    return Err(OpError::invalid_input(format!(
                        "\"{}\" is not in the alphabet",
                        char::from(*symbol)
                    )));
                }
                group = group << 6 | u32::from(value);
            }
            group <<= 6 * (4 - chunk.len());
            // n symbols carry n - 1 bytes.
            output.extend((0..chunk.len() - 1).map(|index| (group >> (16 - 8 * index)) as u8));
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    fn config(alphabet: &str, padding: bool) -> Base64CustomConfig {
        Base64CustomConfig { alphabet: alphabet.into(), padding }
    }

    #[test]
    fn the_standard_alphabet_gives_standard_base64() {
        for (plain, encoded) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")] {
            let default = Base64CustomConfig::default();
            assert_eq!(Base64CustomEncode::run(&default, plain.as_bytes()).unwrap(), encoded);
            assert_eq!(Base64CustomDecode::run(&default, encoded).unwrap(), plain.as_bytes());
        }
        assert_eq!(Base64CustomEncode::run(&config(STANDARD, false), b"f").unwrap(), "Zg");
        assert_eq!(Base64CustomDecode::run(&config(STANDARD, true), "Zg").unwrap(), b"f");
    }

    #[test]
    fn a_shuffled_alphabet_round_trips_and_differs_from_standard() {
        let reversed: String = STANDARD.chars().rev().collect();
        let shuffled = config(&reversed, true);
        let encoded = Base64CustomEncode::run(&shuffled, b"nohut \xff\x00").unwrap();
        assert_ne!(encoded, Base64CustomEncode::run(&Base64CustomConfig::default(), b"nohut \xff\x00").unwrap());
        assert_eq!(Base64CustomDecode::run(&shuffled, &encoded).unwrap(), b"nohut \xff\x00");
    }

    #[test]
    fn bad_alphabets_and_inputs_are_reported() {
        for alphabet in ["short", &"A".repeat(64), &STANDARD.replace('+', "=")] {
            let error = Base64CustomEncode::run(&config(alphabet, true), b"x").unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidConfig, "{alphabet}");
        }
        let default = Base64CustomConfig::default();
        assert_eq!(Base64CustomDecode::run(&default, "Zm9*").unwrap_err().code, ErrorCode::InvalidInput);
        assert_eq!(Base64CustomDecode::run(&default, "Zm9vY").unwrap_err().code, ErrorCode::InvalidInput);
    }
}
