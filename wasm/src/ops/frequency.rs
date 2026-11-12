//! Counting what a text is made of: the first thing to look at in a classic cipher.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Tag};
use crate::score::COMMON_WORDS_BY_LANGUAGE as LANGUAGES;

const BAR_WIDTH: f64 = 30.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum Counted {
    /// Letters only, upper and lower case counted together.
    #[default]
    Letters,
    /// Every character, as written.
    Characters,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct FrequencyConfig {
    pub count: Counted,
}

pub struct CharacterFrequency;

impl Op for CharacterFrequency {
    const ID: &'static str = "character-frequency";
    const NAME: &'static str = "Character Frequency";
    const DESCRIPTION: &'static str =
        "Counts how often each letter or character occurs, most frequent first";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Frequency_analysis");
    const TAGS: &'static [Tag] = &[Tag::Analysis];

    type Config = FrequencyConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        let options = ["letters", "characters"].map(String::from).to_vec();
        vec![ParamSpec::new("count", "Count", ParamKind::Choice { options })]
    }

    fn run(config: &FrequencyConfig, input: &str) -> Result<String, OpError> {
        let mut counts: HashMap<char, usize> = HashMap::new();
        for c in input.chars() {
            match config.count {
                Counted::Letters if c.is_alphabetic() => {
                    for lower in c.to_lowercase() {
                        *counts.entry(lower).or_default() += 1;
                    }
                }
                Counted::Letters => {}
                Counted::Characters => *counts.entry(c).or_default() += 1,
            }
        }
        let total: usize = counts.values().sum();
        if total == 0 {
            return Ok("Nothing to count".into());
        }
        let mut rows: Vec<(char, usize)> = counts.into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        // The chance that two characters picked at random are the same. It survives a
        // letter substitution, so it tells a substitution cipher (high, like the
        // language) from a polyalphabetic one or random data (low).
        let pairs = total as f64 * (total as f64 - 1.0);
        let coincidence = if pairs > 0.0 {
            rows.iter().map(|(_, n)| *n as f64 * (*n as f64 - 1.0)).sum::<f64>() / pairs
        } else {
            0.0
        };
        let what = if config.count == Counted::Letters { "letters" } else { "characters" };
        let mut output = format!(
            "{total} {what}, {} distinct. Index of coincidence {coincidence:.4}",
            rows.len()
        );
        if config.count == Counted::Letters {
            output.push_str(" (English text is about 0.067, random letters 0.038)");
        }
        output.push('\n');

        let most = rows[0].1 as f64;
        for (c, count) in rows {
            let shown = match c {
                ' ' => "space".to_owned(),
                '\n' => "\\n".to_owned(),
                '\r' => "\\r".to_owned(),
                '\t' => "\\t".to_owned(),
                c if c.is_control() => format!("U+{:04X}", c as u32),
                c => c.to_string(),
            };
            let percent = count as f64 * 100.0 / total as f64;
            let bar = "█".repeat(((count as f64 / most) * BAR_WIDTH).round().max(1.0) as usize);
            output.push_str(&format!("\n{shown:<6}{count:>8}  {percent:>5.1}%  {bar}"));
        }
        Ok(output)
    }
}

/// The share of words that have to be common words before a language is named.
const MIN_SHARE: f64 = 0.12;

pub struct DetectLanguage;

impl Op for DetectLanguage {
    const ID: &'static str = "detect-language";
    const NAME: &'static str = "Detect Language";
    const DESCRIPTION: &'static str =
        "Guesses which of eight European languages a text is written in, from its common words";
    const TAGS: &'static [Tag] = &[Tag::Analysis];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let words: Vec<String> = input
            .split(|c: char| !c.is_alphabetic())
            .filter(|word| !word.is_empty())
            .take(5000)
            .map(str::to_lowercase)
            .collect();
        if words.len() < 4 {
            return Ok("Too little text to tell".into());
        }
        let mut shares: Vec<(&str, f64)> = LANGUAGES
            .iter()
            .map(|(name, common)| {
                let hits = words.iter().filter(|word| common.contains(&word.as_str())).count();
                (*name, hits as f64 / words.len() as f64)
            })
            .collect();
        shares.sort_by(|a, b| b.1.total_cmp(&a.1));
        let (best, share) = shares[0];
        let (runner_up, second) = shares[1];
        Ok(if share < MIN_SHARE {
            "No language recognized. The text may be encoded, encrypted or in another language".into()
        } else if second > share * 0.8 {
            format!("{best} or {runner_up} ({:.0}% and {:.0}% common words)", share * 100.0, second * 100.0)
        } else {
            format!("{best} ({:.0}% of the words are common {best} words)", share * 100.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_counted_case_insensitively_and_sorted() {
        let table = CharacterFrequency::run(&FrequencyConfig::default(), "Aa bb\nB c!").unwrap();
        let lines: Vec<&str> = table.lines().collect();
        assert!(lines[0].starts_with("6 letters, 3 distinct."));
        assert!(lines[2].starts_with("b            3   50.0%  ██████████████████████████████"));
        assert!(lines[3].starts_with("a            2   33.3%"));
        assert!(lines[4].starts_with("c            1   16.7%"));
    }

    #[test]
    fn characters_include_spaces_and_line_breaks() {
        let config = FrequencyConfig { count: Counted::Characters };
        let table = CharacterFrequency::run(&config, "a a\n").unwrap();
        assert!(table.contains("space"));
        assert!(table.contains("\\n"));
        assert_eq!(CharacterFrequency::run(&config, "").unwrap(), "Nothing to count");
    }

    #[test]
    fn the_index_of_coincidence_separates_prose_from_a_flat_distribution() {
        let ioc = |text: &str| -> f64 {
            let table = CharacterFrequency::run(&FrequencyConfig::default(), text).unwrap();
            let start = table.find("coincidence ").unwrap() + "coincidence ".len();
            table[start..start + 6].parse().unwrap()
        };
        let prose = "the secret message is hidden in the last file that you will find here \
            nobody has opened it for years and the key was written on the back of a map";
        assert!(ioc(prose) > 0.06, "{}", ioc(prose));
        assert!(ioc(&"abcdefghijklmnopqrstuvwxyz".repeat(10)) < 0.04);
    }

    #[test]
    fn languages_are_told_apart() {
        let detect = |text: &str| DetectLanguage::run(&NoConfig {}, text).unwrap();
        for (text, language) in [
            ("The message is hidden in the last file that you will find.", "English"),
            ("Gizli mesaj bu dosyada ve bir anahtar ile daha da güvenli.", "Turkish"),
            ("Die Nachricht ist in der letzten Datei und nicht auf dem Tisch.", "German"),
            ("Le message est dans le dernier fichier et il ne faut pas le lire.", "French"),
            ("El mensaje se esconde en los archivos que no son para todos.", "Spanish"),
        ] {
            assert!(detect(text).starts_with(language), "{text}: {}", detect(text));
        }
        assert!(detect("Gur frperg zrffntr vf uvqqra va gur ynfg svyr").starts_with("No language"));
        assert_eq!(detect("hi"), "Too little text to tell");
    }
}
