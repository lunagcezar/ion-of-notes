//! Remote cursor and selection overlay.

use crate::server::realtime::Awareness;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};

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

pub(super) fn draw_remote_cursors(
    ui: &mut egui::Ui,
    rect: &Rect,
    text: &str,
    font: &FontId,
    cursors: &[&Awareness],
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

const CURSOR_WIDTH_CACHE_CAP: usize = 1024;

type WidthCache = Arc<Mutex<HashMap<CursorWidthKey, f32>>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CursorWidthKey {
    text_hash: u64,
    chars: usize,
    font_size: u32,
    family: u64,
}

fn text_width(ui: &egui::Ui, text: &str, chars: usize, font: &FontId, color: Color32) -> f32 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    let mut family = DefaultHasher::new();
    std::mem::discriminant(&font.family).hash(&mut family);
    let key = CursorWidthKey {
        text_hash: hasher.finish(),
        chars,
        font_size: font.size.to_bits(),
        family: family.finish(),
    };
    let id = egui::Id::unique("cursor_width_cache");
    let cache = ui
        .ctx()
        .data(|data| data.get_temp::<WidthCache>(id))
        .unwrap_or_else(|| {
            let cache: WidthCache = Arc::new(Mutex::new(HashMap::new()));
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, Arc::clone(&cache)));
            cache
        });
    if let Ok(map) = cache.lock()
        && let Some(width) = map.get(&key)
    {
        return *width;
    }
    let prefix: String = text.chars().take(chars).collect();
    let width = ui
        .painter()
        .layout_no_wrap(prefix, font.clone(), color)
        .size()
        .x;
    if let Ok(mut map) = cache.lock() {
        if map.len() >= CURSOR_WIDTH_CACHE_CAP {
            map.clear();
        }
        map.insert(key, width);
    }
    width
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
