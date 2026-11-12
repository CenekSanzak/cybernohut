//! Small tools that turn up in every investigation: finding the text in a binary,
//! reading a hex dump, and a handful of everyday encodings.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Suggestion, Tag};
use crate::score::text_score;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct StringsConfig {
    /// Shorter runs are ignored.
    pub min_length: i32,
}

impl Default for StringsConfig {
    fn default() -> Self {
        StringsConfig { min_length: 4 }
    }
}

pub struct Strings;

impl Op for Strings {
    const ID: &'static str = "strings";
    const NAME: &'static str = "Strings";
    const DESCRIPTION: &'static str =
        "Lists the runs of printable characters in binary data, like the Unix strings tool";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Strings_(Unix)");
    const TAGS: &'static [Tag] = &[Tag::Analysis];

    type Config = StringsConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<String>;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new(
            "minLength",
            "Minimum length",
            ParamKind::Int { min: Some(1), max: None },
        )]
    }

    fn run(config: &StringsConfig, input: &[u8]) -> Result<Vec<String>, OpError> {
        let min = usize::try_from(config.min_length).unwrap_or(1).max(1);
        Ok(input
            .split(|byte| !(byte.is_ascii_graphic() || *byte == b' ' || *byte == b'\t'))
            .filter(|run| run.len() >= min)
            .map(|run| String::from_utf8(run.to_vec()).expect("printable ASCII is UTF-8"))
            .collect())
    }
}

const DUMP_WIDTH: usize = 16;

pub struct ToHexdump;

impl Op for ToHexdump {
    const ID: &'static str = "to-hexdump";
    const NAME: &'static str = "To Hexdump";
    const DESCRIPTION: &'static str =
        "Shows bytes sixteen to a line with their offset and their text, like xxd";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Hex_dump");
    const TAGS: &'static [Tag] = &[Tag::NumberBase, Tag::Analysis];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        let mut output = String::with_capacity(input.len().div_ceil(DUMP_WIDTH) * 68);
        for (index, row) in input.chunks(DUMP_WIDTH).enumerate() {
            if index > 0 {
                output.push('\n');
            }
            output.push_str(&format!("{:08x}: ", index * DUMP_WIDTH));
            for position in 0..DUMP_WIDTH {
                match row.get(position) {
                    Some(byte) => output.push_str(&format!("{byte:02x}")),
                    None => output.push_str("  "),
                }
                if position % 2 == 1 {
                    output.push(' ');
                }
            }
            output.push(' ');
            output.extend(row.iter().map(|byte| {
                if byte.is_ascii_graphic() || *byte == b' ' { char::from(*byte) } else { '.' }
            }));
        }
        Ok(output)
    }
}

/// The hex digits of one dump line, without its offset and its text column.
fn dump_line_digits(line: &str) -> Option<String> {
    let (offset, rest) = line.split_once(':')?;
    if offset.is_empty() || !offset.trim().bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // The text column starts after two spaces; hex groups are separated by one.
    let rest = rest.trim_start();
    let hex = rest.split("  ").next().unwrap_or(rest);
    let digits: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    (digits.len() % 2 == 0 && digits.bytes().all(|b| b.is_ascii_hexdigit())).then_some(digits)
}

pub struct FromHexdump;

impl Op for FromHexdump {
    const ID: &'static str = "from-hexdump";
    const NAME: &'static str = "From Hexdump";
    const DESCRIPTION: &'static str = "Turns the output of xxd or hexdump -C back into bytes";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Hex_dump");
    const TAGS: &'static [Tag] = &[Tag::NumberBase];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let mut lines = input.lines().filter(|line| !line.trim().is_empty()).peekable();
        lines.peek()?;
        lines
            .all(|line| dump_line_digits(&line.replace('|', "  ")).is_some_and(|d| !d.is_empty()))
            .then(|| Suggestion::new("Every line is an offset, a colon and hex bytes: a hex dump"))
    }

    fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
        let mut digits = String::new();
        for (number, line) in input.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            // `hexdump -C` puts the text column between bars, and has no colon.
            let line = match line.split_once('|') {
                Some((before, _)) if !before.contains(':') => {
                    let mut parts = before.trim().splitn(2, ' ');
                    format!("{}: {}", parts.next().unwrap_or(""), parts.next().unwrap_or("").replace("  ", " "))
                }
                _ => line.to_owned(),
            };
            digits.push_str(&dump_line_digits(&line).ok_or_else(|| {
                OpError::invalid_input(format!("Line {} is not a hex dump line", number + 1))
            })?);
        }
        hex_simd::decode_to_vec(digits).map_err(|_| OpError::invalid_input("Not a hex dump"))
    }
}

