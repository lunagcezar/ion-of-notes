//! Remote cursor and selection overlay.

use crate::server::realtime::Awareness;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke};

use super::super::super::palette::Palette;
pub(super) fn draw_remote_cursors(
    ui: &mut egui::Ui,
    rect: &Rect,
    text: &str,
    font: &FontId,
    cursors: &[Awareness],
    palette: &Palette,
) {
    for cursor in cursors {
        let color = parse_hex_color(&cursor.color).unwrap_or(palette.accent);
        if let Some(end) = cursor.selection_end {
            let start = cursor.index.min(end);
            let end = cursor.index.max(end);
            let start_x = rect.min.x + text_width(ui, text, start, font, color);
            let end_x = rect.min.x + text_width(ui, text, end, font, color);
            ui.painter().rect_filled(
                Rect::from_min_max(Pos2::new(start_x, rect.min.y), Pos2::new(end_x, rect.max.y)),
                CornerRadius::same(2),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
            );
        }
        let x = (rect.min.x + text_width(ui, text, cursor.index, font, color))
            .clamp(rect.min.x, rect.max.x - 2.0);
        ui.painter()
            .vline(x, rect.y_range(), Stroke::new(2.0, color));
        ui.painter().text(
            Pos2::new(x + 3.0, rect.min.y - 2.0),
            Align2::LEFT_BOTTOM,
            &cursor.name,
            FontId::proportional(10.0),
            color,
        );
    }
}

fn text_width(ui: &egui::Ui, text: &str, chars: usize, font: &FontId, color: Color32) -> f32 {
    let prefix: String = text.chars().take(chars).collect();
    ui.painter()
        .layout_no_wrap(prefix, font.clone(), color)
        .size()
        .x
}

fn parse_hex_color(value: &str) -> Option<Color32> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color32::from_rgb(red, green, blue))
}
