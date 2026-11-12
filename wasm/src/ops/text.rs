use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Suggestion, Tag, Value};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct Utf8DecodeConfig {
    /// Replace invalid sequences with U+FFFD instead of failing.
    pub lossy: bool,
}

/// The explicit bytes-to-text step. It exists as an operation because, unlike text to
/// bytes, it can fail, and a silent lossy conversion corrupts binary data.
pub struct Utf8Decode;

impl Op for Utf8Decode {
    const ID: &'static str = "utf8-decode";
    const NAME: &'static str = "Decode Text (UTF-8)";
    const DESCRIPTION: &'static str = "Interprets raw bytes as UTF-8 text";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/UTF-8");
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::Encoding];

    type Config = Utf8DecodeConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new("lossy", "Replace invalid bytes", ParamKind::Bool)]
    }

    fn detect(input: &[u8]) -> Option<Suggestion<Utf8DecodeConfig>> {
        (!input.is_empty() && std::str::from_utf8(input).is_ok())
            .then(|| Suggestion::new("The bytes are valid UTF-8 text"))
    }

    fn run(config: &Utf8DecodeConfig, input: &[u8]) -> Result<String, OpError> {
        if config.lossy {
            return Ok(String::from_utf8_lossy(input).into_owned());
        }
        std::str::from_utf8(input).map(str::to_owned).map_err(|e| {
            OpError::invalid_input(format!(
                "Not valid UTF-8 at byte {}. Turn on \"Replace invalid bytes\" to decode anyway.",
                e.valid_up_to()
            ))
        })
    }

    /// Valid bytes become the text without being copied.
    fn take_over(_: &Utf8DecodeConfig, input: Value) -> Result<Result<Value, OpError>, Value> {
        match input {
            Value::Bytes(bytes) => match String::from_utf8(bytes) {
                Ok(text) => Ok(Ok(Value::Text(text))),
                Err(error) => Err(Value::Bytes(error.into_bytes())),
            },
            other => Err(other),
        }
    }
}

macro_rules! text_op {
    ($name:ident, $id:literal, $label:literal, $description:literal, $run:expr) => {
        text_op!($name, $id, $label, $description, $run, Err);
    };
    // With a last argument: how to rewrite text the engine hands over, where it lies.
    // It returns the text untouched when that is not possible.
    ($name:ident, $id:literal, $label:literal, $description:literal, $run:expr, $in_place:expr) => {
        pub struct $name;

        impl Op for $name {
            const ID: &'static str = $id;
            const NAME: &'static str = $label;
            const DESCRIPTION: &'static str = $description;
            const TAGS: &'static [Tag] = &[Tag::Text];

            type Config = NoConfig;
            type In<'a> = &'a str;
            type Out = String;

            fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
                let run: fn(&str) -> String = $run;
                Ok(run(input))
            }

            fn take_over(_: &NoConfig, input: Value) -> Result<Result<Value, OpError>, Value> {
                let in_place: fn(String) -> Result<String, String> = $in_place;
                match input {
                    Value::Text(text) => match in_place(text) {
                        Ok(text) => Ok(Ok(Value::Text(text))),
                        Err(text) => Err(Value::Text(text)),
                    },
                    other => Err(other),
                }
            }
        }
    };
}

text_op!(Reverse, "reverse", "Reverse", "Reverses the order of the characters", |s| s
    .chars()
    .rev()
    .collect());
// Changing the case of ASCII keeps every character the same size, so it can be done
// where the text lies. Other scripts can grow or shrink ("ß" becomes "SS").
text_op!(
    Uppercase,
    "to-uppercase",
    "To Uppercase",
    "Converts the text to uppercase",
    |s| s.to_uppercase(),
    |mut text| {
        if !text.is_ascii() {
            return Err(text);
        }
        text.make_ascii_uppercase();
        Ok(text)
    }
);
text_op!(
    Lowercase,
    "to-lowercase",
    "To Lowercase",
    "Converts the text to lowercase",
    |s| s.to_lowercase(),
    |mut text| {
        if !text.is_ascii() {
            return Err(text);
        }
        text.make_ascii_lowercase();
        Ok(text)
    }
);
text_op!(
    RemoveSpaces,
    "remove-spaces",
    "Remove Spaces",
    "Removes spaces and tabs, keeping line breaks",
    |s| s.chars().filter(|c| !c.is_whitespace() || *c == '\n').collect()
);

