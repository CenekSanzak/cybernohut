use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Tag};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct CaesarConfig {
    /// Letters to shift by. 0 lists every shift instead.
    pub shift: i32,
    /// When listing every shift, keep only the candidates containing this text.
    pub known_part: String,
}

fn shift_char(c: char, shift: i32) -> char {
    if !c.is_ascii_alphabetic() {
        return c;
    }
    let base = if c.is_ascii_uppercase() { b'A' } else { b'a' } as i32;
    ((c as i32 - base + shift).rem_euclid(26) + base) as u8 as char
}

fn shift_text(text: &str, shift: i32) -> String {
    text.chars().map(|c| shift_char(c, shift)).collect()
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

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(shift_text(input, 13))
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
}
