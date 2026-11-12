use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Suggestion, Tag};
use crate::score::text_score;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct CaesarConfig {
    /// Letters to shift by. 0 lists every shift instead.
    pub shift: i32,
    /// When listing every shift, keep only the candidates containing this text.
    pub known_part: String,
}

/// Shifts ASCII letters and leaves every other byte alone. Working on bytes is valid
/// because all bytes of a multi-byte UTF-8 character are above the ASCII range, so
/// they are never touched and the result is still UTF-8.
fn shift_text(text: &str, shift: i32) -> String {
    let shift = shift.rem_euclid(26) as u8;
    let mut bytes = text.as_bytes().to_vec();
    for byte in &mut bytes {
        let base = match *byte {
            b'a'..=b'z' => b'a',
            b'A'..=b'Z' => b'A',
            _ => continue,
        };
        *byte = (*byte - base + shift) % 26 + base;
    }
    String::from_utf8(bytes).expect("shifting ASCII letters keeps the text valid UTF-8")
}

/// How much better a rewritten text has to score before it is worth suggesting, and
/// how readable it has to be in absolute terms.
const MIN_GAIN: f64 = 0.15;
const MIN_SCORE: f64 = 0.6;

/// Substitution ciphers need some letters to say anything about.
fn enough_letters(text: &str) -> bool {
    text.bytes().filter(u8::is_ascii_alphabetic).take(8).count() == 8
}

/// The start of the text, which is all that is tried when looking for a shift.
fn sample(text: &str) -> &str {
    match text.char_indices().nth(2000) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

/// The shift that makes the text most readable, if any makes it clearly readable.
fn best_shift(text: &str) -> Option<(i32, f64, f64)> {
    if !enough_letters(text) {
        return None;
    }
    let sample = sample(text);
    let before = text_score(sample);
    let (shift, after) = (1..26)
        .map(|shift| (shift, text_score(&shift_text(sample, shift))))
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    (after >= MIN_SCORE && after >= before + MIN_GAIN).then_some((shift, before, after))
}

fn readability(before: f64, after: f64) -> String {
    format!("readability goes from {before:.2} to {after:.2}")
}

pub struct CaesarCipher;

impl Op for CaesarCipher {
    const ID: &'static str = "caesar-cipher";
    const NAME: &'static str = "Caesar Cipher";
    const DESCRIPTION: &'static str =
        "Shifts letters by a fixed amount. With shift 0 it lists all 26 shifts.";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Caesar_cipher");
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::SubstitutionCipher];

    type Config = CaesarConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("shift", "Shift", ParamKind::Int { min: Some(-25), max: Some(25) }),
            ParamSpec::new("knownPart", "Known plaintext", ParamKind::Text),
        ]
    }

    fn detect(input: &str) -> Option<Suggestion<CaesarConfig>> {
        let (shift, before, after) = best_shift(input)?;
        // A shift of 13 is ROT13, which offers itself.
        (shift != 13).then(|| Suggestion {
            config: CaesarConfig { shift, known_part: String::new() },
            reason: format!(
                "Shifting letters by {shift} gives readable text ({})",
                readability(before, after)
            ),
        })
    }

    fn run(config: &CaesarConfig, input: &str) -> Result<String, OpError> {
        if config.shift != 0 {
            return Ok(shift_text(input, config.shift));
        }
        let known = config.known_part.to_lowercase();
        let candidates: Vec<String> = (0..26)
            .map(|shift| (shift, shift_text(input, shift)))
            .filter(|(_, text)| known.is_empty() || text.to_lowercase().contains(&known))
            .map(|(shift, text)| format!("Shift {shift}: {text}"))
            .collect();
        Ok(candidates.join("\n"))
    }
}

pub struct Rot13;

impl Op for Rot13 {
    const ID: &'static str = "rot13";
    const NAME: &'static str = "ROT13";
    const DESCRIPTION: &'static str = "Caesar cipher with a shift of 13; applying it twice undoes it";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/ROT13");
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::SubstitutionCipher];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let (shift, before, after) = best_shift(input)?;
        (shift == 13).then(|| {
            Suggestion::new(format!(
                "Rotating letters by 13 gives readable text ({})",
                readability(before, after)
            ))
        })
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(shift_text(input, 13))
    }
}

/// Maps A to Z, B to Y and so on. Applying it twice undoes it.
pub struct Atbash;

impl Op for Atbash {
    const ID: &'static str = "atbash";
    const NAME: &'static str = "Atbash";
    const DESCRIPTION: &'static str = "Mirrors the alphabet: A becomes Z, B becomes Y";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Atbash");
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::SubstitutionCipher];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        if !enough_letters(input) {
            return None;
        }
        let sample = sample(input);
        let before = text_score(sample);
        let after = text_score(&Self::run(&NoConfig {}, sample).ok()?);
        (after >= MIN_SCORE && after >= before + MIN_GAIN).then(|| {
            Suggestion::new(format!(
                "Mirroring the alphabet gives readable text ({})",
                readability(before, after)
            ))
        })
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let mut bytes = input.as_bytes().to_vec();
        for byte in &mut bytes {
            *byte = match *byte {
                b'a'..=b'z' => b'z' - (*byte - b'a'),
                b'A'..=b'Z' => b'Z' - (*byte - b'A'),
                other => other,
            };
        }
        Ok(String::from_utf8(bytes).expect("mirroring ASCII letters keeps the text valid UTF-8"))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct VigenereConfig {
    /// Only its letters are used.
    pub key: String,
    pub decrypt: bool,
}

pub struct Vigenere;