/// The words of an identifier or phrase: split on anything that is not a letter or
/// digit, and where a lowercase letter is followed by an uppercase one.
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for c in text.chars() {
        if !c.is_alphanumeric() || (c.is_uppercase() && previous_lower) {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        }
        if c.is_alphanumeric() {
            current.push(c);
        }
        previous_lower = c.is_lowercase() || c.is_numeric();
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
        None => String::new(),
    }
}

/// Applies a per-line conversion, so a list of names converts name by name.
fn each_line(text: &str, convert: impl Fn(&str) -> String) -> String {
    text.split('\n').map(convert).collect::<Vec<_>>().join("\n")
}

text_op!(
    TitleCase,
    "to-title-case",
    "To Title Case",
    "Capitalizes the first letter of every word",
    |s| {
        let mut output = String::with_capacity(s.len());
        let mut at_start = true;
        for c in s.chars() {
            if at_start {
                output.extend(c.to_uppercase());
            } else {
                output.extend(c.to_lowercase());
            }
            at_start = !c.is_alphanumeric() && c != '\'';
        }
        output
    }
);
text_op!(
    CamelCase,
    "to-camel-case",
    "To camelCase",
    "Joins the words of each line as camelCase",
    |s| each_line(s, |line| {
        let words = words(line);
        words
            .iter()
            .enumerate()
            .map(|(index, word)| if index == 0 { word.to_lowercase() } else { capitalize(word) })
            .collect()
    })
);
text_op!(
    SnakeCase,
    "to-snake-case",
    "To snake_case",
    "Joins the words of each line as snake_case",
    |s| each_line(s, |line| words(line).join("_").to_lowercase())
);
text_op!(
    KebabCase,
    "to-kebab-case",
    "To kebab-case",
    "Joins the words of each line as kebab-case",
    |s| each_line(s, |line| words(line).join("-").to_lowercase())
);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct SliceConfig {
    /// First character to keep. Negative counts from the end.
    pub start: i32,
    /// One past the last character to keep. Negative counts from the end, 0 means the end.
    pub end: i32,
}

pub struct SliceText;

impl Op for SliceText {
    const ID: &'static str = "slice-text";
    const NAME: &'static str = "Slice Text";
    const DESCRIPTION: &'static str = "Extracts the characters between a start and an end index";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = SliceConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("start", "Start", ParamKind::Int { min: None, max: None }),
            ParamSpec::new("end", "End (0 = to the end)", ParamKind::Int { min: None, max: None }),
        ]
    }

    fn run(config: &SliceConfig, input: &str) -> Result<String, OpError> {
        let len = input.chars().count() as i64;
        let resolve = |index: i32| -> usize {
            let index = index as i64;
            (if index < 0 { len + index } else { index }).clamp(0, len) as usize
        };
        let start = resolve(config.start);
        let end = if config.end == 0 { len as usize } else { resolve(config.end) };
        Ok(input.chars().skip(start).take(end.saturating_sub(start)).collect())
    }
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

pub struct CountWords;

impl Op for CountWords {
    const ID: &'static str = "count-words";
    const NAME: &'static str = "Count Words";
    const DESCRIPTION: &'static str = "Counts the whitespace-separated words in the text";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = f64;

    fn run(_: &NoConfig, input: &str) -> Result<f64, OpError> {
        Ok(word_count(input) as f64)
    }
}

pub struct TextStats;

