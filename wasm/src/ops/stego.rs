//! Looking for data hidden in images: after their end, in their metadata, and in the
//! lowest bits of their pixels.

use std::io::Read;

use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::analysis::signature;
use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Tag};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const LINK: &str = "https://en.wikipedia.org/wiki/Steganography";

fn be16(bytes: &[u8], at: usize) -> Option<usize> {
    bytes.get(at..at + 2).map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
}

struct Chunk<'a> {
    kind: &'a [u8],
    data: &'a [u8],
}

/// The chunks of a PNG, and where the file's own data ends.
fn png_chunks(input: &[u8]) -> Result<(Vec<Chunk<'_>>, usize), OpError> {
    if !input.starts_with(PNG_SIGNATURE) {
        return Err(OpError::invalid_input("This is not a PNG image"));
    }
    let truncated = || OpError::invalid_input("The PNG is cut off in the middle of a chunk");
    let mut chunks = Vec::new();
    let mut at = PNG_SIGNATURE.len();
    loop {
        let header = input.get(at..at + 8).ok_or_else(truncated)?;
        let length = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let data = input.get(at + 8..at + 8 + length).ok_or_else(truncated)?;
        // Length, type, data, checksum.
        at += 12 + length;
        if at > input.len() {
            return Err(truncated());
        }
        let kind = &header[4..8];
        chunks.push(Chunk { kind, data });
        if kind == b"IEND" {
            return Ok((chunks, at));
        }
    }
}

/// Where a JPEG ends: just past its end-of-image marker.
fn jpeg_end(input: &[u8]) -> Result<usize, OpError> {
    let broken = || OpError::invalid_input("The JPEG has no end-of-image marker");
    let mut at = 2;
    loop {
        if *input.get(at).ok_or_else(broken)? != 0xff {
            return Err(broken());
        }
        let marker = *input.get(at + 1).ok_or_else(broken)?;
        match marker {
            0xd9 => return Ok(at + 2),
            // Padding, and markers that stand alone.
            0xff => at += 1,
            0x01 | 0xd0..=0xd8 => at += 2,
            _ => {
                at += 2 + be16(input, at + 2).ok_or_else(broken)?;
                if marker == 0xda {
                    // Compressed image data follows a start-of-scan, up to the next
                    // marker that is not a restart or a stuffed 0xff.
                    while let Some(found) = input.get(at..).and_then(|rest| rest.iter().position(|b| *b == 0xff)) {
                        at += found;
                        match input.get(at + 1) {
                            Some(0x00 | 0xd0..=0xd7) => at += 2,
                            Some(_) => break,
                            None => return Err(broken()),
                        }
                    }
                }
            }
        }
    }
}

pub struct TrailingData;

impl Op for TrailingData {
    const ID: &'static str = "extract-trailing-data";
    const NAME: &'static str = "Extract Trailing Data";
    const DESCRIPTION: &'static str =
        "Returns whatever follows the end of a PNG or JPEG image, a common place to hide a file";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Steganography];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn run(_: &NoConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let end = match signature(input) {
            Some("PNG image") => png_chunks(input)?.1,
            Some("JPEG image") => jpeg_end(input)?,
            _ => return Err(OpError::invalid_input("This works on PNG and JPEG images")),
        };
        if end == input.len() {
            return Err(OpError::invalid_input("Nothing follows the end of the image"));
        }
        Ok(input[end..].to_vec())
    }
}

/// Bytes shown as text where they are text, and summarized where they are not.
fn readable(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) if !text.chars().any(|c| c.is_control() && !c.is_whitespace()) => text.to_owned(),
        _ => format!("({} bytes of binary data)", bytes.len()),
    }
}

