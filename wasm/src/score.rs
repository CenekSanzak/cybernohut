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

/// How often each letter occurs in English text, in percent, from A to Z.
pub const ENGLISH_LETTER_PERCENT: [f64; 26] = [
    8.2, 1.5, 2.8, 4.3, 12.7, 2.2, 2.0, 6.1, 7.0, 0.15, 0.77, 4.0, 2.4, 6.7, 7.5, 1.9, 0.095,
    6.0, 6.3, 9.1, 2.8, 0.98, 2.4, 0.15, 2.0, 0.074,
];

const COMMON_WORDS: &[&str] = &[
    "the", "be", "to", "of", "and", "a", "in", "that", "have", "i", "it", "for", "not", "on",
    "with", "he", "as", "you", "do", "at", "this", "but", "his", "by", "from", "they", "we",
    "say", "her", "she", "or", "an", "will", "my", "one", "all", "would", "there", "their",
    "what", "so", "up", "out", "if", "about", "who", "get", "which", "go", "me", "is", "are",
    "was", "were", "has", "had", "can", "your", "no", "yes", "here", "hello", "world", "flag",
    "key", "secret", "password", "message", "data", "text", "file", "user", "name", "test",
];

/// Vowels of the Latin script, with the accented forms European languages use.
const VOWELS: &str = "aeiouàáâãäåæèéêëìíîïòóôõöøùúûüāăēęěīıōőœūůű";
/// Share of vowels among letters: natural languages sit near 40%, and a letter
/// substitution almost always moves a text well below 30%.
const VOWELS_TOO_FEW: f64 = 0.27;
const VOWELS_ENOUGH: f64 = 0.35;
const VOWELS_TOO_MANY: f64 = 0.65;
/// What prose in an unrecognized language earns of the part English can earn in full.
const ANY_LANGUAGE_CEILING: f64 = 0.8;

/// A word as any language writes it: letters only, apart from punctuation around it
/// and apostrophes or hyphens inside, in one case or capitalized.
fn is_wordlike(token: &str) -> bool {
    let core = token.trim_matches(|c: char| !c.is_alphanumeric());
    let mut letters = core.chars().filter(|c| !matches!(c, '\'' | '\u{2019}' | '-'));
    let Some(first) = letters.next() else { return false };
    let rest: Vec<char> = letters.collect();
    rest.len() < 20
        && first.is_alphabetic()
        && rest.iter().all(|c| c.is_alphabetic())
        && (rest.iter().all(|c| !c.is_uppercase())
            || (first.is_uppercase() && rest.iter().all(|c| !c.is_lowercase())))
}

/// How much the text is shaped like prose in some language, from 0 to 1, without
/// knowing which: separate words, and a natural share of vowels where the script has
/// them.
fn natural_language_fit(sample: &str) -> f64 {
    let tokens: Vec<&str> = sample.split_whitespace().collect();
    // One or two tokens are a word or a name, not prose.
    if tokens.len() < 3 {
        return 0.0;
    }
    let word_shape =
        tokens.iter().filter(|token| is_wordlike(token)).count() as f64 / tokens.len() as f64;

    let letters = sample.chars().filter(|c| c.is_alphabetic()).count() as f64;
    let latin: Vec<char> = sample
        .chars()
        .filter(|c| c.is_alphabetic() && (*c as u32) < 0x250)
        .flat_map(char::to_lowercase)
        .collect();
    // Other scripts mark vowels differently or not at all, so only word shape counts.
    let vowel_fit = if (latin.len() as f64) < letters / 2.0 {
        1.0
    } else {
        let share = latin.iter().filter(|c| VOWELS.contains(**c)).count() as f64
            / latin.len().max(1) as f64;
        let enough = (share - VOWELS_TOO_FEW) / (VOWELS_ENOUGH - VOWELS_TOO_FEW);
        let not_too_many = (VOWELS_TOO_MANY - share) / 0.1;
        enough.min(not_too_many).clamp(0.0, 1.0)
    };
    word_shape * vowel_fit
}

/// A few of the most common words of each language. Short function words are what
/// tells languages apart, and they are present in almost any sentence.
pub const COMMON_WORDS_BY_LANGUAGE: &[(&str, &[&str])] = &[
    ("English", &["the", "and", "of", "to", "in", "is", "that", "it", "for", "was", "with", "you", "are", "this", "not", "have", "be", "on"]),
    ("Turkish", &["bir", "ve", "bu", "da", "de", "için", "ile", "ne", "çok", "ama", "gibi", "daha", "var", "ben", "en", "kadar", "olarak", "mi", "değil", "her"]),
    ("German", &["der", "die", "und", "das", "ist", "nicht", "ein", "ich", "zu", "den", "mit", "sie", "auf", "für", "es", "eine", "dem", "von"]),
    ("French", &["le", "la", "les", "et", "des", "est", "un", "une", "du", "que", "dans", "pour", "pas", "qui", "il", "sur", "ne", "ce", "au"]),
    ("Spanish", &["el", "la", "de", "que", "y", "en", "los", "se", "del", "las", "por", "un", "para", "con", "no", "una", "su", "es", "lo"]),
    ("Italian", &["il", "di", "che", "e", "la", "per", "un", "non", "sono", "con", "una", "le", "si", "del", "della", "è", "gli", "anche"]),
    ("Portuguese", &["o", "de", "que", "e", "do", "da", "em", "um", "para", "não", "uma", "os", "com", "se", "na", "por", "mais", "as", "dos"]),
    ("Dutch", &["de", "het", "een", "van", "en", "is", "dat", "op", "te", "zijn", "voor", "niet", "met", "ik", "je", "aan", "ook", "als"]),
];

