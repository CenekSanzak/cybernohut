//! Operations that describe their input instead of transforming it.

use crate::core::{NoConfig, Op, OpError, Tag};

/// File signatures: byte offset, the bytes found there, what they identify, and the
/// media type.
/// More specific signatures come before the general ones they overlap with.
const SIGNATURES: &[(usize, &[u8], &str, &str)] = &[
    (0, b"\x89PNG\r\n\x1a\n", "PNG image", "image/png"),
    (0, b"\xff\xd8\xff", "JPEG image", "image/jpeg"),
    (0, b"GIF87a", "GIF image", "image/gif"),
    (0, b"GIF89a", "GIF image", "image/gif"),
    (8, b"WEBP", "WebP image", "image/webp"),
    (8, b"WAVE", "WAV audio", "audio/wav"),
    (0, b"BM", "BMP image", "image/bmp"),
    (0, b"%PDF-", "PDF document", "application/pdf"),
    (0, b"PK\x03\x04", "ZIP archive (also DOCX, XLSX, JAR, APK)", "application/zip"),
    (0, b"\x1f\x8b", "gzip compressed data", "application/gzip"),
    (0, b"BZh", "bzip2 compressed data", "application/x-bzip2"),
    (0, b"\xfd7zXZ\x00", "XZ compressed data", "application/x-xz"),
    (0, b"7z\xbc\xaf\x27\x1c", "7-Zip archive", "application/x-7z-compressed"),
    (0, b"Rar!\x1a\x07", "RAR archive", "application/vnd.rar"),
    (0, b"\x28\xb5\x2f\xfd", "Zstandard compressed data", "application/zstd"),
    (257, b"ustar", "tar archive", "application/x-tar"),
    (0, b"\x7fELF", "ELF executable", "application/x-elf"),
    (0, b"MZ", "Windows executable (PE)", "application/vnd.microsoft.portable-executable"),
    (0, b"\xcf\xfa\xed\xfe", "Mach-O executable (64-bit)", "application/x-mach-binary"),
    (0, b"\xce\xfa\xed\xfe", "Mach-O executable (32-bit)", "application/x-mach-binary"),
    (0, b"\xca\xfe\xba\xbe", "Java class file or Mach-O universal binary", "application/java-vm"),
    (0, b"\x00asm", "WebAssembly module", "application/wasm"),
    (0, b"SQLite format 3\x00", "SQLite database", "application/vnd.sqlite3"),
    (4, b"ftyp", "MP4 or QuickTime media", "video/mp4"),
    (0, b"ID3", "MP3 audio", "audio/mpeg"),
    (0, b"OggS", "Ogg media", "application/ogg"),
    (0, b"fLaC", "FLAC audio", "audio/flac"),
    (0, b"-----BEGIN ", "PEM-encoded key or certificate", "application/x-pem-file"),
    (0, b"\xef\xbb\xbf", "Text (UTF-8 with byte order mark)", "text/plain"),
    (0, b"\xff\xfe", "Text (UTF-16, little endian)", "text/plain"),
    (0, b"\xfe\xff", "Text (UTF-16, big endian)", "text/plain"),
];

/// What the signature bytes of `input` identify, if anything, with its media type.
pub fn identify(input: &[u8]) -> Option<(&'static str, &'static str)> {
    SIGNATURES
        .iter()
        .find(|(offset, magic, ..)| input.get(*offset..).is_some_and(|rest| rest.starts_with(magic)))
        .map(|(_, _, description, mime)| (*description, *mime))
}

