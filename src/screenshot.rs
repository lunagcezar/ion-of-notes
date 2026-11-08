//! Converts a rendered `egui::ColorImage` into a PNG file.
//!
//! This powers `--screenshot`: the harness runs the app with that flag after
//! every build and checks that the resulting picture is not blank.

use egui::ColorImage;
use std::io;
use std::path::Path;

/// Flatten the image into interleaved RGBA bytes plus its dimensions.
pub fn color_image_to_rgba(image: &ColorImage) -> (Vec<u8>, u32, u32) {
    let width = image.size[0] as u32;
    let height = image.size[1] as u32;
    let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        // egui stores premultiplied alpha; PNG needs straight (unmultiplied) RGBA.
        rgba.extend_from_slice(&pixel.to_srgba_unmultiplied());
    }
    (rgba, width, height)
}

/// Write the image as a PNG, creating parent directories as needed.
pub fn write_png(path: &Path, image: &ColorImage) -> io::Result<()> {
    let (rgba, width, height) = color_image_to_rgba(image);
    let buffer: image::RgbaImage = image::ImageBuffer::from_raw(width, height, rgba)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "image buffer too short"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    buffer.save(path).map_err(io::Error::other)
}
