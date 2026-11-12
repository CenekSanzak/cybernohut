//! Operations that describe their input instead of transforming it.

use crate::core::{NoConfig, Op, OpError, Tag};

/// File signatures: byte offset, the bytes found there, and what they identify.
/// More specific signatures come before the general ones they overlap with.
const SIGNATURES: &[(usize, &[u8], &str)] = &[
    (0, b"\x89PNG\r\n\x1a\n", "PNG image"),
    (0, b"\xff\xd8\xff", "JPEG image"),
    (0, b"GIF87a", "GIF image"),
    (0, b"GIF89a", "GIF image"),
    (8, b"WEBP", "WebP image"),
    (8, b"WAVE", "WAV audio"),
    (0, b"BM", "BMP image"),
    (0, b"%PDF-", "PDF document"),
    (0, b"PK\x03\x04", "ZIP archive (also DOCX, XLSX, JAR, APK)"),
    (0, b"\x1f\x8b", "gzip compressed data"),
    (0, b"BZh", "bzip2 compressed data"),
    (0, b"\xfd7zXZ\x00", "XZ compressed data"),
    (0, b"7z\xbc\xaf\x27\x1c", "7-Zip archive"),
    (0, b"Rar!\x1a\x07", "RAR archive"),
    (0, b"\x28\xb5\x2f\xfd", "Zstandard compressed data"),
    (257, b"ustar", "tar archive"),
    (0, b"\x7fELF", "ELF executable"),
    (0, b"MZ", "Windows executable (PE)"),
    (0, b"\xcf\xfa\xed\xfe", "Mach-O executable (64-bit)"),
    (0, b"\xce\xfa\xed\xfe", "Mach-O executable (32-bit)"),
    (0, b"\xca\xfe\xba\xbe", "Java class file or Mach-O universal binary"),
    (0, b"\x00asm", "WebAssembly module"),
    (0, b"SQLite format 3\x00", "SQLite database"),
    (4, b"ftyp", "MP4 or QuickTime media"),
    (0, b"ID3", "MP3 audio"),
    (0, b"OggS", "Ogg media"),
    (0, b"fLaC", "FLAC audio"),
    (0, b"-----BEGIN ", "PEM-encoded key or certificate"),
    (0, b"\xef\xbb\xbf", "Text (UTF-8 with byte order mark)"),
    (0, b"\xff\xfe", "Text (UTF-16, little endian)"),
    (0, b"\xfe\xff", "Text (UTF-16, big endian)"),
];

pub struct DetectFileType;

impl Op for DetectFileType {
    const ID: &'static str = "detect-file-type";
    const NAME: &'static str = "Detect File Type";
    const DESCRIPTION: &'static str = "Identifies data by the signature bytes at its start";
    const LINK: Option<&'static str> =
        Some("https://en.wikipedia.org/wiki/List_of_file_signatures");
    const TAGS: &'static [Tag] = &[Tag::Analysis];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        let by_signature = SIGNATURES
            .iter()
            .find(|(offset, magic, _)| input.get(*offset..).is_some_and(|rest| rest.starts_with(magic)))
            .map(|(_, _, description)| *description);
        Ok(match by_signature {
            Some(description) => description,
            None if input.is_empty() => "Empty",
            None if std::str::from_utf8(input).is_ok() => "Text (UTF-8)",
            None => "Unknown binary data",
        }
        .to_owned())
    }
}

/// Shannon entropy in bits per byte: near 0 for repetitive data, near 8 for data that
/// is compressed, encrypted or random.
pub struct Entropy;

impl Op for Entropy {
    const ID: &'static str = "entropy";
    const NAME: &'static str = "Entropy";
    const DESCRIPTION: &'static str =
        "Shannon entropy in bits per byte, from 0 (repetitive) to 8 (random-looking)";
    const LINK: Option<&'static str> =
        Some("https://en.wikipedia.org/wiki/Entropy_(information_theory)");
    const TAGS: &'static [Tag] = &[Tag::Analysis];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = f64;

    fn run(_: &NoConfig, input: &[u8]) -> Result<f64, OpError> {
        if input.is_empty() {
            return Ok(0.0);
        }
        let mut counts = [0u64; 256];
        for byte in input {
            counts[*byte as usize] += 1;
        }
        let total = input.len() as f64;
        let entropy: f64 = counts
            .iter()
            .filter(|count| **count > 0)
            .map(|count| {
                let p = *count as f64 / total;
                -p * p.log2()
            })
            .sum();
        // Round away floating-point noise so results read cleanly.
        Ok((entropy * 1e6).round() / 1e6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect(input: &[u8]) -> String {
        DetectFileType::run(&NoConfig {}, input).unwrap()
    }

    #[test]
    fn signatures_are_recognized() {
        assert_eq!(detect(b"\x89PNG\r\n\x1a\n\x00\x00"), "PNG image");
        assert_eq!(detect(b"%PDF-1.7"), "PDF document");
        assert_eq!(detect(b"\x7fELF\x02\x01"), "ELF executable");
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00WEBPVP8 "), "WebP image");
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00WAVEfmt "), "WAV audio");
        assert_eq!(detect(b"\x00\x00\x00\x18ftypmp42"), "MP4 or QuickTime media");
    }

    #[test]
    fn a_signature_at_an_offset_past_the_end_does_not_match_or_panic() {
        assert_eq!(detect(b"RIFF"), "Text (UTF-8)");
    }

    #[test]
    fn unsigned_data_falls_back_to_text_or_unknown() {
        assert_eq!(detect("merhaba dünya".as_bytes()), "Text (UTF-8)");
        assert_eq!(detect(&[0x00, 0xff, 0x80, 0x01]), "Unknown binary data");
        assert_eq!(detect(b""), "Empty");
    }

    #[test]
    fn entropy_spans_zero_to_eight_bits() {
        assert_eq!(Entropy::run(&NoConfig {}, b"").unwrap(), 0.0);
        assert_eq!(Entropy::run(&NoConfig {}, b"aaaaaaaa").unwrap(), 0.0);
        assert_eq!(Entropy::run(&NoConfig {}, b"abababab").unwrap(), 1.0);
        let every_byte: Vec<u8> = (0..=255).collect();
        assert_eq!(Entropy::run(&NoConfig {}, &every_byte).unwrap(), 8.0);
    }
}
