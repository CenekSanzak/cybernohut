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
    /// False when the image holds more than eight bits per value and only the low
    /// eight are here: enough to read hidden bits, not to write the image back.
    exact: bool,
}

/// Undoes PNG row filtering for `rows` rows of `row_bytes` bytes taken from the front
/// of `raw`, where a pixel is `pixel_bytes` wide. Each row was stored as differences
/// from its neighbours; this adds them back.
fn unfilter(raw: &mut &[u8], row_bytes: usize, rows: usize, pixel_bytes: usize) -> Result<Vec<u8>, OpError> {
    let short = || OpError::invalid_input("The PNG holds fewer pixels than its header says");
    let mut previous = vec![0u8; row_bytes];
    let mut samples = Vec::with_capacity(row_bytes * rows);
    for _ in 0..rows {
        let (row, rest) = raw.split_at_checked(row_bytes + 1).ok_or_else(short)?;
        *raw = rest;
        let (filter, row) = (row[0], &row[1..]);
        let mut current = vec![0u8; row_bytes];
        for index in 0..row_bytes {
            let left = if index >= pixel_bytes { current[index - pixel_bytes] } else { 0 };
            let up = previous[index];
            let up_left = if index >= pixel_bytes { previous[index - pixel_bytes] } else { 0 };
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
    Ok(samples)
}

/// The seven passes of an interlaced PNG: where each starts and how far apart its
/// pixels are, across and down.
const ADAM7: [(usize, usize, usize, usize); 7] =
    [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)];

/// Decodes the pixels of a PNG with 8 or 16 bits per value. Images with fewer bits
/// per value are reported, not guessed at.
fn png_pixels(input: &[u8]) -> Result<Pixels, OpError> {
    let (chunks, _) = png_chunks(input)?;
    let header = chunks
        .first()
        .filter(|chunk| chunk.kind == b"IHDR" && chunk.data.len() == 13)
        .ok_or_else(|| OpError::invalid_input("The PNG has no header chunk"))?
        .data;
    let width = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let height = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let (depth, color_type, interlaced) = (header[8], header[9], header[12] != 0);
    if depth != 8 && depth != 16 {
        return Err(OpError::invalid_input(format!("{depth}-bit PNG images are not supported")));
    }
    let channels = match color_type {
        0 | 3 => 1,
        4 => 2,
        2 => 3,
        6 => 4,
        other => return Err(OpError::invalid_input(format!("Unknown PNG colour type {other}"))),
    };
    let value_bytes = usize::from(depth / 8);
    let pixel_bytes = channels * value_bytes;
    let too_large = || OpError::invalid_input("The image is empty or too large to decode");
    let stored = |columns: usize, rows: usize| {
        columns.checked_mul(pixel_bytes).and_then(|row| (row + 1).checked_mul(rows))
    };
    let passes: Vec<(usize, usize, usize, usize)> =
        if interlaced { ADAM7.to_vec() } else { vec![(0, 0, 1, 1)] };
    let pass_size = |&(x0, y0, dx, dy): &(usize, usize, usize, usize)| {
        (width.saturating_sub(x0).div_ceil(dx), height.saturating_sub(y0).div_ceil(dy))
    };
    let total = passes
        .iter()
        .map(pass_size)
        .filter(|(columns, rows)| *columns > 0 && *rows > 0)
        .try_fold(0usize, |sum, (columns, rows)| sum.checked_add(stored(columns, rows)?))
        .filter(|total| *total > 0 && *total <= MAX_PIXEL_BYTES)
        .ok_or_else(too_large)?;

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

    // One value per channel, in place in the full image. Of a 16-bit value the low
    // byte is kept, which is where its lowest bits are.
    let mut values = vec![0u8; width * height * channels];
    let mut rest = raw.as_slice();
    for pass in &passes {
        let (columns, rows) = pass_size(pass);
        if columns == 0 || rows == 0 {
            continue;
        }
        let (x0, y0, dx, dy) = *pass;
        let samples = unfilter(&mut rest, columns * pixel_bytes, rows, pixel_bytes)?;
        for (index, pixel) in samples.chunks_exact(pixel_bytes).enumerate() {
            let (x, y) = (x0 + index % columns * dx, y0 + index / columns * dy);
            for channel in 0..channels {
                values[(y * width + x) * channels + channel] =
                    pixel[channel * value_bytes + value_bytes - 1];
            }
        }
    }

    let palette = chunks.iter().find(|chunk| chunk.kind == b"PLTE").map(|chunk| chunk.data);
    let mut rgba = Vec::with_capacity(width * height * 4);
    for pixel in values.chunks_exact(channels) {
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
    Ok(Pixels { rgba, width, height, exact: depth == 8 })
}

/// Decodes an uncompressed 24- or 32-bit BMP, the kind paint programs write.
fn bmp_pixels(input: &[u8]) -> Result<Pixels, OpError> {
    let broken = || OpError::invalid_input("The BMP header is incomplete");
    let le32 = |at: usize| input.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let start = le32(10).ok_or_else(broken)? as usize;
    let (width, height) = (le32(18).ok_or_else(broken)? as i32, le32(22).ok_or_else(broken)? as i32);
    let bits = input.get(28..30).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(broken)?;
    let compression = le32(30).ok_or_else(broken)?;
    let pixel_bytes = match (bits, compression) {
        (24, 0) => 3,
        (32, 0 | 3) => 4,
        _ => {
            return Err(OpError::invalid_input(format!(
                "Only uncompressed 24- and 32-bit BMP images are supported; this one is {bits}-bit"
            )))
        }
    };
    let (columns, rows) = (width.unsigned_abs() as usize, height.unsigned_abs() as usize);
    // Rows are padded to a multiple of four bytes.
    let row_bytes = columns.checked_mul(pixel_bytes).map(|row| row.div_ceil(4) * 4);
    let data = row_bytes
        .and_then(|row| row.checked_mul(rows))
        .filter(|total| *total > 0 && *total <= MAX_PIXEL_BYTES)
        .and_then(|total| input.get(start..start.checked_add(total)?))
        .ok_or_else(|| OpError::invalid_input("The BMP is empty, too large, or cut short"))?;
    let row_bytes = row_bytes.expect("checked with the total");

    let mut rgba = Vec::with_capacity(columns * rows * 4);
    for y in 0..rows {
        // A positive height means the bottom row comes first.
        let stored = if height > 0 { rows - 1 - y } else { y };
        let row = &data[stored * row_bytes..][..columns * pixel_bytes];
        for pixel in row.chunks_exact(pixel_bytes) {
            let alpha = if pixel_bytes == 4 { pixel[3] } else { 255 };
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], alpha]);
        }
    }
    Ok(Pixels { rgba, width: columns, height: rows, exact: true })
}

