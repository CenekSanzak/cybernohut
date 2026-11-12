//! How much a value looks like something a person would want to read.
//!
//! Auto mode uses this to tell a decoding step that reveals plaintext from one that
//! merely succeeds: Base64 decodes plenty of ordinary words into garbage.

use crate::core::Value;
use crate::ops::analysis::signature;

/// Only the start of a value is scored; that is enough to judge it.
const SAMPLE_CHARS: usize = 4096;

/// The twelve most frequent English letters. They make up roughly 80% of English
/// text and 46% of uniformly random letters.
const FREQUENT_LETTERS: &[u8] = b"etaoinshrdlu";
const FREQUENT_IN_RANDOM: f64 = 12.0 / 26.0;
const FREQUENT_IN_ENGLISH: f64 = 0.78;

const COMMON_WORDS: &[&str] = &[
    "the", "be", "to", "of", "and", "a", "in", "that", "have", "i", "it", "for", "not", "on",
    "with", "he", "as", "you", "do", "at", "this", "but", "his", "by", "from", "they", "we",
    "say", "her", "she", "or", "an", "will", "my", "one", "all", "would", "there", "their",
    "what", "so", "up", "out", "if", "about", "who", "get", "which", "go", "me", "is", "are",
    "was", "were", "has", "had", "can", "your", "no", "yes", "here", "hello", "world", "flag",
    "key", "secret", "password", "message", "data", "text", "file", "user", "name", "test",
];

/// Whether the text contains something shaped like `ctf{...}`.
fn has_flag(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.iter().enumerate().any(|(open, byte)| {
        *byte == b'{'
            && open >= 2
            && bytes[open - 2..open].iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
            && bytes[open + 1..]
                .iter()
                .position(|b| matches!(b, b'}' | b'{' | b'\n'))
                .is_some_and(|close| close >= 3 && bytes[open + 1 + close] == b'}')
    })
}

/// A score from 0 (noise) to 1 (ordinary English prose), plus a bonus for a flag.
///
/// The bonus is deliberately not capped at 1. The shape of a flag survives a letter
/// substitution, so ciphertext earns it too; capping would hide the difference between
/// such ciphertext and its plaintext, which is exactly what auto mode has to see.
pub fn text_score(text: &str) -> f64 {
    let sample: String = text.chars().take(SAMPLE_CHARS).collect();
    let total = sample.chars().count();
    if total == 0 {
        return 0.0;
    }
    let total = total as f64;

    let printable = sample
        .chars()
        .filter(|c| c.is_ascii_graphic() || matches!(c, ' ' | '\n' | '\r' | '\t') || c.is_alphabetic())
        .count() as f64
        / total;
    // Control characters mean this is not text, whatever else it contains.
    if printable < 0.9 {
        return printable * 0.3;
    }

    let letters = sample.chars().filter(char::is_ascii_alphabetic).count() as f64;
    let spaces = sample.chars().filter(|c| *c == ' ').count() as f64;
    let prose_shape = (letters + spaces) / total;
    // About one character in six of English prose is a space. Text with none, such as
    // percent-encoded prose, is not there yet however many words it contains.
    let spacing = (spaces / total / 0.1).min(1.0);

    let frequent = sample
        .bytes()
        .filter(|b| FREQUENT_LETTERS.contains(&b.to_ascii_lowercase()))
        .count() as f64;
    let frequency_fit = if letters > 0.0 {
        ((frequent / letters - FREQUENT_IN_RANDOM) / (FREQUENT_IN_ENGLISH - FREQUENT_IN_RANDOM))
            .clamp(0.0, 1.0)
    } else {
        0.0
    };

    let words: Vec<String> = sample
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let known = words.iter().filter(|word| COMMON_WORDS.contains(&word.as_str())).count() as f64;
    // In prose about a third of the words are on a short common-word list.
    let word_fit = if words.is_empty() { 0.0 } else { (known / words.len() as f64 * 3.0).min(1.0) };

    let flag_bonus = if has_flag(&sample) { 0.3 } else { 0.0 };

    0.2 * printable
        + 0.2 * prose_shape
        + 0.15 * spacing
        + 0.15 * frequency_fit
        + 0.3 * word_fit
        + flag_bonus
}

pub fn value_score(value: &Value) -> f64 {
    match value {
        Value::Text(text) => text_score(text),
        Value::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => text_score(text),
            // A recognizable file is a good place to have arrived, even if unreadable.
            Err(_) if signature(bytes).is_some() => 0.9,
            Err(_) => 0.0,
        },
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROSE: &str = "The secret message is hidden in the last file that you will find here.";

    #[test]
    fn prose_scores_far_above_its_encodings() {
        let prose = text_score(PROSE);
        let base64 = text_score("VGhlIHNlY3JldCBtZXNzYWdlIGlzIGhpZGRlbiBpbiB0aGUgbGFzdCBmaWxl");
        let hex = text_score("54686520736563726574206d657373616765206973206869646465");
        let url = text_score("The%20secret%20message%20is%20hidden%20in%20the%20last%20file");
        let rot13 = text_score("Gur frperg zrffntr vf uvqqra va gur ynfg svyr gung lbh jvyy svaq urer.");
        assert!(prose > 0.9, "prose {prose}");
        for (name, encoded) in [("base64", base64), ("hex", hex), ("rot13", rot13)] {
            assert!(encoded < prose - 0.3, "{name} {encoded} vs prose {prose}");
        }
        assert!(url < prose - 0.12, "url {url} vs prose {prose}");
    }

    #[test]
    fn control_characters_sink_the_score() {
        assert!(text_score("ab\u{1}\u{2}\u{3}cd\u{4}") < 0.2);
        assert_eq!(text_score(""), 0.0);
    }

    #[test]
    fn a_flag_is_recognized_and_rewarded() {
        assert!(has_flag("here: ctf{n0hut_power}"));
        assert!(!has_flag("fn main() {\n}"));
        assert!(!has_flag("{}"));
        assert!(text_score("ctf{n0hut_power}") > text_score("ctf_n0hut_power_") + 0.2);
    }

    #[test]
    fn bytes_score_as_text_or_by_signature() {
        assert!(value_score(&Value::Bytes(PROSE.as_bytes().to_vec())) > 0.9);
        assert_eq!(value_score(&Value::Bytes(b"\x89PNG\r\n\x1a\n\x00\xff".to_vec())), 0.9);
        assert_eq!(value_score(&Value::Bytes(vec![0xff, 0xfe + 1, 0x80])), 0.0);
    }
}
