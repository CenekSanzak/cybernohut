//! DEFLATE in its three wrappers: gzip, zlib and none.

use std::io::{Read, Write};

use flate2::read::{DeflateDecoder, MultiGzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use flate2::Compression;

use crate::core::{NoConfig, Op, OpError, Suggestion, Tag};

/// A few kilobytes can inflate to gigabytes. Past this size decompression stops with
/// an error instead of exhausting memory.
const MAX_INFLATED_BYTES: u64 = 512 * 1024 * 1024;

fn read_all(decoder: impl Read, format: &str) -> Result<Vec<u8>, OpError> {
    let mut output = Vec::new();
    decoder
        .take(MAX_INFLATED_BYTES + 1)
        .read_to_end(&mut output)
        .map_err(|error| OpError::invalid_input(format!("Not valid {format} data: {error}")))?;
    if output.len() as u64 > MAX_INFLATED_BYTES {
        return Err(OpError::invalid_input("The decompressed data is larger than 512 MB"));
    }
    Ok(output)
}

/// What a compressed stream that was cut short decompresses to, as far as it goes.
/// Auto mode works on the start of large inputs, and uses this to see what a stream
/// holds without having all of it. `None` when the operation is not a decompression
/// or nothing came out.
pub fn inflate_prefix(op_id: &str, input: &[u8]) -> Option<Vec<u8>> {
    let mut decoder: Box<dyn Read + '_> = match op_id {
        "gunzip" => Box::new(MultiGzDecoder::new(input)),
        "zlib-inflate" => Box::new(ZlibDecoder::new(input)),
        "raw-inflate" => Box::new(DeflateDecoder::new(input)),
        _ => return None,
    };
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    // Read until the stream breaks off, keeping what came before.
    while let Ok(read @ 1..) = decoder.read(&mut buffer) {
        output.extend_from_slice(&buffer[..read]);
        if output.len() as u64 > MAX_INFLATED_BYTES {
            break;
        }
    }
    // The cut may fall inside a multi-byte character; text should still read as text.
    if let Err(error) = std::str::from_utf8(&output) {
        if error.error_len().is_none() {
            output.truncate(error.valid_up_to());
        }
    }
    (!output.is_empty()).then_some(output)
}

fn looks_like_gzip(input: &[u8]) -> bool {
    input.starts_with(&[0x1f, 0x8b, 0x08])
}

/// The two-byte zlib header: method 8, a window of at most 32 KB, and a checksum that
/// makes the pair a multiple of 31.
fn looks_like_zlib(input: &[u8]) -> bool {
    match input {
        [cmf, flags, _, ..] => {
            cmf & 0x0f == 8 && cmf >> 4 <= 7 && (u16::from(*cmf) << 8 | u16::from(*flags)) % 31 == 0
        }
        _ => false,
    }
}

macro_rules! compress_op {
    ($name:ident, $id:literal, $label:literal, $description:literal, $link:expr, $encoder:ident) => {
        pub struct $name;

        impl Op for $name {
            const ID: &'static str = $id;
            const NAME: &'static str = $label;
            const DESCRIPTION: &'static str = $description;
            const LINK: Option<&'static str> = Some($link);
            const TAGS: &'static [Tag] = &[Tag::Compression];

            type Config = NoConfig;
            type In<'a> = &'a [u8];
            type Out = Vec<u8>;

            fn run(_: &NoConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
                let mut encoder = $encoder::new(Vec::new(), Compression::default());
                // Writing into a Vec cannot fail.
                encoder.write_all(input).expect("writing to memory");
                Ok(encoder.finish().expect("writing to memory"))
            }
        }
    };
}

macro_rules! decompress_op {
    (
        $name:ident, $id:literal, $label:literal, $description:literal, $link:expr,
        $decoder:ident, $format:literal, $detect:expr
    ) => {
        pub struct $name;

        impl Op for $name {
            const ID: &'static str = $id;
            const NAME: &'static str = $label;
            const DESCRIPTION: &'static str = $description;
            const LINK: Option<&'static str> = Some($link);
            const TAGS: &'static [Tag] = &[Tag::Compression];

            type Config = NoConfig;
            type In<'a> = &'a [u8];
            type Out = Vec<u8>;

            fn detect(input: &[u8]) -> Option<Suggestion<NoConfig>> {
                let detect: fn(&[u8]) -> Option<&'static str> = $detect;
                detect(input).map(Suggestion::new)
            }

            fn run(_: &NoConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
                read_all($decoder::new(input), $format)
            }
        }
    };
}

