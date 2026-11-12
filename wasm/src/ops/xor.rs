use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Suggestion, Tag, Value};
use crate::score::{text_score, ENGLISH_LETTER_PERCENT};

pub use super::key::KeyFormat;
use super::key::parse_key;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct XorConfig {
    pub key: String,
    /// Whether `key` is literal text or hexadecimal bytes.
    pub key_format: KeyFormat,
}

/// Detection looks at this much of the input.
const SAMPLE_BYTES: usize = 2048;
const MAX_KEY_BYTES: usize = 32;
/// Each key byte is found from the bytes it was applied to, so it needs a few.
const MIN_BYTES_PER_KEY_BYTE: usize = 12;
/// How readable the result has to be, and how much more readable than the input.
const MIN_SCORE: f64 = 0.7;
const MIN_GAIN: f64 = 0.15;

fn apply(input: &[u8], key: &[u8]) -> Vec<u8> {
    input.iter().zip(key.iter().cycle()).map(|(byte, key)| byte ^ key).collect()
}

/// Scores bytes as text. A sample may end in the middle of a multi-byte character;
/// that is not held against it.
fn readability(bytes: &[u8]) -> Option<f64> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text_score(text)),
        Err(error) if error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()]).ok().map(text_score)
        }
        Err(_) => None,
    }
}

/// How much one decrypted byte looks like it came from English text.
fn byte_fit(byte: u8) -> f64 {
    match byte {
        b'a'..=b'z' => ENGLISH_LETTER_PERCENT[usize::from(byte - b'a')],
        b'A'..=b'Z' => ENGLISH_LETTER_PERCENT[usize::from(byte - b'A')] / 2.0,
        b' ' => 15.0,
        b'\n' | b'\r' | b'\t' | b'!'..=b'~' => 1.0,
        _ => -30.0,
    }
}

/// The same without assuming a language: text is mostly letters and spaces, and
/// letters outside ASCII are bytes above 0x7f.
fn byte_fit_any_language(byte: u8) -> f64 {
    match byte {
        b'a'..=b'z' => 6.0,
        b'A'..=b'Z' => 3.0,
        b' ' => 15.0,
        0x80..=0xff => 3.0,
        b'\n' | b'\r' | b'\t' | b'!'..=b'~' => 1.0,
        _ => -30.0,
    }
}

/// The key byte that makes every `length`-th byte from `offset` look most like text.
fn column_key(sample: &[u8], offset: usize, length: usize, fit: fn(u8) -> f64) -> u8 {
    let total = |key: u8| -> f64 {
        sample.iter().skip(offset).step_by(length).map(|byte| fit(byte ^ key)).sum()
    };
    (0..=255u8).max_by(|a, b| total(*a).total_cmp(&total(*b))).expect("the range is not empty")
}

/// Key lengths worth trying. Text XORed with a repeating key repeats itself at
/// multiples of the key length far more often than chance, so in a long sample only
/// the lengths that show this are kept.
fn candidate_lengths(sample: &[u8]) -> Vec<usize> {
    let longest = MAX_KEY_BYTES.min(sample.len() / MIN_BYTES_PER_KEY_BYTE);
    if sample.len() < 400 {
        return (2..=longest).collect();
    }
    let mut rated: Vec<(usize, f64)> = (2..=longest)
        .map(|length| {
            let pairs = sample.len() - length;
            let equal = (0..pairs).filter(|i| sample[*i] == sample[i + length]).count();
            (length, equal as f64 / pairs as f64)
        })
        .filter(|(_, rate)| *rate >= 0.03)
        .collect();
    rated.sort_by(|a, b| b.1.total_cmp(&a.1));
    rated.into_iter().take(3).map(|(length, _)| length).collect()
}

/// A key that is a shorter key repeated is cut down to that shorter key.
fn shortest_period(key: &[u8]) -> &[u8] {
    let period = (1..=key.len())
        .find(|period| key.len() % period == 0 && key.chunks(*period).all(|c| c == &key[..*period]))
        .unwrap_or(key.len());
    &key[..period]
}

/// The repeating key that turns the sample into the most readable text.
fn recover_key(sample: &[u8]) -> Option<(Vec<u8>, f64)> {
    let mut best: Option<(Vec<u8>, f64)> = None;
    for length in candidate_lengths(sample) {
        // English statistics find an English key with less text; the neutral ones
        // are there for everything else.
        for fit in [byte_fit, byte_fit_any_language] {
            let key: Vec<u8> =
                (0..length).map(|offset| column_key(sample, offset, length, fit)).collect();
            let key = shortest_period(&key);
            let Some(score) = readability(&apply(sample, key)) else { continue };
            if best.as_ref().is_none_or(|(_, best)| score > *best) {
                best = Some((key.to_vec(), score));
            }
        }
    }
    best
}

