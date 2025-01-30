
use wasm_bindgen::prelude::*;
use image::{DynamicImage, ImageFormat, RgbaImage, RgbImage};
enum ImageType {
    Rgb,
    Rgba,
}

#[wasm_bindgen]
pub fn transpose_image(image_data: &[u8]) -> Vec<u8> {
    let img = image::load_from_memory(image_data).expect("Invalid image data");
    let image_type = match img {
        DynamicImage::ImageRgba8(_) => ImageType::Rgba,
        DynamicImage::ImageRgb8(_) => ImageType::Rgb,
        _ => panic!("Unsupported image format"),
    };
    let transposed = match img {
        DynamicImage::ImageRgba8(img) => transpose_rgba_image(&img),
        DynamicImage::ImageRgb8(img) => transpose_rgb_image(&img),
        _ => panic!("Unsupported image format"),
    };

    let mut output_bytes: Vec<u8> = Vec::new();
    let mut cursor = 
    std::io::Cursor::new(&mut output_bytes);
    match image_type {
        ImageType::Rgba => {
            transposed
                .write_to(&mut cursor, ImageFormat::Png)
                .expect("Failed to write image");
        }
        ImageType::Rgb => {
            transposed
                .write_to(&mut cursor, ImageFormat::Jpeg)
                .expect("Failed to write image");
        }
    }
    output_bytes
}
fn transpose_rgb_image(img: &RgbImage) -> DynamicImage {
    let (width, height) = img.dimensions();
    let mut transposed = RgbImage::new(height, width);

    for x in 0..width {
        for y in 0..height {
            transposed.put_pixel(y, x, *img.get_pixel(x, y));
        }
    }

    DynamicImage::ImageRgb8(transposed)
}
fn transpose_rgba_image(img: &RgbaImage) -> DynamicImage {
    let (width, height) = img.dimensions();
    let mut transposed = RgbaImage::new(height, width);

    for x in 0..width {
        for y in 0..height {
            transposed.put_pixel(y, x, *img.get_pixel(x, y));
        }
    }

    DynamicImage::ImageRgba8(transposed)
}