impl Op for TextStats {
    const ID: &'static str = "text-stats";
    const NAME: &'static str = "Text Statistics";
    const DESCRIPTION: &'static str = "Counts characters, words, lines and non-whitespace characters";
    const TAGS: &'static [Tag] = &[Tag::Text];
    const OUTPUTS: &'static [&'static str] =
        &["charCount", "wordCount", "lineCount", "nonWhitespaceCount"];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = (f64, f64, f64, f64);

    fn run(_: &NoConfig, input: &str) -> Result<Self::Out, OpError> {
        Ok((
            input.chars().count() as f64,
            word_count(input) as f64,
            input.split('\n').count() as f64,
            input.chars().filter(|c| !c.is_whitespace()).count() as f64,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct DelimiterConfig {
    pub delimiter: String,
}

impl Default for DelimiterConfig {
    fn default() -> Self {
        DelimiterConfig { delimiter: ",".into() }
    }
}

fn delimiter_param() -> Vec<ParamSpec> {
    vec![ParamSpec::new("delimiter", "Delimiter", ParamKind::Text)]
}

pub struct SplitText;

impl Op for SplitText {
    const ID: &'static str = "split-text";
    const NAME: &'static str = "Split Text";
    const DESCRIPTION: &'static str = "Splits the text at a delimiter into a list";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = DelimiterConfig;
    type In<'a> = &'a str;
    type Out = Vec<String>;

    fn params() -> Vec<ParamSpec> {
        delimiter_param()
    }

    fn run(config: &DelimiterConfig, input: &str) -> Result<Vec<String>, OpError> {
        if config.delimiter.is_empty() {
            return Err(OpError::invalid_config("The delimiter cannot be empty"));
        }
        Ok(input.split(config.delimiter.as_str()).map(str::to_owned).collect())
    }
}

pub struct JoinText;

impl Op for JoinText {
    const ID: &'static str = "join-text";
    const NAME: &'static str = "Join Text";
    const DESCRIPTION: &'static str = "Joins a list of texts with a delimiter";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = DelimiterConfig;
    type In<'a> = &'a [String];
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        delimiter_param()
    }

    fn run(config: &DelimiterConfig, input: &[String]) -> Result<String, OpError> {
        Ok(input.join(&config.delimiter))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct ConcatenateConfig {
    pub separator: String,
}

impl Default for ConcatenateConfig {
    fn default() -> Self {
        ConcatenateConfig { separator: " ".into() }
    }
}

pub struct ConcatenateText;

impl Op for ConcatenateText {
    const ID: &'static str = "concatenate-text";
    const NAME: &'static str = "Concatenate Text";
    const DESCRIPTION: &'static str = "Joins two texts with a separator";
    const TAGS: &'static [Tag] = &[Tag::Text];
    const INPUTS: &'static [&'static str] = &["input1", "input2"];

    type Config = ConcatenateConfig;
    type In<'a> = (&'a str, &'a str);
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new("separator", "Separator", ParamKind::Text)]
    }

    fn run(config: &ConcatenateConfig, (first, second): Self::In<'_>) -> Result<String, OpError> {
        Ok(format!("{first}{}{second}", config.separator))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    #[test]
    fn valid_utf8_decodes() {
        let config = Utf8DecodeConfig::default();
        assert_eq!(Utf8Decode::run(&config, "nohut 🌰".as_bytes()).unwrap(), "nohut 🌰");
    }

    #[test]
    fn invalid_utf8_fails_unless_lossy() {
        let bytes = [b'a', 0xff, b'b'];
        let strict = Utf8Decode::run(&Utf8DecodeConfig::default(), &bytes).unwrap_err();
        assert_eq!(strict.code, ErrorCode::InvalidInput);
        let lossy = Utf8Decode::run(&Utf8DecodeConfig { lossy: true }, &bytes).unwrap();
        assert_eq!(lossy, "a\u{fffd}b");
    }

    #[test]
    fn case_conversion_is_unicode_aware() {
        assert_eq!(Uppercase::run(&NoConfig {}, "çiğ nohut").unwrap(), "ÇIĞ NOHUT");
        assert_eq!(Lowercase::run(&NoConfig {}, "ÇOK").unwrap(), "çok");
    }

    #[test]
    fn reverse_and_remove_spaces() {
        assert_eq!(Reverse::run(&NoConfig {}, "abc").unwrap(), "cba");
        assert_eq!(RemoveSpaces::run(&NoConfig {}, "a b\tc\nd e").unwrap(), "abc\nde");
    }

    #[test]
    fn slice_handles_negative_and_open_ends() {
        let slice = |start, end| SliceText::run(&SliceConfig { start, end }, "nohut").unwrap();
        assert_eq!(slice(0, 0), "nohut");
        assert_eq!(slice(1, 3), "oh");
        assert_eq!(slice(-3, 0), "hut");
        assert_eq!(slice(0, -1), "nohu");
        assert_eq!(slice(4, 2), "");
        assert_eq!(slice(0, 99), "nohut");
    }

    #[test]
    fn stats_and_word_count() {
        assert_eq!(CountWords::run(&NoConfig {}, "  one two\nthree ").unwrap(), 3.0);
        assert_eq!(TextStats::run(&NoConfig {}, "ab c\nd").unwrap(), (6.0, 3.0, 2.0, 4.0));
    }

    #[test]
    fn split_then_join_round_trips() {
        let config = DelimiterConfig { delimiter: ";".into() };
        let parts = SplitText::run(&config, "a;b;;c").unwrap();
        assert_eq!(parts, ["a", "b", "", "c"]);
        assert_eq!(JoinText::run(&config, &parts).unwrap(), "a;b;;c");
    }

    #[test]
    fn empty_delimiter_is_rejected() {
        let config = DelimiterConfig { delimiter: String::new() };
        assert_eq!(SplitText::run(&config, "abc").unwrap_err().code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn case_styles_split_words_the_same_way() {
        let run = |op: fn(&NoConfig, &str) -> Result<String, OpError>, text: &str| op(&NoConfig {}, text).unwrap();
        for input in ["parse HTTP response", "parseHttpResponse", "parse_http_response", "parse-http-response"] {
            assert_eq!(run(CamelCase::run, input), "parseHttpResponse", "{input}");
            assert_eq!(run(SnakeCase::run, input), "parse_http_response", "{input}");
            assert_eq!(run(KebabCase::run, input), "parse-http-response", "{input}");
        }
        assert_eq!(run(TitleCase::run, "the quick BROWN fox's den"), "The Quick Brown Fox's Den");
        assert_eq!(run(SnakeCase::run, "first name\nlastName"), "first_name\nlast_name");
        assert_eq!(run(CamelCase::run, "ığdır şehri"), "ığdırŞehri");
    }

    #[test]
    fn case_conversion_in_place_matches_the_ordinary_result() {
        for text in ["Nohut 123", "straße İstanbul"] {
            let ordinary = Uppercase::run(&NoConfig {}, text).unwrap();
            let in_place = match Uppercase::take_over(&NoConfig {}, Value::Text(text.into())) {
                Ok(result) => result.unwrap(),
                // Handed back: the engine then runs it the ordinary way.
                Err(value) => Value::Text(Uppercase::run(&NoConfig {}, match &value {
                    Value::Text(text) => text,
                    _ => unreachable!(),
                }).unwrap()),
            };
            assert_eq!(in_place, Value::Text(ordinary));
        }
        assert!(Uppercase::take_over(&NoConfig {}, Value::Text("straße".into())).is_err());
        assert_eq!(
            Lowercase::take_over(&NoConfig {}, Value::Text("ABC".into())).unwrap().unwrap(),
            Value::Text("abc".into())
        );
    }

    #[test]
    fn concatenate_uses_the_separator() {
        let config = ConcatenateConfig { separator: "-".into() };
        assert_eq!(ConcatenateText::run(&config, ("a", "b")).unwrap(), "a-b");
    }
}