fn png_metadata(input: &[u8]) -> Result<String, OpError> {
    let (chunks, end) = png_chunks(input)?;
    let mut lines = Vec::new();
    for chunk in &chunks {
        let (keyword, rest) = match chunk.data.iter().position(|byte| *byte == 0) {
            Some(nul) => (readable(&chunk.data[..nul]), &chunk.data[nul + 1..]),
            None => continue,
        };
        match chunk.kind {
            b"tEXt" => lines.push(format!("{keyword}: {}", readable(rest))),
            // One byte naming the compression method, then zlib data.
            b"zTXt" => {
                let mut text = Vec::new();
                let inflated = ZlibDecoder::new(rest.get(1..).unwrap_or_default())
                    .take(1 << 20)
                    .read_to_end(&mut text);
                lines.push(match inflated {
                    Ok(_) => format!("{keyword}: {}", readable(&text)),
                    Err(_) => format!("{keyword}: (compressed text that does not decompress)"),
                });
            }
            // Flags, a language tag and a translated keyword precede the text.
            b"iTXt" => {
                let compressed = rest.first() == Some(&1);
                let mut fields = rest.get(2..).unwrap_or_default().splitn(3, |byte| *byte == 0);
                let text = fields.nth(2).unwrap_or_default();
                lines.push(if compressed {
                    format!("{keyword}: (compressed international text, {} bytes)", text.len())
                } else {
                    format!("{keyword}: {}", readable(text))
                });
            }
            _ => {}
        }
    }
    if lines.is_empty() {
        lines.push("No text in the image's metadata".into());
    }
    let kinds: Vec<String> = chunks
        .iter()
        .map(|chunk| format!("{} ({})", String::from_utf8_lossy(chunk.kind), chunk.data.len()))
        .collect();
    lines.push(format!("\nChunks: {}", kinds.join(", ")));
    if end < input.len() {
        lines.push(format!("{} bytes follow the end of the image", input.len() - end));
    }
    Ok(lines.join("\n"))
}

fn jpeg_metadata(input: &[u8]) -> Result<String, OpError> {
    let mut lines = Vec::new();
    let mut at = 2;
    // Metadata sits in the segments before the image data starts.
    while let (Some(0xff), Some(marker)) = (input.get(at), input.get(at + 1)) {
        let Some(length) = be16(input, at + 2) else { break };
        let body = input.get(at + 4..at + 2 + length).unwrap_or_default();
        match marker {
            0xfe => lines.push(format!("Comment: {}", readable(body))),
            0xe1 if body.starts_with(b"Exif\0") => {
                lines.push(format!("EXIF data, {} bytes", body.len()));
            }
            0xe1 if body.starts_with(b"http://ns.adobe.com/xap/") => {
                lines.push(format!("XMP data, {} bytes", body.len()));
            }
            0xda => break,
            _ => {}
        }
        at += 2 + length;
    }
    if lines.is_empty() {
        lines.push("No comment, EXIF or XMP data in the image".into());
    }
    if let Ok(end) = jpeg_end(input) {
        if end < input.len() {
            lines.push(format!("{} bytes follow the end of the image", input.len() - end));
        }
    }
    Ok(lines.join("\n"))
}

pub struct ImageMetadata;

impl Op for ImageMetadata {
    const ID: &'static str = "image-metadata";
    const NAME: &'static str = "Image Metadata";
    const DESCRIPTION: &'static str =
        "Lists the text, comments and chunks stored in a PNG or JPEG besides its pixels";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Steganography, Tag::Analysis];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        match signature(input) {
            Some("PNG image") => png_metadata(input),
            Some("JPEG image") => jpeg_metadata(input),
            _ => Err(OpError::invalid_input("This works on PNG and JPEG images")),
        }
    }
}

/// The largest image whose pixels are decoded: 256 MB of them.
const MAX_PIXEL_BYTES: usize = 256 * 1024 * 1024;