/// What the signature bytes of `input` identify, if anything.
pub fn signature(input: &[u8]) -> Option<&'static str> {
    identify(input).map(|(description, _)| description)
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Width and height in pixels, for the image formats that state them near the start.
pub fn dimensions(input: &[u8]) -> Option<(u32, u32)> {
    let le16 = |at: usize| input.get(at..at + 2).map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])));
    let be16 = |at: usize| input.get(at..at + 2).map(|b| u32::from(u16::from_be_bytes([b[0], b[1]])));
    match signature(input)? {
        "PNG image" => Some((be32(input.get(16..20)?), be32(input.get(20..24)?))),
        "GIF image" => Some((le16(6)?, le16(8)?)),
        "BMP image" => {
            let field = |at: usize| input.get(at..at + 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            // A negative height means the rows are stored top to bottom.
            Some((field(18)?.unsigned_abs(), field(22)?.unsigned_abs()))
        }
        "JPEG image" => {
            // Walk the segments to the start-of-frame, which holds the size.
            let mut at = 2;
            loop {
                let marker = *input.get(at + 1)?;
                if *input.get(at)? != 0xff {
                    return None;
                }
                let is_frame = (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc);
                if is_frame {
                    return Some((be16(at + 7)?, be16(at + 5)?));
                }
                at += 2 + be16(at + 2)? as usize;
            }
        }
        _ => None,
    }
}

pub struct DetectFileType;

impl Op for DetectFileType {
    const ID: &'static str = "detect-file-type";
    const NAME: &'static str = "Detect File Type";
    const DESCRIPTION: &'static str =
        "Identifies data by the signature bytes at its start, with its media type and image size";
    const LINK: Option<&'static str> =
        Some("https://en.wikipedia.org/wiki/List_of_file_signatures");
    const TAGS: &'static [Tag] = &[Tag::Analysis];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        Ok(match identify(input) {
            Some((description, mime)) => match dimensions(input) {
                Some((width, height)) => format!("{description} ({mime}), {width} × {height} pixels"),
                None => format!("{description} ({mime})"),
            },
            None if input.is_empty() => "Empty".into(),
            None if std::str::from_utf8(input).is_ok() => "Text (UTF-8) (text/plain)".into(),
            None => "Unknown binary data (application/octet-stream)".into(),
        })
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
        assert_eq!(detect(b"%PDF-1.7"), "PDF document (application/pdf)");
        assert_eq!(detect(b"\x7fELF\x02\x01"), "ELF executable (application/x-elf)");
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00WEBPVP8 "), "WebP image (image/webp)");
        assert_eq!(detect(b"RIFF\x24\x00\x00\x00WAVEfmt "), "WAV audio (audio/wav)");
        assert_eq!(detect(b"\x00\x00\x00\x18ftypmp42"), "MP4 or QuickTime media (video/mp4)");
        // Too short to hold a size: the type is still named.
        assert_eq!(detect(b"\x89PNG\r\n\x1a\n\x00\x00"), "PNG image (image/png)");
    }

    #[test]
    fn image_sizes_are_read() {
        let mut png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR".to_vec();
        png.extend([0, 0, 2, 128, 0, 0, 1, 224]);
        assert_eq!(detect(&png), "PNG image (image/png), 640 × 480 pixels");
        assert_eq!(detect(b"GIF89a\x20\x03\x58\x02"), "GIF image (image/gif), 800 × 600 pixels");
        // SOI, an APP0 segment of 4 bytes, then SOF0 with height 2 and width 3.
        let jpeg = b"\xff\xd8\xff\xe0\x00\x04JF\xff\xc0\x00\x11\x08\x00\x02\x00\x03";
        assert_eq!(detect(jpeg), "JPEG image (image/jpeg), 3 × 2 pixels");
        assert_eq!(detect(b"\xff\xd8\xff\xe0\x00"), "JPEG image (image/jpeg)");
    }

    #[test]
    fn a_signature_at_an_offset_past_the_end_does_not_match_or_panic() {
        assert_eq!(detect(b"RIFF"), "Text (UTF-8) (text/plain)");
    }

    #[test]
    fn unsigned_data_falls_back_to_text_or_unknown() {
        assert_eq!(detect("merhaba dünya".as_bytes()), "Text (UTF-8) (text/plain)");
        assert_eq!(detect(&[0x00, 0xff, 0x80, 0x01]), "Unknown binary data (application/octet-stream)");
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
