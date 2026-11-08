//! Visual constants, typography and theme application (SRP: all colors, fonts
//! and spacing live in one place, NFR-06).

use crate::model::BlockKind;
use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// True once a strong display face has been registered with egui.
static STRONG_FONT: AtomicBool = AtomicBool::new(false);
static FONTS_INSTALLED: AtomicBool = AtomicBool::new(false);

/// All colors and visual constants live here (NFR-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// The custom window chrome (tab strip) above the sidebar and page.
    pub chrome: Color32,
    pub sidebar: Color32,
    pub content: Color32,
    pub topbar: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub hover: Color32,
    pub selected: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub callout: Color32,
    pub code_bg: Color32,
    pub icon_bg: Color32,
    pub chip_text: Color32,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            chrome: Color32::from_rgb(247, 247, 245),
            sidebar: Color32::from_rgb(247, 247, 245),
            content: Color32::from_rgb(255, 255, 255),
            topbar: Color32::from_rgb(255, 255, 255),
            text: Color32::from_rgb(55, 53, 47),
            muted: Color32::from_rgb(155, 154, 151),
            hover: Color32::from_rgb(239, 239, 237),
            selected: Color32::from_rgb(228, 228, 225),
            border: Color32::from_rgb(233, 233, 231),
            accent: Color32::from_rgb(35, 131, 226),
            callout: Color32::from_rgb(241, 241, 239),
            code_bg: Color32::from_rgb(246, 246, 244),
            icon_bg: Color32::from_rgb(238, 238, 236),
            chip_text: Color32::from_rgb(55, 53, 47),
        }
    }

    pub fn dark() -> Self {
        Self {
            // The tab strip is a touch lighter than the near-black page.
            chrome: Color32::from_rgb(28, 28, 28),
            sidebar: Color32::from_rgb(32, 32, 32),
            content: Color32::from_rgb(25, 25, 25),
            topbar: Color32::from_rgb(28, 28, 28),
            text: Color32::from_rgb(233, 233, 231),
            muted: Color32::from_rgb(138, 138, 136),
            hover: Color32::from_rgb(48, 48, 48),
            selected: Color32::from_rgb(58, 58, 58),
            border: Color32::from_rgb(58, 58, 58),
            accent: Color32::from_rgb(42, 128, 225),
            callout: Color32::from_rgb(42, 42, 42),
            code_bg: Color32::from_rgb(36, 36, 36),
            icon_bg: Color32::from_rgb(48, 48, 48),
            chip_text: Color32::from_rgb(230, 230, 228),
        }
    }

    pub fn for_theme(dark: bool) -> Self {
        if dark { Self::dark() } else { Self::light() }
    }
}

/// Stable chip colors for the three known statuses plus a neutral fallback.
pub fn status_color(status: &str) -> Color32 {
    match status {
        "Done" => Color32::from_rgb(219, 237, 219),
        "In progress" => Color32::from_rgb(253, 236, 200),
        "Not started" => Color32::from_rgb(227, 226, 224),
        _ => Color32::from_rgb(211, 229, 239),
    }
}

/// Slash-command options. The divider is intentionally excluded (it has no text).
pub fn block_menu_options() -> Vec<(BlockKind, &'static str)> {
    BlockKind::all()
        .iter()
        .filter(|kind| !kind.is_divider())
        .map(|kind| (*kind, kind.label()))
        .collect()
}

/// Shorten a page title so it fits one tab in the top strip.
pub fn tab_title(title: &str) -> String {
    const MAX_CHARS: usize = 22;
    let chars: Vec<char> = title.chars().collect();
    if chars.len() <= MAX_CHARS {
        return title.to_string();
    }
    let head: String = chars[..MAX_CHARS].iter().collect();
    format!("{head}…")
}

/// Register a bold system face for titles and headings. Called once; the rest
/// of the app keeps the (regular) proportional family for body copy.
///
/// The original uses a clean, fairly heavy UI sans on Windows, so Segoe UI is
/// used when present. Nothing is read from the reconstructed program; these are
/// operating-system font files. If they are absent, egui's built-in fonts are
/// used and `strong_font` falls back to the proportional family.
pub fn install_fonts(ctx: &egui::Context) {
    if FONTS_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let regular = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf").ok();
    let bold = std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf").ok();
    // Segoe UI Symbol / Emoji supply glyphs for page icons (spec-013/014).
    let symbol = std::fs::read(r"C:\Windows\Fonts\seguisym.ttf").ok();
    let emoji = std::fs::read(r"C:\Windows\Fonts\seguiemj.ttf").ok();
    let serif = std::fs::read(r"C:\Windows\Fonts\times.ttf").ok();
    if regular.is_none() && bold.is_none() && symbol.is_none() && emoji.is_none() && serif.is_none()
    {
        return;
    }

    let mut defs = FontDefinitions::default();
    if let Some(bytes) = regular {
        defs.font_data.insert(
            "ui-regular".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        defs.families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "ui-regular".to_owned());
    }
    if let Some(bytes) = bold {
        let fallback = defs
            .families
            .get(&FontFamily::Proportional)
            .and_then(|v| v.first())
            .cloned();
        defs.font_data.insert(
            "ui-strong".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        let strong = defs
            .families
            .entry(FontFamily::Name("Strong".into()))
            .or_default();
        strong.insert(0, "ui-strong".to_owned());
        if let Some(name) = fallback {
            strong.push(name);
        }
        STRONG_FONT.store(true, Ordering::SeqCst);
    }
    if let Some(bytes) = emoji {
        defs.font_data
            .insert("ui-emoji".to_owned(), Arc::new(FontData::from_owned(bytes)));
        defs.families
            .entry(FontFamily::Proportional)
            .or_default()
            .push("ui-emoji".to_owned());
    }
    if let Some(bytes) = symbol {
        defs.font_data.insert(
            "ui-symbol".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );
        defs.families
            .entry(FontFamily::Proportional)
            .or_default()
            .push("ui-symbol".to_owned());
    }
    if let Some(bytes) = serif {
        defs.font_data
            .insert("ui-serif".to_owned(), Arc::new(FontData::from_owned(bytes)));
        // Fall back to the installed UI face for glyphs Times lacks.
        let fallback = defs
            .families
            .get(&FontFamily::Proportional)
            .and_then(|v| v.first())
            .cloned();
        let serif_family = defs
            .families
            .entry(FontFamily::Name("Serif".into()))
            .or_default();
        serif_family.push("ui-serif".to_owned());
        if let Some(name) = fallback {
            serif_family.push(name);
        }
    }
    ctx.set_fonts(defs);
}

/// A bold font id for page titles and section headings.
pub fn strong_font(size: f32) -> FontId {
    if STRONG_FONT.load(Ordering::SeqCst) {
        FontId::new(size, FontFamily::Name("Strong".into()))
    } else {
        FontId::proportional(size)
    }
}

pub(super) fn apply_theme(ctx: &egui::Context, dark: bool) {
    install_fonts(ctx);
    let palette = Palette::for_theme(dark);
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = palette.content;
    visuals.window_fill = palette.content;
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.muted);
    visuals.faint_bg_color = palette.hover;
    visuals.code_bg_color = palette.code_bg;
    visuals.text_edit_bg_color = Some(palette.content);
    visuals.widgets.hovered.weak_bg_fill = palette.hover;
    visuals.widgets.active.weak_bg_fill = palette.selected;
    visuals.widgets.noninteractive.bg_stroke.color = palette.border;
    ctx.set_visuals_of(theme, visuals);
}
