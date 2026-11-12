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
}