fn pixels(input: &[u8]) -> Result<Pixels, OpError> {
    match signature(input) {
        Some("PNG image") => png_pixels(input),
        Some("BMP image") => bmp_pixels(input),
        _ => Err(OpError::invalid_input("This works on PNG and BMP images, whose pixels are stored exactly")),
    }
}

fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
    chunk.extend(kind);
    chunk.extend(data);
    let mut checksum = crc32fast::Hasher::new();
    checksum.update(kind);
    checksum.update(data);
    chunk.extend(checksum.finalize().to_be_bytes());
    chunk
}

/// Writes pixels as an 8-bit RGBA PNG. PNG is lossless, so every bit survives.
fn encode_png(pixels: &Pixels) -> Vec<u8> {
    let mut raw = Vec::with_capacity((pixels.width * 4 + 1) * pixels.height);
    for row in pixels.rgba.chunks_exact(pixels.width * 4) {
        // Filter 0: the row as it is.
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &raw).expect("writing to memory");
    let compressed = encoder.finish().expect("writing to memory");

    let mut header = (pixels.width as u32).to_be_bytes().to_vec();
    header.extend((pixels.height as u32).to_be_bytes());
    // Eight bits per value, RGBA, and the only compression, filter and interlace
    // methods there are.
    header.extend([8, 6, 0, 0, 0]);
    let mut png = PNG_SIGNATURE.to_vec();
    png.extend(png_chunk(b"IHDR", &header));
    png.extend(png_chunk(b"IDAT", &compressed));
    png.extend(png_chunk(b"IEND", &[]));
    png
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

fn lsb_params() -> Vec<ParamSpec> {
    let orders = ["rows", "columns"].map(String::from).to_vec();
    vec![
        ParamSpec::new("channels", "Channels", ParamKind::Text),
        ParamSpec::new("bit", "Bit", ParamKind::Int { min: Some(0), max: Some(7) }),
        ParamSpec::new("order", "Pixel order", ParamKind::Choice { options: orders }),
    ]
}

/// The position in the RGBA data of every value the settings select, in reading order.
fn selected_values(config: &LsbConfig, width: usize, height: usize) -> Result<Vec<usize>, OpError> {
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
    let order: Vec<usize> = match config.order {
        PixelOrder::Rows => (0..width * height).collect(),
        PixelOrder::Columns => (0..width).flat_map(|x| (0..height).map(move |y| y * width + x)).collect(),
    };
    Ok(order.iter().flat_map(|pixel| offsets.iter().map(move |offset| pixel * 4 + offset)).collect())
}

pub struct LsbExtract;

impl Op for LsbExtract {
    const ID: &'static str = "lsb-extract";
    const NAME: &'static str = "Extract LSB";
    const DESCRIPTION: &'static str =
        "Collects one bit from each colour value of a PNG or BMP image's pixels into bytes: where LSB steganography hides data";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Bit_numbering#Least_significant_bit_in_digital_steganography");
    const TAGS: &'static [Tag] = &[Tag::Steganography];

    type Config = LsbConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        lsb_params()
    }

    fn run(config: &LsbConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let Pixels { rgba, width, height, .. } = pixels(input)?;
        let values = selected_values(config, width, height)?;
        // The first bit read is the highest bit of the byte it goes into.
        Ok(values
            .chunks_exact(8)
            .map(|group| group.iter().fold(0u8, |byte, at| byte << 1 | (rgba[*at] >> config.bit & 1)))
            .collect())
    }
}