const MORSE: &[(char, &str)] = &[
    ('a', ".-"), ('b', "-..."), ('c', "-.-."), ('d', "-.."), ('e', "."), ('f', "..-."),
    ('g', "--."), ('h', "...."), ('i', ".."), ('j', ".---"), ('k', "-.-"), ('l', ".-.."),
    ('m', "--"), ('n', "-."), ('o', "---"), ('p', ".--."), ('q', "--.-"), ('r', ".-."),
    ('s', "..."), ('t', "-"), ('u', "..-"), ('v', "...-"), ('w', ".--"), ('x', "-..-"),
    ('y', "-.--"), ('z', "--.."), ('0', "-----"), ('1', ".----"), ('2', "..---"),
    ('3', "...--"), ('4', "....-"), ('5', "....."), ('6', "-...."), ('7', "--..."),
    ('8', "---.."), ('9', "----."), ('.', ".-.-.-"), (',', "--..--"), ('?', "..--.."),
    ('\'', ".----."), ('!', "-.-.--"), ('/', "-..-."), ('(', "-.--."), (')', "-.--.-"),
    ('&', ".-..."), (':', "---..."), (';', "-.-.-."), ('=', "-...-"), ('+', ".-.-."),
    ('-', "-....-"), ('_', "..--.-"), ('"', ".-..-."), ('$', "...-..-"), ('@', ".--.-."),
];

pub struct ToMorse;

impl Op for ToMorse {
    const ID: &'static str = "to-morse";
    const NAME: &'static str = "To Morse Code";
    const DESCRIPTION: &'static str =
        "Writes text as dots and dashes, with a space between letters and a slash between words";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Morse_code");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let mut words = Vec::new();
        for word in input.split_whitespace() {
            let mut letters = Vec::new();
            for c in word.chars().flat_map(char::to_lowercase) {
                let (_, code) = MORSE.iter().find(|(letter, _)| *letter == c).ok_or_else(|| {
                    OpError::invalid_input(format!("\"{c}\" has no Morse code"))
                })?;
                letters.push(*code);
            }
            words.push(letters.join(" "));
        }
        Ok(words.join(" / "))
    }
}

pub struct FromMorse;

impl Op for FromMorse {
    const ID: &'static str = "from-morse";
    const NAME: &'static str = "From Morse Code";
    const DESCRIPTION: &'static str =
        "Reads dots and dashes; letters are separated by spaces, words by a slash or a line break";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Morse_code");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let marks = input.chars().filter(|c| matches!(c, '.' | '-' | '_' | '·' | '–' | '—')).count();
        (marks >= 6
            && input.chars().all(|c| matches!(c, '.' | '-' | '_' | '·' | '–' | '—' | '/' | '|') || c.is_whitespace())
            && Self::run(&NoConfig {}, input).is_ok())
        .then(|| Suggestion::new("Only dots, dashes and separators: Morse code"))
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let normalized: String = input
            .chars()
            .map(|c| match c {
                '·' | '•' => '.',
                '_' | '–' | '—' => '-',
                '|' | '\n' => '/',
                other => other,
            })
            .collect();
        let mut words = Vec::new();
        for word in normalized.split('/').map(str::trim).filter(|word| !word.is_empty()) {
            let mut text = String::new();
            for code in word.split_whitespace() {
                let (letter, _) = MORSE.iter().find(|(_, known)| *known == code).ok_or_else(|| {
                    OpError::invalid_input(format!("\"{code}\" is not a Morse code letter"))
                })?;
                text.push(*letter);
            }
            words.push(text.to_uppercase());
        }
        Ok(words.join(" "))
    }
}

