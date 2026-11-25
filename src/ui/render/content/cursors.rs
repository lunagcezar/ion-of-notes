//! Remote cursor and selection overlay.
//!
//! Positions come from the laid-out `Galley`, so a caret or selection that
//! wraps onto later rows lands on the correct row and column instead of being
//! approximated from a first-line text width.

use crate::server::realtime::Awareness;
use egui::text::CCursor;
use egui::{Align2, Color32, CornerRadius, FontId, Galley, Pos2, Rect, Stroke};
use std::collections::HashMap;

use super::super::super::palette::Palette;

/// Group the awareness records for one page by block id once per frame.
///
/// Rendering then does a hash lookup per block instead of scanning every
/// remote cursor for every block (spec-103).
pub fn index_cursors<'a>(
    cursors: &'a [Awareness],
    page_id: &str,
) -> HashMap<&'a str, Vec<&'a Awareness>> {
    let mut by_block: HashMap<&'a str, Vec<&'a Awareness>> = HashMap::new();
    for cursor in cursors.iter().filter(|cursor| cursor.page_id == page_id) {
        by_block
            .entry(cursor.block_id.as_str())
            .or_default()
            .push(cursor);
    }
    by_block
}

/// Selection rectangles in galley-local coordinates, one per visual row.
pub fn selection_rects(galley: &Galley, start: usize, end: usize) -> Vec<Rect> {
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    if start == end {
        return Vec::new();
    }
    let start_cursor = galley.layout_from_cursor(CCursor::new(start));
    let end_cursor = galley.layout_from_cursor(CCursor::new(end));
    let mut rects = Vec::new();
    for row_index in start_cursor.row..=end_cursor.row {
        let Some(row) = galley.rows.get(row_index) else {
            continue;
        };
        let left = if row_index == start_cursor.row {
            row.pos.x + row.x_offset(start_cursor.column)
        } else {
            row.rect().left()
        };
        let right = if row_index == end_cursor.row {
            row.pos.x + row.x_offset(end_cursor.column)
        } else {
            row.rect().right()
        };
        if right > left {
            rects.push(Rect::from_min_max(
                Pos2::new(left, row.rect().top()),
                Pos2::new(right, row.rect().bottom()),
            ));
        }
    }
    rects
}

pub(super) fn draw_remote_cursors(
    ui: &mut egui::Ui,
    galley: &Galley,
    galley_pos: Pos2,
    cursors: &[&Awareness],
    palette: &Palette,
) {
    let painter = ui.painter();
    for cursor in cursors {
        let color = parse_hex_color(&cursor.color).unwrap_or(palette.accent);
        if let Some(end) = cursor.selection_end {
            for rect in selection_rects(galley, cursor.index, end) {
                painter.rect_filled(
                    rect.translate(galley_pos.to_vec2()),
                    CornerRadius::same(2),
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40),
                );
            }
        }
        let caret = galley.pos_from_cursor(CCursor::new(cursor.index));
        let x = galley_pos.x + caret.center().x;
        let top = galley_pos.y + caret.top();
        let bottom = galley_pos.y + caret.bottom();
        painter.vline(x, top..=bottom, Stroke::new(2.0, color));
        painter.text(
            Pos2::new(x + 3.0, top - 2.0),
            Align2::LEFT_BOTTOM,
            &cursor.name,
            FontId::proportional(10.0),
            color,
        );
    }
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
