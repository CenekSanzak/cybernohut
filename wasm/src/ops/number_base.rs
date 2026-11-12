//! Bytes written out as numbers in base 2, 8 or 10, and back.

use crate::core::{NoConfig, Op, OpError, Tag};

fn encode(input: &[u8], format: fn(u8) -> String) -> String {
    input.iter().map(|byte| format(*byte)).collect::<Vec<_>>().join(" ")
}

/// Accepts numbers separated by anything that is not a digit of the base: spaces,
/// commas, line breaks. Binary may also be one unbroken run of 8-bit groups.
fn decode(input: &str, radix: u32, name: &str) -> Result<Vec<u8>, OpError> {
    let mut output = Vec::new();
    for token in input.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()) {
        let groups: Vec<&str> = if radix == 2 && token.len() > 8 && token.len() % 8 == 0 {
            (0..token.len()).step_by(8).map(|start| &token[start..start + 8]).collect()
        } else {
            vec![token]
        };
        for group in groups {
            let value = u8::from_str_radix(group, radix).map_err(|_| {
                OpError::invalid_input(format!("\"{group}\" is not a {name} number from 0 to 255"))
            })?;
            output.push(value);
        }
    }
    Ok(output)
}

macro_rules! base_ops {
    ($to:ident, $from:ident, $id:literal, $label:literal, $radix:literal, $format:expr) => {
        pub struct $to;

        impl Op for $to {
            const ID: &'static str = concat!("to-", $id);
            const NAME: &'static str = concat!("To ", $label);
            const DESCRIPTION: &'static str =
                concat!("Writes each byte as a ", $id, " number, separated by spaces");
            const TAGS: &'static [Tag] = &[Tag::NumberBase];

            type Config = NoConfig;
            type In<'a> = &'a [u8];
            type Out = String;

            fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
                Ok(encode(input, $format))
            }
        }

        pub struct $from;

        impl Op for $from {
            const ID: &'static str = concat!("from-", $id);
            const NAME: &'static str = concat!("From ", $label);
            const DESCRIPTION: &'static str =
                concat!("Reads ", $id, " numbers from 0 to 255 back into bytes");
            const TAGS: &'static [Tag] = &[Tag::NumberBase];

            type Config = NoConfig;
            type In<'a> = &'a str;
            type Out = Vec<u8>;

            fn run(_: &NoConfig, input: &str) -> Result<Vec<u8>, OpError> {
                decode(input, $radix, $id)
            }
        }
    };
}

base_ops!(ToBinary, FromBinary, "binary", "Binary", 2, |byte| format!("{byte:08b}"));
base_ops!(ToOctal, FromOctal, "octal", "Octal", 8, |byte| format!("{byte:o}"));
base_ops!(ToDecimal, FromDecimal, "decimal", "Decimal", 10, |byte| byte.to_string());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    #[test]
    fn each_base_has_its_usual_spelling() {
        assert_eq!(ToBinary::run(&NoConfig {}, b"hi").unwrap(), "01101000 01101001");
        assert_eq!(ToOctal::run(&NoConfig {}, b"hi").unwrap(), "150 151");
        assert_eq!(ToDecimal::run(&NoConfig {}, b"hi").unwrap(), "104 105");
    }

    #[test]
    fn every_byte_survives_every_round_trip() {
        let bytes: Vec<u8> = (0..=255).collect();
        let binary = ToBinary::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(FromBinary::run(&NoConfig {}, &binary).unwrap(), bytes);
        let octal = ToOctal::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(FromOctal::run(&NoConfig {}, &octal).unwrap(), bytes);
        let decimal = ToDecimal::run(&NoConfig {}, &bytes).unwrap();
        assert_eq!(FromDecimal::run(&NoConfig {}, &decimal).unwrap(), bytes);
    }

    #[test]
    fn separators_are_flexible_and_binary_may_be_unbroken() {
        assert_eq!(FromDecimal::run(&NoConfig {}, "104,105\n33").unwrap(), b"hi!");
        assert_eq!(FromBinary::run(&NoConfig {}, "0110100001101001").unwrap(), b"hi");
        assert_eq!(FromBinary::run(&NoConfig {}, "1101000 1101001").unwrap(), b"hi");
    }

    #[test]
    fn out_of_range_and_wrong_digits_are_errors() {
        for (run, input) in [
            (FromDecimal::run as fn(&NoConfig, &str) -> _, "256"),
            (FromBinary::run, "102"),
            (FromOctal::run, "8"),
        ] {
            assert_eq!(run(&NoConfig {}, input).unwrap_err().code, ErrorCode::InvalidInput, "{input}");
        }
    }
}