const GZIP_LINK: &str = "https://en.wikipedia.org/wiki/Gzip";
const ZLIB_LINK: &str = "https://en.wikipedia.org/wiki/Zlib";
const DEFLATE_LINK: &str = "https://en.wikipedia.org/wiki/Deflate";

compress_op!(Gzip, "gzip", "Gzip", "Compresses the input into the gzip format", GZIP_LINK, GzEncoder);
compress_op!(
    ZlibDeflate,
    "zlib-deflate",
    "Zlib Deflate",
    "Compresses the input with DEFLATE in a zlib wrapper",
    ZLIB_LINK,
    ZlibEncoder
);
compress_op!(
    RawDeflate,
    "raw-deflate",
    "Raw Deflate",
    "Compresses the input with DEFLATE, without a header or checksum",
    DEFLATE_LINK,
    DeflateEncoder
);

decompress_op!(
    Gunzip,
    "gunzip",
    "Gunzip",
    "Decompresses gzip data",
    GZIP_LINK,
    MultiGzDecoder,
    "gzip",
    |input| looks_like_gzip(input).then_some("Starts with the gzip signature 1f 8b 08")
);
decompress_op!(
    ZlibInflate,
    "zlib-inflate",
    "Zlib Inflate",
    "Decompresses zlib data",
    ZLIB_LINK,
    ZlibDecoder,
    "zlib",
    |input| looks_like_zlib(input).then_some("Starts with a valid zlib header")
);
decompress_op!(
    RawInflate,
    "raw-inflate",
    "Raw Inflate",
    "Decompresses DEFLATE data that has no header or checksum",
    DEFLATE_LINK,
    DeflateDecoder,
    "DEFLATE",
    // Without a header there is nothing to recognize.
    |_| None
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    const TEXT: &[u8] = b"nohut nohut nohut nohut nohut nohut nohut nohut nohut nohut";

    #[test]
    fn each_format_round_trips_and_shrinks_repetitive_input() {
        let gz = Gzip::run(&NoConfig {}, TEXT).unwrap();
        let zlib = ZlibDeflate::run(&NoConfig {}, TEXT).unwrap();
        let raw = RawDeflate::run(&NoConfig {}, TEXT).unwrap();
        for compressed in [&gz, &zlib, &raw] {
            assert!(compressed.len() < TEXT.len());
        }
        assert_eq!(Gunzip::run(&NoConfig {}, &gz).unwrap(), TEXT);
        assert_eq!(ZlibInflate::run(&NoConfig {}, &zlib).unwrap(), TEXT);
        assert_eq!(RawInflate::run(&NoConfig {}, &raw).unwrap(), TEXT);
    }

    #[test]
    fn data_from_other_tools_is_read() {
        // python3 -c "import zlib; print(zlib.compress(b'hello').hex())"
        let zlib = [0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00, 0x06, 0x2c, 0x02, 0x15];
        assert_eq!(ZlibInflate::run(&NoConfig {}, &zlib).unwrap(), b"hello");
    }

    #[test]
    fn concatenated_gzip_members_are_all_read() {
        let mut both = Gzip::run(&NoConfig {}, b"first ").unwrap();
        both.extend(Gzip::run(&NoConfig {}, b"second").unwrap());
        assert_eq!(Gunzip::run(&NoConfig {}, &both).unwrap(), b"first second");
    }

    #[test]
    fn corrupt_and_truncated_data_is_an_input_error() {
        let gz = Gzip::run(&NoConfig {}, TEXT).unwrap();
        for bad in [&gz[..gz.len() - 6], b"not gzip at all".as_slice(), b""] {
            assert_eq!(Gunzip::run(&NoConfig {}, bad).unwrap_err().code, ErrorCode::InvalidInput);
        }
    }

    #[test]
    fn headers_are_detected() {
        let gz = Gzip::run(&NoConfig {}, TEXT).unwrap();
        let zlib = ZlibDeflate::run(&NoConfig {}, TEXT).unwrap();
        assert!(Gunzip::detect(&gz).is_some());
        assert!(Gunzip::detect(&zlib).is_none());
        assert!(ZlibInflate::detect(&zlib).is_some());
        assert!(ZlibInflate::detect(b"xy plain text").is_none());
        assert!(RawInflate::detect(&gz).is_none());
    }
}