const ENTITIES: &[(&str, char)] = &[
    ("amp", '&'), ("lt", '<'), ("gt", '>'), ("quot", '"'), ("apos", '\''), ("nbsp", '\u{a0}'),
    ("copy", '©'), ("reg", '®'), ("trade", '™'), ("euro", '€'), ("pound", '£'), ("yen", '¥'),
    ("cent", '¢'), ("sect", '§'), ("deg", '°'), ("plusmn", '±'), ("times", '×'), ("divide", '÷'),
    ("laquo", '«'), ("raquo", '»'), ("ldquo", '“'), ("rdquo", '”'), ("lsquo", '‘'), ("rsquo", '’'),
    ("ndash", '–'), ("mdash", '—'), ("hellip", '…'), ("bull", '•'), ("middot", '·'),
];

/// Decodes the entities in `input` and says how many there were.
fn decode_entities(input: &str) -> (String, usize) {
    let mut output = String::with_capacity(input.len());
    let mut decoded = 0;
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        let end = rest[1..].find(|c: char| c == ';' || c == '&' || c.is_whitespace()).map(|i| i + 1);
        let character = match end {
            Some(end) if rest.as_bytes()[end] == b';' => {
                let name = &rest[1..end];
                let numeric = name.strip_prefix('#').and_then(|number| {
                    match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => number.parse().ok(),
                    }
                });
                numeric
                    .and_then(char::from_u32)
                    .or_else(|| ENTITIES.iter().find(|(known, _)| *known == name).map(|(_, c)| *c))
                    .map(|c| (c, end + 1))
            }
            _ => None,
        };
        match character {
            Some((c, consumed)) => {
                output.push(c);
                decoded += 1;
                rest = &rest[consumed..];
            }
            // Not an entity: the ampersand stands for itself.
            None => {
                output.push('&');
                rest = &rest[1..];
            }
        }
    }
    output.push_str(rest);
    (output, decoded)
}

pub struct HtmlEntityDecode;

impl Op for HtmlEntityDecode {
    const ID: &'static str = "html-entity-decode";
    const NAME: &'static str = "HTML Entity Decode";
    const DESCRIPTION: &'static str =
        "Turns entities such as &amp;lt;, &amp;#65; and &amp;#x41; back into their characters";
    const LINK: Option<&'static str> =
        Some("https://en.wikipedia.org/wiki/List_of_XML_and_HTML_character_entity_references");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let (_, decoded) = decode_entities(input);
        // One `&amp;` in a sentence is ordinary; text made of entities is encoded.
        (decoded >= 3 && decoded * 12 >= input.chars().count())
            .then(|| Suggestion::new(format!("Contains {decoded} HTML entities")))
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(decode_entities(input).0)
    }
}

pub struct HtmlEntityEncode;

impl Op for HtmlEntityEncode {
    const ID: &'static str = "html-entity-encode";
    const NAME: &'static str = "HTML Entity Encode";
    const DESCRIPTION: &'static str =
        "Escapes the characters that have a meaning in HTML: & < > \" and '";
    const LINK: Option<&'static str> =
        Some("https://en.wikipedia.org/wiki/List_of_XML_and_HTML_character_entity_references");
    const TAGS: &'static [Tag] = &[Tag::Encoding];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let mut output = String::with_capacity(input.len());
        for c in input.chars() {
            match c {
                '&' => output.push_str("&amp;"),
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '"' => output.push_str("&quot;"),
                '\'' => output.push_str("&#39;"),
                other => output.push(other),
            }
        }
        Ok(output)
    }
}

fn rot47(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '!'..='~' => char::from((c as u8 - b'!' + 47) % 94 + b'!'),
            other => other,
        })
        .collect()
}

/// ROT13's cousin over all 94 printable ASCII characters, so digits and punctuation
/// are scrambled too. Applying it twice undoes it.
pub struct Rot47;

