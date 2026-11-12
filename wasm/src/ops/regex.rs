use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

const LINK: &str = "https://docs.rs/regex-lite/latest/regex_lite/#syntax";

fn compile(pattern: &str) -> Result<Regex, OpError> {
    if pattern.is_empty() {
        return Err(OpError::invalid_config("Enter a pattern"));
    }
    Regex::new(pattern).map_err(|e| OpError::invalid_config(format!("Invalid pattern: {e}")))
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct RegexExtractConfig {
    pub pattern: String,
    /// Which capture group to keep. 0 is the whole match.
    pub group: i32,
}

pub struct RegexExtract;

impl Op for RegexExtract {
    const ID: &'static str = "regex-extract";
    const NAME: &'static str = "Regex Extract";
    const DESCRIPTION: &'static str = "Lists every match of a regular expression";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = RegexExtractConfig;
    type In<'a> = &'a str;
    type Out = Vec<String>;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("pattern", "Pattern", ParamKind::Text),
            ParamSpec::new("group", "Capture group", ParamKind::Int { min: Some(0), max: None }),
        ]
    }

    fn run(config: &RegexExtractConfig, input: &str) -> Result<Vec<String>, OpError> {
        let regex = compile(&config.pattern)?;
        let group = usize::try_from(config.group).unwrap_or(usize::MAX);
        if group >= regex.captures_len() {
            return Err(OpError::invalid_config(format!(
                "The pattern has no capture group {}",
                config.group
            )));
        }
        Ok(regex
            .captures_iter(input)
            // A group that did not take part in a match contributes nothing.
            .filter_map(|captures| captures.get(group).map(|found| found.as_str().to_owned()))
            .collect())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct RegexReplaceConfig {
    pub pattern: String,
    /// May refer to capture groups as `$1` or `${name}`.
    pub replacement: String,
}

pub struct RegexReplace;

impl Op for RegexReplace {
    const ID: &'static str = "regex-replace";
    const NAME: &'static str = "Regex Replace";
    const DESCRIPTION: &'static str =
        "Replaces every match of a regular expression; $1 refers to a capture group";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = RegexReplaceConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("pattern", "Pattern", ParamKind::Text),
            ParamSpec::new("replacement", "Replace with", ParamKind::Text),
        ]
    }

    fn run(config: &RegexReplaceConfig, input: &str) -> Result<String, OpError> {
        let regex = compile(&config.pattern)?;
        Ok(regex.replace_all(input, config.replacement.as_str()).into_owned())
    }
}