struct Pixels {
    /// Red, green, blue, alpha for every pixel, row by row.
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

/// Decodes the pixels of an 8-bit, non-interlaced PNG. That covers what image editors
/// and steganography tools write; other variants are reported, not guessed at.
fn png_pixels(input: &[u8]) -> Result<Pixels, OpError> {
    let (chunks, _) = png_chunks(input)?;
    let unsupported = |what: &str| OpError::invalid_input(format!("{what} PNG images are not supported"));
    let header = chunks
        .first()
        .filter(|chunk| chunk.kind == b"IHDR" && chunk.data.len() == 13)
        .ok_or_else(|| OpError::invalid_input("The PNG has no header chunk"))?
        .data;
    let width = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let height = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let (depth, color_type, interlaced) = (header[8], header[9], header[12] != 0);
    if depth != 8 {
        return Err(unsupported(&format!("{depth}-bit")));
    }
    if interlaced {
        return Err(unsupported("Interlaced"));
    }
    let channels = match color_type {
        0 | 3 => 1,
        4 => 2,
        2 => 3,
        6 => 4,
        other => return Err(OpError::invalid_input(format!("Unknown PNG colour type {other}"))),
    };
    let row_bytes = width.checked_mul(channels).filter(|row| *row > 0);
    let total = row_bytes
        .and_then(|row| (row + 1).checked_mul(height))
        .filter(|total| *total <= MAX_PIXEL_BYTES && height > 0)
        .ok_or_else(|| OpError::invalid_input("The image is empty or too large to decode"))?;
    let row_bytes = row_bytes.expect("checked with the total");

    let compressed: Vec<u8> = chunks
        .iter()
        .filter(|chunk| chunk.kind == b"IDAT")
        .flat_map(|chunk| chunk.data.iter().copied())
        .collect();
    let mut raw = Vec::with_capacity(total);
    ZlibDecoder::new(compressed.as_slice())
        .take(total as u64)
        .read_to_end(&mut raw)
        .map_err(|_| OpError::invalid_input("The PNG's pixel data does not decompress"))?;
    if raw.len() != total {
        return Err(OpError::invalid_input("The PNG holds fewer pixels than its header says"));
    }

    // Each row was stored as differences from its neighbours; add them back.
    let mut previous = vec![0u8; row_bytes];
    let mut samples = Vec::with_capacity(row_bytes * height);
    for row in raw.chunks_exact(row_bytes + 1) {
        let (filter, row) = (row[0], &row[1..]);
        let mut current = vec![0u8; row_bytes];
        for index in 0..row_bytes {
            let left = if index >= channels { current[index - channels] } else { 0 };
            let up = previous[index];
            let up_left = if index >= channels { previous[index - channels] } else { 0 };
            let predicted = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                4 => {
                    let (a, b, c) = (i16::from(left), i16::from(up), i16::from(up_left));
                    let estimate = a + b - c;
                    let (da, db, dc) = ((estimate - a).abs(), (estimate - b).abs(), (estimate - c).abs());
                    if da <= db && da <= dc {
                        left
                    } else if db <= dc {
                        up
                    } else {
                        up_left
                    }
                }
                other => return Err(OpError::invalid_input(format!("Unknown PNG row filter {other}"))),
            };
            current[index] = row[index].wrapping_add(predicted);
        }
        samples.extend_from_slice(&current);
        previous = current;
    }