fn describe(key: &[u8]) -> XorConfig {
    if key.len() > 1 && key.iter().all(u8::is_ascii_graphic) {
        let key = String::from_utf8(key.to_vec()).expect("ASCII is UTF-8");
        XorConfig { key, key_format: KeyFormat::Text }
    } else {
        let key = hex_simd::encode_to_string(key, hex_simd::AsciiCase::Lower);
        XorConfig { key, key_format: KeyFormat::Hex }
    }
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

    /// Looks for the key, single-byte or repeating, that turns the start of the input
    /// into clearly readable text.
    fn detect(input: &[u8]) -> Option<Suggestion<XorConfig>> {
        let sample = &input[..input.len().min(SAMPLE_BYTES)];
        if sample.len() < 8 {
            return None;
        }
        let before = readability(sample).unwrap_or(0.0);
        let single = (1..=255u8)
            .filter_map(|key| Some((vec![key], readability(&apply(sample, &[key]))?)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        // Both are tried: with the wrong kind of key a text can still look half readable.
        let (key, after) = [single, recover_key(sample)]
            .into_iter()
            .flatten()
            .reduce(|best, other| if other.1 > best.1 { other } else { best })?;
        (after >= MIN_SCORE && after >= before + MIN_GAIN).then(|| {
            let config = describe(&key);
            let what = match key.as_slice() {
                [byte] => format!("the single byte 0x{byte:02x}"),
                _ => format!("the repeating {}-byte key \"{}\"", key.len(), config.key),
            };
            Suggestion {
                config,
                reason: format!(
                    "XOR with {what} gives readable text \
                     (readability goes from {before:.2} to {after:.2})"
                ),
            }
        })
    }

    fn run(config: &XorConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let key = parse_key(&config.key, config.key_format)?;
        Ok(apply(input, &key))
    }

    fn take_over(config: &XorConfig, input: Value) -> Result<Result<Value, OpError>, Value> {
        let Ok(key) = parse_key(&config.key, config.key_format) else { return Err(input) };
        let mut bytes = match input {
            Value::Bytes(bytes) => bytes,
            Value::Text(text) => text.into_bytes(),
            other => return Err(other),
        };
        for (byte, key) in bytes.iter_mut().zip(key.iter().cycle()) {
            *byte ^= key;
        }
        Ok(Ok(Value::Bytes(bytes)))
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

    const LONG: &[u8] = b"The secret message is hidden in the last file that you will find \
        here. Nobody has opened it for years, and the key was written on the back of a map.";

    #[test]
    fn detection_recovers_a_repeating_key() {
        for key in ["ICE", "nohut", "correcthorse"] {
            let encrypted = Xor::run(&config(key, KeyFormat::Text), LONG).unwrap();
            let suggestion = Xor::detect(&encrypted).unwrap_or_else(|| panic!("no key for {key}"));
            assert_eq!(suggestion.config, config(key, KeyFormat::Text));
            assert!(suggestion.reason.contains(key));
        }
    }

    #[test]
    fn detection_recovers_a_key_over_text_in_another_language() {
        let turkish = "Gizli mesaj, burada bulacağın son dosyanın içinde saklı duruyor. Kimse \
            yıllardır o dosyaya bakmadı ama anahtar eski bir haritanın arkasında yazılı.";
        let encrypted = Xor::run(&config("nohut", KeyFormat::Text), turkish.as_bytes()).unwrap();
        let suggestion = Xor::detect(&encrypted).expect("a key is suggested");
        assert_eq!(suggestion.config, config("nohut", KeyFormat::Text));
    }

    #[test]
    fn a_key_with_unprintable_bytes_is_given_as_hex() {
        let encrypted = Xor::run(&config("00ff10", KeyFormat::Hex), LONG).unwrap();
        assert_eq!(Xor::detect(&encrypted).unwrap().config, config("00ff10", KeyFormat::Hex));
    }

    #[test]
    fn a_long_key_is_found_in_a_long_input() {
        let text = LONG.repeat(12);
        let encrypted = Xor::run(&config("a much longer passphrase", KeyFormat::Text), &text).unwrap();
        assert_eq!(Xor::detect(&encrypted).unwrap().config.key.len(), 48);
    }

    #[test]
    fn random_looking_bytes_get_no_key() {
        let noise: Vec<u8> = (0..1500u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        assert!(Xor::detect(&noise).is_none());
    }

    #[test]
    fn empty_and_malformed_keys_are_config_errors() {
        let empty = Xor::run(&config("", KeyFormat::Text), b"x").unwrap_err();
        assert_eq!(empty.code, ErrorCode::InvalidConfig);
        let bad_hex = Xor::run(&config("zz", KeyFormat::Hex), b"x").unwrap_err();
        assert_eq!(bad_hex.code, ErrorCode::InvalidConfig);
    }
}
