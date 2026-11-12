//! Base58, the alphabet of Bitcoin addresses and IPFS hashes: Base64 without the
//! characters that look alike.

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero};

use crate::core::{NoConfig, Op, OpError, Suggestion, Tag};

const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const LINK: &str = "https://en.wikipedia.org/wiki/Binary-to-text_encoding#Base58";
/// The conversion is quadratic in the input length; this keeps it well under a second.
const MAX_BYTES: usize = 64 * 1024;

fn too_long() -> OpError {
    OpError::invalid_input("Base58 is meant for short values; this input is larger than 64 KB")
}

pub struct Base58Encode;

impl Op for Base58Encode {
    const ID: &'static str = "base58-encode";
    const NAME: &'static str = "Base58 Encode";
    const DESCRIPTION: &'static str = "Encodes to Base58 with the Bitcoin alphabet";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        if input.len() > MAX_BYTES {
            return Err(too_long());
        }
        // The input is one big number; leading zero bytes, which a number cannot
        // show, are written as leading `1`s.
        let zeros = input.iter().take_while(|byte| **byte == 0).count();
        let mut number = BigUint::from_bytes_be(input);
        let base = BigUint::from(58u8);
        let mut digits = Vec::new();
        while !number.is_zero() {
            let (quotient, remainder) = number.div_rem(&base);
            digits.push(ALPHABET[remainder.to_usize().expect("a remainder below 58")]);
            number = quotient;
        }
        digits.extend(std::iter::repeat_n(b'1', zeros));
        digits.reverse();
        Ok(String::from_utf8(digits).expect("the alphabet is ASCII"))
    }
}

pub struct Base58Decode;

impl Op for Base58Decode {
    const ID: &'static str = "base58-decode";
    const NAME: &'static str = "Base58 Decode";
    const DESCRIPTION: &'static str = "Decodes Base58 written with the Bitcoin alphabet";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let compact = input.trim();
        // Base58 has no shape of its own, so only its tell-tale absences count: no
        // 0, O, I or l among a good number of mixed letters and digits.
        let mixed = compact.bytes().any(|b| b.is_ascii_digit())
            && compact.bytes().any(|b| b.is_ascii_uppercase())
            && compact.bytes().any(|b| b.is_ascii_lowercase());
        (compact.len() >= 16 && mixed && compact.bytes().all(|b| ALPHABET.contains(&b)))
            .then(|| Suggestion::new("Letters and digits without 0, O, I or l: the Base58 alphabet"))
    }

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        let compact: Vec<u8> = input.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
        if compact.len() > MAX_BYTES * 2 {
            return Err(too_long());
        }
        let mut number = BigUint::zero();
        for symbol in &compact {
            let value = ALPHABET.iter().position(|known| known == symbol).ok_or_else(|| {
                OpError::invalid_input(format!("\"{}\" is not a Base58 character", char::from(*symbol)))
            })?;
            number = number * 58u8 + value;
        }
        let zeros = compact.iter().take_while(|symbol| **symbol == b'1').count();
        let mut output = vec![0u8; zeros];
        if !number.is_zero() {
            output.extend(number.to_bytes_be());
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors_round_trip() {
        // From the Base58 test vectors used by Bitcoin Core.
        for (hex, encoded) in [
            ("", ""),
            ("61", "2g"),
            ("626262", "a3gV"),
            ("73696d706c792061206c6f6e6720737472696e67", "2cFupjhnEsSn59qHXstmK2ffpLv2"),
            ("00eb15231dfceb60925886b67d065299925915aeb172c06647", "1NS17iag9jJgTHD1VXjvLCEnZuQ3rJDE9L"),
            ("00000000000000000000", "1111111111"),
        ] {
            let bytes = hex_simd::decode_to_vec(hex).unwrap();
            assert_eq!(Base58Encode::run(&NoConfig {}, &bytes).unwrap(), encoded);
            assert_eq!(Base58Decode::run(&NoConfig {}, encoded).unwrap(), bytes);
        }
    }

    #[test]
    fn characters_outside_the_alphabet_are_rejected_and_shape_is_detected() {
        assert!(Base58Decode::run(&NoConfig {}, "abc0").is_err());
        assert!(Base58Decode::detect("1NS17iag9jJgTHD1VXjvLCEnZuQ3rJDE9L").is_some());
        assert!(Base58Decode::detect("password").is_none());
        assert!(Base58Decode::detect("VGhlIHNlY3JldCBtZXNzYWdl0O").is_none());
    }
}