    let palette = chunks.iter().find(|chunk| chunk.kind == b"PLTE").map(|chunk| chunk.data);
    let mut rgba = Vec::with_capacity(width * height * 4);
    for pixel in samples.chunks_exact(channels) {
        rgba.extend_from_slice(&match (color_type, pixel) {
            (0, [gray]) => [*gray, *gray, *gray, 255],
            (4, [gray, alpha]) => [*gray, *gray, *gray, *alpha],
            (2, [r, g, b]) => [*r, *g, *b, 255],
            (6, [r, g, b, a]) => [*r, *g, *b, *a],
            (3, [index]) => {
                let at = usize::from(*index) * 3;
                let entry = palette
                    .and_then(|palette| palette.get(at..at + 3))
                    .ok_or_else(|| OpError::invalid_input("A pixel refers to a colour missing from the palette"))?;
                [entry[0], entry[1], entry[2], 255]
            }
            _ => unreachable!("chunks_exact yields the channel count"),
        });
    }
    Ok(Pixels { rgba, width, height })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum PixelOrder {
    /// Left to right, then down: the usual order.
    #[default]
    Rows,
    /// Top to bottom, then right.
    Columns,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct LsbConfig {
    /// Which values of each pixel to read, in order: any of r, g, b, a.
    pub channels: String,
    /// Which bit of each value: 0 is the lowest.
    pub bit: u8,
    pub order: PixelOrder,
}

impl Default for LsbConfig {
    fn default() -> Self {
        LsbConfig { channels: "rgb".into(), bit: 0, order: PixelOrder::Rows }
    }
}

pub struct LsbExtract;

impl Op for LsbExtract {
    const ID: &'static str = "lsb-extract";
    const NAME: &'static str = "Extract LSB (PNG)";
    const DESCRIPTION: &'static str =
        "Collects one bit from each colour value of a PNG's pixels into bytes: where LSB steganography hides data";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Bit_numbering#Least_significant_bit_in_digital_steganography");
    const TAGS: &'static [Tag] = &[Tag::Steganography];

    type Config = LsbConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        let orders = ["rows", "columns"].map(String::from).to_vec();
        vec![
            ParamSpec::new("channels", "Channels", ParamKind::Text),
            ParamSpec::new("bit", "Bit", ParamKind::Int { min: Some(0), max: Some(7) }),
            ParamSpec::new("order", "Pixel order", ParamKind::Choice { options: orders }),
        ]
    }

    fn run(config: &LsbConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let offsets: Vec<usize> = config
            .channels
            .chars()
            .map(|channel| "rgba".find(channel.to_ascii_lowercase()))
            .collect::<Option<_>>()
            .filter(|offsets: &Vec<usize>| !offsets.is_empty())
            .ok_or_else(|| OpError::invalid_config("Channels is a combination of r, g, b and a, such as rgb"))?;
        if config.bit > 7 {
            return Err(OpError::invalid_config("Bit is a number from 0 to 7"));
        }
        let Pixels { rgba, width, height } = png_pixels(input)?;

        let mut output = Vec::with_capacity(width * height * offsets.len() / 8);
        let (mut byte, mut filled) = (0u8, 0u8);
        let mut take = |pixel: usize| {
            for offset in &offsets {
                // The first bit read is the highest bit of the byte it goes into.
                byte = byte << 1 | (rgba[pixel * 4 + offset] >> config.bit & 1);
                filled += 1;
                if filled == 8 {
                    output.push(byte);
                    (byte, filled) = (0, 0);
                }
            }
        };
        match config.order {
            PixelOrder::Rows => (0..width * height).for_each(&mut take),
            PixelOrder::Columns => {
                (0..width).for_each(|x| (0..height).for_each(|y| take(y * width + x)));
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut out = (data.len() as u32).to_be_bytes().to_vec();
        out.extend(kind);
        out.extend(data);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(kind);
        hasher.update(data);
        out.extend(hasher.finalize().to_be_bytes());
        out
    }

    /// Builds a PNG from raw samples, storing every row with the given filter.
    fn png(width: u32, height: u32, color_type: u8, samples: &[u8], filter: u8, extra: &[Vec<u8>]) -> Vec<u8> {
        let channels = match color_type { 0 | 3 => 1, 4 => 2, 2 => 3, _ => 4 };
        let row_bytes = width as usize * channels;
        let mut raw = Vec::new();
        let mut previous = vec![0u8; row_bytes];
        for row in samples.chunks(row_bytes) {
            raw.push(filter);
            for (index, value) in row.iter().enumerate() {
                let left = if index >= channels { row[index - channels] } else { 0 };
                let up = previous[index];
                let up_left = if index >= channels { previous[index - channels] } else { 0 };
                let predicted = match filter {
                    0 => 0,
                    1 => left,
                    2 => up,
                    3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                    _ => {
                        let (a, b, c) = (i16::from(left), i16::from(up), i16::from(up_left));
                        let p = a + b - c;
                        let (da, db, dc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                        if da <= db && da <= dc { left } else if db <= dc { up } else { up_left }
                    }
                };
                raw.push(value.wrapping_sub(predicted));
            }
            previous = row.to_vec();
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut header = width.to_be_bytes().to_vec();
        header.extend(height.to_be_bytes());
        header.extend([8, color_type, 0, 0, 0]);
        let mut out = PNG_SIGNATURE.to_vec();
        out.extend(chunk(b"IHDR", &header));
        for extra in extra {
            out.extend(extra);
        }
        // Split across two chunks, as large images are.
        let (first, second) = compressed.split_at(compressed.len() / 2);
        out.extend(chunk(b"IDAT", first));
        out.extend(chunk(b"IDAT", second));
        out.extend(chunk(b"IEND", &[]));
        out
    }

    /// RGB samples for a 16 x 8 image whose lowest bits spell `message`.
    fn carrier(message: &[u8]) -> Vec<u8> {
        let bits: Vec<u8> = message.iter().flat_map(|byte| (0..8).rev().map(move |i| byte >> i & 1)).collect();
        (0..16 * 8 * 3)
            .map(|index| {
                let colour = (index * 37 % 251) as u8 & 0xfe;
                colour | bits.get(index).copied().unwrap_or(0)
            })
            .collect()
    }

    #[test]
    fn a_message_in_the_lowest_bits_is_read_back_under_every_row_filter() {
        let message = b"ctf{h1dden_1n_pl41n_s1ght}";
        for filter in 0..=4 {
            let image = png(16, 8, 2, &carrier(message), filter, &[]);
            let extracted = LsbExtract::run(&LsbConfig::default(), &image).unwrap();
            assert_eq!(extracted.len(), 16 * 8 * 3 / 8);
            assert!(extracted.starts_with(message), "filter {filter}");
        }
    }

    #[test]
    fn channels_bit_and_order_select_what_is_read() {
        // Two pixels side by side, then two below: RGBA values chosen bit by bit.
        let samples = [
            0b01, 0b10, 0b00, 0b11, /**/ 0b11, 0b00, 0b10, 0b01,
            0b00, 0b11, 0b01, 0b10, /**/ 0b10, 0b01, 0b11, 0b00,
        ];
        let image = png(2, 2, 6, &samples, 0, &[]);
        let run = |channels: &str, bit: u8, order: PixelOrder| {
            LsbExtract::run(&LsbConfig { channels: channels.into(), bit, order }, &image).unwrap()
        };
        // Red and alpha, lowest bit, by rows: 1,1 1,1 0,0 0,0.
        assert_eq!(run("ra", 0, PixelOrder::Rows), [0b1111_0000]);
        // The same by columns: pixels 0, 2, 1, 3.
        assert_eq!(run("ra", 0, PixelOrder::Columns), [0b1100_1100]);
        // Bit 1 of green and blue by rows: 1,0 0,1 1,0 0,1.
        assert_eq!(run("gb", 1, PixelOrder::Rows), [0b1001_1001]);
        // Four pixels of one channel are half a byte: nothing complete to return.
        assert!(run("r", 0, PixelOrder::Rows).is_empty());
    }

    #[test]
    fn grayscale_and_palette_images_are_decoded() {
        let gray = png(8, 1, 0, &[1, 0, 1, 0, 1, 0, 1, 1], 0, &[]);
        let config = LsbConfig { channels: "r".into(), ..LsbConfig::default() };
        assert_eq!(LsbExtract::run(&config, &gray).unwrap(), [0b1010_1011]);
        // Palette entry 0 is odd in red, entry 1 is even.
        let palette = chunk(b"PLTE", &[1, 0, 0, 2, 0, 0]);
        let indexed = png(8, 1, 3, &[0, 0, 1, 1, 0, 1, 0, 1], 0, &[palette]);
        assert_eq!(LsbExtract::run(&config, &indexed).unwrap(), [0b1100_1010]);
    }

    #[test]
    fn unsupported_and_broken_images_are_explained() {
        let mut sixteen_bit = png(1, 1, 2, &[0, 0, 0], 0, &[]);
        sixteen_bit[24] = 16;
        let error = LsbExtract::run(&LsbConfig::default(), &sixteen_bit).unwrap_err();
        assert!(error.message.contains("16-bit"), "{}", error.message);
        let not_png = LsbExtract::run(&LsbConfig::default(), b"GIF89a").unwrap_err();
        assert_eq!(not_png.code, ErrorCode::InvalidInput);
        let bad = LsbConfig { channels: "xyz".into(), ..LsbConfig::default() };
        let image = png(1, 1, 2, &[0, 0, 0], 0, &[]);
        assert_eq!(LsbExtract::run(&bad, &image).unwrap_err().code, ErrorCode::InvalidConfig);
        let cut = &image[..image.len() - 20];
        assert_eq!(LsbExtract::run(&LsbConfig::default(), cut).unwrap_err().code, ErrorCode::InvalidInput);
    }

    #[test]
    fn data_after_the_end_of_an_image_is_returned() {
        let mut image = png(1, 1, 2, &[0, 0, 0], 0, &[]);
        let nothing = TrailingData::run(&NoConfig {}, &image).unwrap_err();
        assert!(nothing.message.contains("Nothing follows"));
        image.extend(b"PK\x03\x04 hidden archive");
        assert_eq!(TrailingData::run(&NoConfig {}, &image).unwrap(), b"PK\x03\x04 hidden archive");

        // SOI, a 2-byte comment, a scan with a stuffed ff and a restart marker, EOI.
        let mut jpeg = b"\xff\xd8\xff\xfe\x00\x04hi\xff\xda\x00\x02\x12\xff\x00\x34\xff\xd0\x56\xff\xd9".to_vec();
        assert!(TrailingData::run(&NoConfig {}, &jpeg).is_err());
        jpeg.extend(b"secret \xff\xd9 more");
        assert_eq!(TrailingData::run(&NoConfig {}, &jpeg).unwrap(), b"secret \xff\xd9 more");
    }

    #[test]
    fn text_stored_beside_the_pixels_is_listed() {
        let text = chunk(b"tEXt", b"Comment\0the key is nohut");
        let mut compressed = b"Hidden\0\0".to_vec();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"compressed note").unwrap();
        compressed.extend(encoder.finish().unwrap());
        let international = chunk(b"iTXt", b"Title\0\0\0tr\0Ba\xc5\x9fl\xc4\xb1k\0nohut resmi");
        let mut image = png(1, 1, 2, &[0, 0, 0], 0, &[text, chunk(b"zTXt", &compressed), international]);
        image.extend(b"12345");
        let listed = ImageMetadata::run(&NoConfig {}, &image).unwrap();
        assert!(listed.contains("Comment: the key is nohut"), "{listed}");
        assert!(listed.contains("Hidden: compressed note"), "{listed}");
        assert!(listed.contains("Title: nohut resmi"), "{listed}");
        assert!(listed.contains("Chunks: IHDR (13), tEXt (24), zTXt"), "{listed}");
        assert!(listed.contains("5 bytes follow the end of the image"), "{listed}");

        let jpeg = b"\xff\xd8\xff\xfe\x00\x0bsee below\xff\xda\x00\x02\xff\xd9tail";
        let listed = ImageMetadata::run(&NoConfig {}, jpeg).unwrap();
        assert_eq!(listed, "Comment: see below\n4 bytes follow the end of the image");
    }
}