/// Patterns worth having at hand when digging through text. They are meant to find
/// candidates, not to validate them.
const PATTERNS: &[(&str, &str)] = &[
    ("flags", r"[A-Za-z0-9_]{2,}\{[^{}\s]{3,}\}"),
    ("urls", r"[a-zA-Z][a-zA-Z0-9+.-]*://[^\s<>'\x22)]+"),
    ("emails", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"),
    ("ipv4 addresses", r"\b(?:\d{1,3}\.){3}\d{1,3}\b"),
    ("domains", r"\b(?:[a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}\b"),
    ("hashes (md5, sha1, sha256)", r"\b(?:[0-9a-fA-F]{64}|[0-9a-fA-F]{40}|[0-9a-fA-F]{32})\b"),
    ("base64 strings", r"(?:[A-Za-z0-9+/]{4}){4,}(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?"),
    ("hex strings", r"\b(?:[0-9a-fA-F]{2}){8,}\b"),
    ("uuids", r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b"),
    ("numbers", r"-?\b\d+(?:\.\d+)?\b"),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct ExtractPatternConfig {
    /// What to look for.
    pub find: String,
    /// List each distinct match once.
    pub unique: bool,
}

impl Default for ExtractPatternConfig {
    fn default() -> Self {
        ExtractPatternConfig { find: PATTERNS[0].0.into(), unique: true }
    }
}

pub struct ExtractPattern;

impl Op for ExtractPattern {
    const ID: &'static str = "extract-pattern";
    const NAME: &'static str = "Extract Pattern";
    const DESCRIPTION: &'static str =
        "Lists flags, URLs, email or IP addresses, hashes and other common patterns";
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::Analysis];

    type Config = ExtractPatternConfig;
    type In<'a> = &'a str;
    type Out = Vec<String>;

    fn params() -> Vec<ParamSpec> {
        let options = PATTERNS.iter().map(|(name, _)| (*name).into()).collect();
        vec![
            ParamSpec::new("find", "Find", ParamKind::Choice { options }),
            ParamSpec::new("unique", "Only once each", ParamKind::Bool),
        ]
    }

    fn run(config: &ExtractPatternConfig, input: &str) -> Result<Vec<String>, OpError> {
        let (_, pattern) = PATTERNS
            .iter()
            .find(|(name, _)| *name == config.find)
            .ok_or_else(|| OpError::invalid_config(format!("There is no pattern \"{}\"", config.find)))?;
        let regex = Regex::new(pattern).expect("the built-in patterns compile");
        let mut found: Vec<String> = regex.find_iter(input).map(|m| m.as_str().to_owned()).collect();
        if config.unique {
            let mut seen = std::collections::HashSet::new();
            found.retain(|item| seen.insert(item.clone()));
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    fn extract(pattern: &str, group: i32, input: &str) -> Result<Vec<String>, OpError> {
        RegexExtract::run(&RegexExtractConfig { pattern: pattern.into(), group }, input)
    }

    #[test]
    fn extracts_whole_matches_or_a_group() {
        let input = "flag{one} and flag{two}";
        assert_eq!(extract(r"flag\{(\w+)\}", 0, input).unwrap(), ["flag{one}", "flag{two}"]);
        assert_eq!(extract(r"flag\{(\w+)\}", 1, input).unwrap(), ["one", "two"]);
        assert!(extract(r"\d+", 0, input).unwrap().is_empty());
    }

    #[test]
    fn bad_patterns_and_groups_are_config_errors() {
        for (pattern, group) in [("", 0), ("(", 0), ("a", 1), ("a", -1)] {
            let error = extract(pattern, group, "a").unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidConfig, "{pattern:?} group {group}");
        }
    }

    #[test]
    fn replaces_with_group_references() {
        let config = RegexReplaceConfig { pattern: r"(\w+)@(\w+)".into(), replacement: "$2 at $1".into() };
        assert_eq!(RegexReplace::run(&config, "a@b, c@d").unwrap(), "b at a, d at c");
    }

    #[test]
    fn every_built_in_pattern_compiles_and_finds_its_example() {
        let text = "Mail root@example.com or see https://nohut.dev/a?b=1 (10.0.0.7). \
            ctf{fl4g_here} d41d8cd98f00b204e9800998ecf8427e \
            550e8400-e29b-41d4-a716-446655440000 VGhlIHNlY3JldCBtZXNzYWdl -12.5";
        let find = |name: &str| {
            let config = ExtractPatternConfig { find: name.into(), unique: true };
            ExtractPattern::run(&config, text).unwrap()
        };
        assert_eq!(find("flags"), ["ctf{fl4g_here}"]);
        assert_eq!(find("urls"), ["https://nohut.dev/a?b=1"]);
        assert_eq!(find("emails"), ["root@example.com"]);
        assert_eq!(find("ipv4 addresses"), ["10.0.0.7"]);
        assert!(find("domains").contains(&"example.com".to_owned()));
        assert_eq!(find("hashes (md5, sha1, sha256)"), ["d41d8cd98f00b204e9800998ecf8427e"]);
        assert_eq!(find("uuids"), ["550e8400-e29b-41d4-a716-446655440000"]);
        assert!(find("base64 strings").contains(&"VGhlIHNlY3JldCBtZXNzYWdl".to_owned()));
        assert!(find("hex strings").contains(&"d41d8cd98f00b204e9800998ecf8427e".to_owned()));
        assert!(find("numbers").contains(&"-12.5".to_owned()));
        for (name, _) in PATTERNS {
            find(name);
        }
    }

    #[test]
    fn duplicates_are_kept_only_when_asked() {
        let all = ExtractPatternConfig { find: "numbers".into(), unique: false };
        assert_eq!(ExtractPattern::run(&all, "7 7 8").unwrap(), ["7", "7", "8"]);
        let once = ExtractPatternConfig { find: "numbers".into(), unique: true };
        assert_eq!(ExtractPattern::run(&once, "7 7 8").unwrap(), ["7", "8"]);
    }
}
