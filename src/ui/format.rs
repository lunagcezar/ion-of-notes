//! Pure number and timestamp formatting for the settings choices (spec-061).
//!
//! These helpers live in the presentation layer because they only turn stored
//! values into display strings; the domain keeps the raw workspace data.

use chrono::NaiveDateTime;

use crate::model::{NumberFormat, TimeFormat};

/// The separators used by a number format. `Automatic` follows the active
/// locale in one place instead of scattering locale checks through the UI.
fn separators(format: NumberFormat) -> (char, char) {
    match format {
        NumberFormat::Automatic => {
            let locale = crate::i18n::current_locale();
            if locale.to_lowercase().starts_with("pt") {
                ('.', ',')
            } else {
                (',', '.')
            }
        }
        NumberFormat::DotThousands => ('.', ','),
        NumberFormat::CommaThousands => (',', '.'),
    }
}

/// Format a number with grouped thousands and the chosen decimal separator.
/// Trailing fractional zeros are removed, so whole values stay whole.
pub fn format_number(value: f64, format: NumberFormat) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let (thousands, decimal) = separators(format);
    let negative = value.is_sign_negative() && value != 0.0;
    let mut raw = format!("{:.2}", value.abs());
    if let Some(dot) = raw.find('.') {
        while raw.ends_with('0') {
            raw.pop();
        }
        if raw.len() == dot + 1 {
            raw.pop();
        }
    }
    let (integer, fraction) = match raw.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (raw.as_str(), None),
    };
    let mut grouped = String::new();
    for (index, ch) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(thousands);
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();
    let mut result = match fraction {
        Some(fraction) => format!("{grouped}{decimal}{fraction}"),
        None => grouped,
    };
    if negative {
        result.insert(0, '-');
    }
    result
}

/// Format a stored timestamp with the chosen clock. Both the legacy
/// `Nov 09, 2026 03:00` shape and ISO-8601 are accepted; anything else is
/// returned unchanged.
pub fn format_timestamp(value: &str, format: TimeFormat) -> String {
    let trimmed = value.trim();
    let parsed = NaiveDateTime::parse_from_str(trimmed, "%b %d, %Y %H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S"));
    let Ok(timestamp) = parsed else {
        return value.to_string();
    };
    match format {
        TimeFormat::Hour24 => timestamp.format("%b %d, %Y %H:%M").to_string(),
        TimeFormat::Hour12 => timestamp.format("%b %d, %Y %-I:%M %p").to_string(),
    }
}