impl Op for Rot47 {
    const ID: &'static str = "rot47";
    const NAME: &'static str = "ROT47";
    const DESCRIPTION: &'static str =
        "Rotates letters, digits and punctuation by 47; applying it twice undoes it";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/ROT13#Variants");
    const TAGS: &'static [Tag] = &[Tag::Text, Tag::SubstitutionCipher];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let sample: String = input.chars().take(2000).collect();
        if sample.chars().filter(|c| c.is_ascii_graphic()).count() < 8 {
            return None;
        }
        let (before, after) = (text_score(&sample), text_score(&rot47(&sample)));
        (after >= 0.6 && after >= before + 0.15).then(|| {
            Suggestion::new(format!(
                "Rotating printable characters by 47 gives readable text \
                 (readability goes from {before:.2} to {after:.2})"
            ))
        })
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(rot47(input))
    }
}

/// Exactly `count` hex digits from the front of `chars`, as a number.
fn take_hex(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, count: usize) -> Option<u32> {
    let digits: String = (0..count).map_while(|_| chars.next_if(char::is_ascii_hexdigit)).collect();
    (digits.len() == count).then(|| u32::from_str_radix(&digits, 16).ok()).flatten()
}

/// Decodes `\uXXXX`, `\u{X}`, `\xXX`, `\n` and friends, and says how many there were.
fn decode_escapes(input: &str) -> Result<(String, usize), OpError> {
    let mut output = String::with_capacity(input.len());
    let mut decoded = 0;
    let mut pending_high: Option<u32> = None;
    let mut chars = input.chars().peekable();
    let bad = |what: &str| OpError::invalid_input(format!("\"{what}\" is not a complete escape"));
    while let Some(c) = chars.next() {
        if c != '\\' {
            output.push(c);
            continue;
        }
        let code = match chars.next() {
            Some('n') => 0x0a,
            Some('r') => 0x0d,
            Some('t') => 0x09,
            Some('0') => 0x00,
            Some('\\') => 0x5c,
            Some('"') => 0x22,
            Some('\'') => 0x27,
            Some('/') => 0x2f,
            Some('x') => take_hex(&mut chars, 2).ok_or_else(|| bad("\\x"))?,
            Some('U') => take_hex(&mut chars, 8).ok_or_else(|| bad("\\U"))?,
            Some('u') => {
                if chars.next_if_eq(&'{').is_some() {
                    let digits: String = std::iter::from_fn(|| chars.next_if(char::is_ascii_hexdigit)).collect();
                    chars.next_if_eq(&'}').ok_or_else(|| bad("\\u{"))?;
                    u32::from_str_radix(&digits, 16).map_err(|_| bad("\\u{"))?
                } else {
                    take_hex(&mut chars, 4).ok_or_else(|| bad("\\u"))?
                }
            }
            Some(other) => {
                // An escape this does not know is left as written.
                output.push('\\');
                output.push(other);
                continue;
            }
            None => {
                output.push('\\');
                continue;
            }
        };
        decoded += 1;
        // JSON and JavaScript write characters beyond U+FFFF as two halves.
        let code = match (pending_high.take(), code) {
            (Some(high), 0xdc00..=0xdfff) => 0x10000 + ((high - 0xd800) << 10) + (code - 0xdc00),
            (_, 0xd800..=0xdbff) => {
                pending_high = Some(code);
                continue;
            }
            (_, code) => code,
        };
        output.push(char::from_u32(code).ok_or_else(|| {
            OpError::invalid_input(format!("U+{code:X} is not a character"))
        })?);
    }
    Ok((output, decoded))
}

pub struct UnescapeText;

impl Op for UnescapeText {
    const ID: &'static str = "unescape-text";
    const NAME: &'static str = "Unescape Text";
    const DESCRIPTION: &'static str =
        "Decodes escapes such as \\u0041, \\x41 and \\n, as written in JSON and source code";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Escape_sequence");
    const TAGS: &'static [Tag] = &[Tag::Encoding, Tag::Text];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn detect(input: &str) -> Option<Suggestion<NoConfig>> {
        let (_, decoded) = decode_escapes(input).ok()?;
        let numeric = input.matches("\\u").count() + input.matches("\\x").count() + input.matches("\\U").count();
        (numeric >= 3 && decoded * 12 >= input.chars().count())
            .then(|| Suggestion::new(format!("Contains {decoded} escape sequences such as \\u0041 or \\x41")))
    }

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(decode_escapes(input)?.0)
    }
}