/// How often each letter from A to Z occurs in a language, in percent. Letters with
/// diacritics are left out, since the classic ciphers do not touch them.
pub const LETTER_PERCENT_BY_LANGUAGE: &[(&str, [f64; 26])] = &[
    ("English", ENGLISH_LETTER_PERCENT),
    ("Turkish", [
        11.92, 2.84, 0.96, 4.71, 8.91, 0.46, 1.25, 1.21, 8.60, 0.03, 4.68, 5.92, 3.75, 7.49,
        2.48, 0.89, 0.01, 6.72, 3.01, 3.31, 3.24, 0.96, 0.01, 0.01, 3.34, 1.50,
    ]),
    ("German", [
        6.5, 1.9, 3.1, 5.1, 16.4, 1.7, 3.0, 4.6, 6.6, 0.3, 1.4, 3.4, 2.5, 9.8, 2.6, 0.7, 0.02,
        7.0, 7.3, 6.2, 4.2, 0.8, 1.9, 0.03, 0.04, 1.1,
    ]),
    ("French", [
        7.6, 0.9, 3.3, 3.7, 14.7, 1.1, 0.9, 0.7, 7.5, 0.6, 0.07, 5.5, 3.0, 7.1, 5.8, 2.5, 1.4,
        6.7, 7.9, 7.2, 6.3, 1.8, 0.05, 0.4, 0.1, 0.3,
    ]),
    ("Spanish", [
        11.5, 2.2, 4.0, 5.0, 12.2, 0.7, 1.8, 0.7, 6.2, 0.5, 0.01, 5.0, 3.2, 6.7, 8.7, 2.5, 0.9,
        6.9, 8.0, 4.6, 2.9, 1.1, 0.02, 0.2, 1.0, 0.5,
    ]),
];

/// How many letters of the text are in common words of English or of the other
/// languages above. Unlike [`text_score`]
/// this keeps growing as more of a text becomes readable, which is what a search for
/// a key needs to tell a nearly right key from the right one.
pub fn common_word_letters(text: &str) -> usize {
    text.split(|c: char| !c.is_alphabetic())
        .filter(|word| !word.is_empty())
        .filter(|word| {
            let word = word.to_lowercase();
            COMMON_WORDS.contains(&word.as_str())
                // One-letter words of other languages match too easily by accident.
                || (word.chars().count() >= 2
                    && COMMON_WORDS_BY_LANGUAGE.iter().any(|(_, words)| words.contains(&word.as_str())))
        })
        .map(|word| word.chars().count())
        .sum()
}

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
/// English is recognized by its letter frequencies and common words. Prose in other
/// languages is recognized by its shape alone and tops out a little lower, at about
/// 0.9, so that an English reading wins when both are possible.
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

    let all_letters = sample.chars().filter(|c| c.is_alphabetic()).count() as f64;
    let letters = sample.chars().filter(char::is_ascii_alphabetic).count() as f64;
    let spaces = sample.chars().filter(|c| *c == ' ').count() as f64;
    let prose_shape = (all_letters + spaces) / total;
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

    // Stray letters between symbols spell "a" and "I" by accident. English evidence
    // counts in full only when at least half of the text is letters.
    let letter_share = (letters / total / 0.5).min(1.0);
    let english = (0.15 * frequency_fit + 0.3 * word_fit) * letter_share;
    let any_language = 0.45 * ANY_LANGUAGE_CEILING * natural_language_fit(&sample);

    0.2 * printable + 0.2 * prose_shape + 0.15 * spacing + english.max(any_language) + flag_bonus
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
    fn prose_in_other_languages_is_recognized_by_its_shape() {
        let turkish = "Gizli mesaj, burada bulacağın son dosyanın içinde saklı duruyor.";
        let german = "Die geheime Nachricht ist in der letzten Datei versteckt, die du findest.";
        let russian = "Секретное сообщение спрятано в последнем файле, который вы найдёте здесь.";
        for prose in [turkish, german, russian] {
            let score = text_score(prose);
            assert!(score > 0.8, "{prose}: {score}");
        }
        // The same sentence under ROT13 and as Base64.
        let rot13 = "Tvmyv zrfnw, ohenqn ohynpnğıa fba qbflnaıa vçvaqr fnxyı qhehlbe.";
        let base64 = "R2l6bGkgbWVzYWosIGJ1cmFkYSBidWxhY2HEn8SxbiBzb24gZG9zeWFuxLFuIGnDp2luZGU=";
        assert!(text_score(rot13) < text_score(turkish) - 0.15, "rot13 {}", text_score(rot13));
        assert!(text_score(base64) < text_score(turkish) - 0.4, "base64 {}", text_score(base64));
    }

    #[test]
    fn a_word_or_two_is_not_prose() {
        assert_eq!(natural_language_fit("deadbeef"), 0.0);
        assert_eq!(natural_language_fit("hello world"), 0.0);
        assert!(natural_language_fit("camelCase tOKEN x9y") < 0.1);
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