pub struct LsbEmbed;

impl Op for LsbEmbed {
    const ID: &'static str = "lsb-embed";
    const NAME: &'static str = "Embed LSB";
    const DESCRIPTION: &'static str =
        "Hides data in one bit of each colour value of an image and writes the result as a PNG. Extract LSB with the same settings reads it back";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Bit_numbering#Least_significant_bit_in_digital_steganography");
    const TAGS: &'static [Tag] = &[Tag::Steganography];
    const INPUTS: &'static [&'static str] = &["image", "data"];

    type Config = LsbConfig;
    type In<'a> = (&'a [u8], &'a [u8]);
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        lsb_params()
    }

    fn run(config: &LsbConfig, (image, data): Self::In<'_>) -> Result<Vec<u8>, OpError> {
        let mut carrier = pixels(image)?;
        if !carrier.exact {
            return Err(OpError::invalid_input(
                "A 16-bit image cannot be written back; convert it to 8-bit first",
            ));
        }
        let values = selected_values(config, carrier.width, carrier.height)?;
        if data.len() * 8 > values.len() {
            return Err(OpError::invalid_input(format!(
                "The data is {} bytes, but with these settings the image holds only {}",
                data.len(),
                values.len() / 8
            )));
        }
        let bits = data.iter().flat_map(|byte| (0..8).rev().map(move |shift| byte >> shift & 1));
        for (at, bit) in values.iter().zip(bits) {
            carrier.rgba[*at] = carrier.rgba[*at] & !(1 << config.bit) | bit << config.bit;
        }
        Ok(encode_png(&carrier))
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
        let mut four_bit = png(1, 1, 0, &[0], 0, &[]);
        four_bit[24] = 4;
        let error = LsbExtract::run(&LsbConfig::default(), &four_bit).unwrap_err();
        assert!(error.message.contains("4-bit"), "{}", error.message);
        let not_png = LsbExtract::run(&LsbConfig::default(), b"GIF89a").unwrap_err();
        assert_eq!(not_png.code, ErrorCode::InvalidInput);
        let bad = LsbConfig { channels: "xyz".into(), ..LsbConfig::default() };
        let image = png(1, 1, 2, &[0, 0, 0], 0, &[]);
        assert_eq!(LsbExtract::run(&bad, &image).unwrap_err().code, ErrorCode::InvalidConfig);
        let cut = &image[..image.len() - 20];
        assert_eq!(LsbExtract::run(&LsbConfig::default(), cut).unwrap_err().code, ErrorCode::InvalidInput);
    }

    #[test]
    fn embedding_writes_a_png_that_extraction_reads_back() {
        let image = png(16, 8, 2, &carrier(b""), 2, &[]);
        let secret = b"ctf{wr1tten_and_r3ad}";
        for config in [
            LsbConfig::default(),
            LsbConfig { channels: "bg".into(), bit: 1, order: PixelOrder::Columns },
        ] {
            let hidden = LsbEmbed::run(&config, (&image, secret)).unwrap();
            assert!(hidden.starts_with(PNG_SIGNATURE));
            assert!(LsbExtract::run(&config, &hidden).unwrap().starts_with(secret), "{config:?}");
            // Only the chosen bit of the chosen values changed.
            let (before, after) = (pixels(&image).unwrap(), pixels(&hidden).unwrap());
            let mask = !(1u8 << config.bit);
            assert!(before.rgba.iter().zip(&after.rgba).all(|(a, b)| a & mask == b & mask));
        }
        let too_much = LsbEmbed::run(&LsbConfig::default(), (&image, &[0u8; 49])).unwrap_err();
        assert!(too_much.message.contains("holds only 48"), "{}", too_much.message);
    }

    #[test]
    fn an_interlaced_image_decodes_to_the_same_pixels() {
        let (width, height) = (11usize, 9usize);
        let samples: Vec<u8> = (0..width * height * 3).map(|i| (i * 53 % 256) as u8).collect();
        // Store the same pixels pass by pass, each row unfiltered.
        let mut raw = Vec::new();
        for (x0, y0, dx, dy) in ADAM7 {
            for y in (y0..height).step_by(dy) {
                if x0 >= width {
                    continue;
                }
                raw.push(0);
                for x in (x0..width).step_by(dx) {
                    raw.extend_from_slice(&samples[(y * width + x) * 3..][..3]);
                }
            }
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        let mut header = (width as u32).to_be_bytes().to_vec();
        header.extend((height as u32).to_be_bytes());
        header.extend([8, 2, 0, 0, 1]);
        let mut interlaced = PNG_SIGNATURE.to_vec();
        interlaced.extend(chunk(b"IHDR", &header));
        interlaced.extend(chunk(b"IDAT", &encoder.finish().unwrap()));
        interlaced.extend(chunk(b"IEND", &[]));

        let plain = png(width as u32, height as u32, 2, &samples, 0, &[]);
        assert_eq!(pixels(&interlaced).unwrap().rgba, pixels(&plain).unwrap().rgba);
    }

    #[test]
    fn sixteen_bit_values_contribute_their_low_byte() {
        // One pixel, RGB, two bytes per value: the low bytes are 1, 0, 1.
        let mut raw = vec![0u8];
        raw.extend([0xff, 0x01, 0xff, 0x00, 0xff, 0x01]);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        let mut header = 1u32.to_be_bytes().to_vec();
        header.extend(1u32.to_be_bytes());
        header.extend([16, 2, 0, 0, 0]);
        let mut image = PNG_SIGNATURE.to_vec();
        image.extend(chunk(b"IHDR", &header));
        image.extend(chunk(b"IDAT", &encoder.finish().unwrap()));
        image.extend(chunk(b"IEND", &[]));
        let decoded = pixels(&image).unwrap();
        assert_eq!(decoded.rgba, [1, 0, 1, 255]);
        assert!(!decoded.exact);
        assert!(LsbEmbed::run(&LsbConfig::default(), (&image, b"")).unwrap_err().message.contains("16-bit"));
    }

    #[test]
    fn a_bmp_is_read_bottom_up_with_blue_first() {
        // 2 x 2, 24-bit: rows padded to 8 bytes, the bottom row stored first.
        let mut bmp = b"BM".to_vec();
        bmp.extend([0u8; 8]);
        bmp.extend(54u32.to_le_bytes());
        bmp.extend(40u32.to_le_bytes());
        bmp.extend(2i32.to_le_bytes());
        bmp.extend(2i32.to_le_bytes());
        bmp.extend(1u16.to_le_bytes());
        bmp.extend(24u16.to_le_bytes());
        bmp.extend([0u8; 24]);
        // Bottom row: (r3 g2 b1), (r6 g5 b4). Top row: (r9 g8 b7), (r12 g11 b10).
        bmp.extend([1, 2, 3, 4, 5, 6, 0, 0]);
        bmp.extend([7, 8, 9, 10, 11, 12, 0, 0]);
        let decoded = pixels(&bmp).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 2));
        assert_eq!(decoded.rgba, [9, 8, 7, 255, 12, 11, 10, 255, 3, 2, 1, 255, 6, 5, 4, 255]);
        // Red of the four pixels, low bit: 1, 0, 1, 0, then green: 0, 1, 0, 1.
        let config = LsbConfig { channels: "rg".into(), bit: 0, order: PixelOrder::Rows };
        assert_eq!(LsbExtract::run(&config, &bmp).unwrap(), [0b1001_1001]);
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