pub struct EscapeText;

impl Op for EscapeText {
    const ID: &'static str = "escape-text";
    const NAME: &'static str = "Escape Text";
    const DESCRIPTION: &'static str =
        "Writes every character outside printable ASCII as a \\u escape, as JSON does";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Escape_sequence");
    const TAGS: &'static [Tag] = &[Tag::Encoding, Tag::Text];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let mut output = String::with_capacity(input.len());
        for c in input.chars() {
            match c {
                '\\' => output.push_str("\\\\"),
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                ' '..='~' => output.push(c),
                other => {
                    let mut units = [0u16; 2];
                    for unit in other.encode_utf16(&mut units) {
                        output.push_str(&format!("\\u{unit:04x}"));
                    }
                }
            }
        }
        Ok(output)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct SortLinesConfig {
    pub reverse: bool,
    /// Compare as numbers where lines are numbers.
    pub numeric: bool,
}

pub struct SortLines;

impl Op for SortLines {
    const ID: &'static str = "sort-lines";
    const NAME: &'static str = "Sort Lines";
    const DESCRIPTION: &'static str = "Puts the lines in alphabetical or numeric order";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = SortLinesConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("reverse", "Reverse", ParamKind::Bool),
            ParamSpec::new("numeric", "Numeric", ParamKind::Bool),
        ]
    }

    fn run(config: &SortLinesConfig, input: &str) -> Result<String, OpError> {
        let mut lines: Vec<&str> = input.lines().collect();
        if config.numeric {
            // Lines that are not numbers keep their order after the numbers.
            let key = |line: &str| line.trim().parse::<f64>().unwrap_or(f64::INFINITY);
            lines.sort_by(|a, b| key(a).total_cmp(&key(b)));
        } else {
            lines.sort_unstable();
        }
        if config.reverse {
            lines.reverse();
        }
        Ok(lines.join("\n"))
    }
}

pub struct UniqueLines;

impl Op for UniqueLines {
    const ID: &'static str = "unique-lines";
    const NAME: &'static str = "Unique Lines";
    const DESCRIPTION: &'static str = "Keeps the first of each distinct line, in the original order";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        let mut seen = std::collections::HashSet::new();
        Ok(input.lines().filter(|line| seen.insert(*line)).collect::<Vec<_>>().join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text<T: for<'a> Op<Config = NoConfig, In<'a> = &'a str, Out = String>>(input: &str) -> String {
        T::run(&NoConfig {}, input).unwrap()
    }

    #[test]
    fn strings_lists_printable_runs_of_the_minimum_length() {
        let binary = b"\x00\x01flag{here}\xff\xfeab\x00key = nohut\n\x7f";
        assert_eq!(Strings::run(&StringsConfig::default(), binary).unwrap(), ["flag{here}", "key = nohut"]);
        assert_eq!(Strings::run(&StringsConfig { min_length: 2 }, binary).unwrap().len(), 3);
        assert_eq!(Strings::run(&StringsConfig { min_length: 0 }, b"a\0").unwrap(), ["a"]);
    }

    #[test]
    fn hexdump_matches_xxd_and_reads_back() {
        let data: Vec<u8> = b"The quick brown fox\n\x00\xff".to_vec();
        let dump = ToHexdump::run(&NoConfig {}, &data).unwrap();
        // `printf 'The quick brown fox\n\x00\xff' | xxd`
        assert_eq!(
            dump,
            "00000000: 5468 6520 7175 6963 6b20 6272 6f77 6e20  The quick brown \n\
             00000010: 666f 780a 00ff                           fox..."
        );
        assert_eq!(FromHexdump::run(&NoConfig {}, &dump).unwrap(), data);
        assert!(FromHexdump::detect(&dump).is_some());
        assert!(FromHexdump::detect("just: some text").is_none());
        // `hexdump -C` layout.
        let canonical = "00000000  54 68 65 20 71 75 69 63  6b 20 62 72 6f 77 6e 20  |The quick brown |\n\
                         00000010  66 6f 78 0a 00 ff                                 |fox...|\n00000016";
        assert_eq!(FromHexdump::run(&NoConfig {}, canonical.trim_end_matches("00000016")).unwrap(), data);
    }