impl Op for Vigenere {
    const ID: &'static str = "vigenere";
    const NAME: &'static str = "Vigenère Cipher";
    const DESCRIPTION: &'static str =
        "Shifts each letter by the matching letter of a repeating key";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Vigen%C3%A8re_cipher");
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::SubstitutionCipher];

    type Config = VigenereConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("key", "Key", ParamKind::Text),
            ParamSpec::new("decrypt", "Decrypt", ParamKind::Bool),
        ]
    }

    fn run(config: &VigenereConfig, input: &str) -> Result<String, OpError> {
        let shifts: Vec<u8> = config
            .key
            .bytes()
            .filter(u8::is_ascii_alphabetic)
            .map(|letter| letter.to_ascii_lowercase() - b'a')
            .collect();
        if shifts.is_empty() {
            return Err(OpError::invalid_config("Enter a key with at least one letter"));
        }
        let mut bytes = input.as_bytes().to_vec();
        // The key advances only on letters, so punctuation does not shift the alignment.
        let mut position = 0;
        for byte in &mut bytes {
            let base = match *byte {
                b'a'..=b'z' => b'a',
                b'A'..=b'Z' => b'A',
                _ => continue,
            };
            let shift = shifts[position % shifts.len()];
            let shift = if config.decrypt { 26 - shift } else { shift };
            *byte = (*byte - base + shift) % 26 + base;
            position += 1;
        }
        Ok(String::from_utf8(bytes).expect("shifting ASCII letters keeps the text valid UTF-8"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(shift: i32, known_part: &str) -> CaesarConfig {
        CaesarConfig { shift, known_part: known_part.into() }
    }

    #[test]
    fn shifts_letters_and_leaves_the_rest() {
        assert_eq!(CaesarCipher::run(&config(13, ""), "Hello, World!").unwrap(), "Uryyb, Jbeyq!");
        assert_eq!(CaesarCipher::run(&config(-13, ""), "Uryyb, Jbeyq!").unwrap(), "Hello, World!");
    }

    #[test]
    fn non_ascii_text_passes_through_untouched() {
        assert_eq!(CaesarCipher::run(&config(1, ""), "çay 🌰 abz").unwrap(), "çbz 🌰 bca");
    }

    #[test]
    fn rot13_is_its_own_inverse() {
        let once = Rot13::run(&NoConfig {}, "Hello").unwrap();
        assert_eq!(once, "Uryyb");
        assert_eq!(Rot13::run(&NoConfig {}, &once).unwrap(), "Hello");
    }

    #[test]
    fn shift_zero_lists_every_shift() {
        let output = CaesarCipher::run(&config(0, ""), "abc").unwrap();
        assert_eq!(output.lines().count(), 26);
        assert!(output.contains("Shift 1: bcd"));
    }

    #[test]
    fn known_part_narrows_the_list() {
        let output = CaesarCipher::run(&config(0, "hello"), "Uryyb").unwrap();
        assert_eq!(output, "Shift 13: Hello");
    }

    const PROSE: &str = "The secret message is hidden in the last file that you will find here.";

    #[test]
    fn detection_finds_the_shift_that_restores_prose() {
        let shifted = CaesarCipher::run(&config(7, ""), PROSE).unwrap();
        let suggestion = CaesarCipher::detect(&shifted).expect("a shift is suggested");
        assert_eq!(suggestion.config.shift, 19);
        assert_eq!(CaesarCipher::run(&suggestion.config, &shifted).unwrap(), PROSE);
        assert!(Rot13::detect(&shifted).is_none());
    }

    #[test]
    fn rot13_and_atbash_offer_themselves_and_caesar_steps_aside() {
        let rotated = Rot13::run(&NoConfig {}, PROSE).unwrap();
        assert!(Rot13::detect(&rotated).is_some());
        assert!(CaesarCipher::detect(&rotated).is_none());
        let mirrored = Atbash::run(&NoConfig {}, PROSE).unwrap();
        assert!(Atbash::detect(&mirrored).is_some());
    }

    #[test]
    fn readable_text_and_short_text_are_left_alone() {
        assert!(CaesarCipher::detect(PROSE).is_none());
        assert!(Rot13::detect(PROSE).is_none());
        assert!(Atbash::detect(PROSE).is_none());
        assert!(CaesarCipher::detect("uryyb").is_none(), "too few letters to judge");
    }

    #[test]
    fn atbash_mirrors_and_undoes_itself() {
        let once = Atbash::run(&NoConfig {}, "Hello, Nohut!").unwrap();
        assert_eq!(once, "Svool, Mlsfg!");
        assert_eq!(Atbash::run(&NoConfig {}, &once).unwrap(), "Hello, Nohut!");
    }

    #[test]
    fn vigenere_matches_the_textbook_example_and_decrypts() {
        let encrypt = VigenereConfig { key: "LEMON".into(), decrypt: false };
        let decrypt = VigenereConfig { key: "LEMON".into(), decrypt: true };
        assert_eq!(Vigenere::run(&encrypt, "ATTACKATDAWN").unwrap(), "LXFOPVEFRNHR");
        assert_eq!(Vigenere::run(&decrypt, "LXFOPVEFRNHR").unwrap(), "ATTACKATDAWN");
    }

    #[test]
    fn vigenere_keeps_case_and_skips_punctuation_without_using_up_the_key() {
        let config = VigenereConfig { key: "b-c".into(), decrypt: false };
        assert_eq!(Vigenere::run(&config, "a, A a!").unwrap(), "b, C b!");
    }

    #[test]
    fn vigenere_needs_a_key_with_letters() {
        let config = VigenereConfig { key: "123".into(), decrypt: false };
        assert!(Vigenere::run(&config, "abc").is_err());
    }
}
