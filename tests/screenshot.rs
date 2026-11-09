//! Tests for PNG screenshot output (public API only).

mod common;

use common::{remove, temp_path};
use egui::{Color32, ColorImage};
use ion_of_notes::screenshot;

fn sample() -> ColorImage {
    ColorImage::new(
        [2, 1],
        vec![
            Color32::from_rgba_unmultiplied(1, 2, 3, 255),
            Color32::from_rgba_unmultiplied(250, 120, 30, 255),
        ],
    )
}

#[test]
fn color_image_to_rgba_preserves_pixels_and_size() {
    let (rgba, w, h) = screenshot::color_image_to_rgba(&sample());
    assert_eq!((w, h), (2, 1));
    assert_eq!(rgba, vec![1, 2, 3, 255, 250, 120, 30, 255]);
}

#[test]
fn write_png_produces_readable_png_with_right_dimensions() {
    let path = temp_path("write", "png");
    remove(&path);
    screenshot::write_png(&path, &sample()).unwrap();
    let decoded = image::open(&path).unwrap().into_rgba8();
    assert_eq!(decoded.dimensions(), (2, 1));
    assert_eq!(decoded.get_pixel(0, 0).0, [1, 2, 3, 255]);
    assert_eq!(decoded.get_pixel(1, 0).0, [250, 120, 30, 255]);
    remove(&path);
}
