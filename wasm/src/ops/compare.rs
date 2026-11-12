//! Telling whether two values are the same, as when checking a file against its hash.

use crate::core::{NoConfig, Op, OpError, Tag};

pub struct Compare;

impl Op for Compare {
    const ID: &'static str = "compare";
    const NAME: &'static str = "Compare";
    const DESCRIPTION: &'static str =
        "Says whether two inputs are identical, and where they first differ";
    const TAGS: &'static [Tag] = &[Tag::Analysis, Tag::Hashing];
    const INPUTS: &'static [&'static str] = &["first", "second"];

    type Config = NoConfig;
    type In<'a> = (&'a [u8], &'a [u8]);
    type Out = String;

    fn run(_: &NoConfig, (first, second): Self::In<'_>) -> Result<String, OpError> {
        // A pasted hash often differs from a computed one only in case or a newline.
        let loosely = |bytes: &[u8]| -> Vec<u8> {
            bytes.trim_ascii().iter().map(u8::to_ascii_lowercase).collect()
        };
        let common = first.iter().zip(second).take_while(|(a, b)| a == b).count();
        Ok(if first == second {
            format!("Identical ({} bytes)", first.len())
        } else if loosely(first) == loosely(second) {
            "Same apart from letter case or surrounding whitespace".into()
        } else if common == first.len().min(second.len()) {
            format!(
                "Different: one is the first {common} bytes of the other ({} and {} bytes)",
                first.len(),
                second.len()
            )
        } else {
            format!(
                "Different: first difference at byte {common} (0x{:02x} and 0x{:02x})",
                first[common], second[common]
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(first: &[u8], second: &[u8]) -> String {
        Compare::run(&NoConfig {}, (first, second)).unwrap()
    }

    #[test]
    fn each_kind_of_result() {
        assert_eq!(compare(b"abc", b"abc"), "Identical (3 bytes)");
        assert_eq!(compare(b"ABC\n", b"abc"), "Same apart from letter case or surrounding whitespace");
        assert_eq!(compare(b"abc", b"abd"), "Different: first difference at byte 2 (0x63 and 0x64)");
        assert!(compare(b"abc", b"abcdef").starts_with("Different: one is the first 3 bytes"));
        assert_eq!(compare(b"", b""), "Identical (0 bytes)");
    }
}