    #[test]
    fn morse_round_trips_and_accepts_common_variants() {
        assert_eq!(text::<ToMorse>("SOS now"), "... --- ... / -. --- .--");
        assert_eq!(text::<FromMorse>("... --- ... / -. --- .--"), "SOS NOW");
        assert_eq!(text::<FromMorse>("··· ___ ···\n-. --- .--"), "SOS NOW");
        assert!(FromMorse::detect("... --- ... / -. --- .--").is_some());
        assert!(FromMorse::detect("--- not morse ---").is_none());
        assert!(FromMorse::run(&NoConfig {}, "........").is_err());
        assert!(ToMorse::run(&NoConfig {}, "ş").is_err());
    }

    #[test]
    fn html_entities_decode_by_name_and_number() {
        assert_eq!(text::<HtmlEntityDecode>("&lt;b&gt; &amp; &#65;&#x42;&euro; R&D &bogus; &"), "<b> & AB€ R&D &bogus; &");
        assert_eq!(text::<HtmlEntityEncode>("<a href=\"x\">R&D's</a>"), "&lt;a href=&quot;x&quot;&gt;R&amp;D&#39;s&lt;/a&gt;");
        assert!(HtmlEntityDecode::detect("&#102;&#108;&#97;&#103;").is_some());
        assert!(HtmlEntityDecode::detect("Research &amp; development is a long word for it.").is_none());
    }

    #[test]
    fn rot47_is_its_own_inverse_and_is_detected() {
        let prose = "The secret message is hidden in the last file that you will find here.";
        let rotated = text::<Rot47>(prose);
        assert_eq!(rotated, "%96 D64C6E >6DD286 :D 9:556? :? E96 =2DE 7:=6 E92E J@F H:== 7:?5 96C6]");
        assert_eq!(text::<Rot47>(&rotated), prose);
        assert!(Rot47::detect(&rotated).is_some());
        assert!(Rot47::detect(prose).is_none());
    }

    #[test]
    fn escapes_decode_in_every_common_notation() {
        assert_eq!(text::<UnescapeText>(r"\u0066\x6c\u{61}\U00000067 \n\t\\ \q"), "flag \n\t\\ \\q");
        assert_eq!(text::<UnescapeText>(r"\ud83c\udf30"), "🌰");
        assert_eq!(text::<EscapeText>("nohut 🌰\nş"), r"nohut \ud83c\udf30\n\u015f");
        assert_eq!(text::<UnescapeText>(&text::<EscapeText>("nohut 🌰\nş \\ x")), "nohut 🌰\nş \\ x");
        assert!(UnescapeText::run(&NoConfig {}, r"\u12").is_err());
        assert!(UnescapeText::detect(r"\u0066\u006c\u0061\u0067").is_some());
        assert!(UnescapeText::detect(r"C:\new\table and a long sentence after it").is_none());
    }

    #[test]
    fn lines_are_sorted_and_deduplicated() {
        let sort = |reverse, numeric, input: &str| SortLines::run(&SortLinesConfig { reverse, numeric }, input).unwrap();
        assert_eq!(sort(false, false, "pear\napple\nfig"), "apple\nfig\npear");
        assert_eq!(sort(true, false, "pear\napple\nfig"), "pear\nfig\napple");
        assert_eq!(sort(false, false, "10\n9\n100"), "10\n100\n9");
        assert_eq!(sort(false, true, "10\n9\nx\n100"), "9\n10\n100\nx");
        assert_eq!(text::<UniqueLines>("b\na\nb\nc\na"), "b\na\nc");
    }
}
