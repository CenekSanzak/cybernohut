use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

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
}
