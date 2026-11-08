//! Internationalization helpers (English, Portuguese (Brazil), Esperanto).
//!
//! Uses `rust-i18n` locale files under `app/locales/`. The active locale is
//! set globally for the UI; persistence of the preference is handled by the
//! settings layer.

pub const SUPPORTED: &[(&str, &str)] = &[
    ("en", "English (US)"),
    ("pt-BR", "Português (Brasil)"),
    ("eo", "Esperanto"),
];

pub fn set_locale(locale: &str) {
    let locale = if SUPPORTED.iter().any(|(code, _)| *code == locale) {
        locale
    } else {
        "en"
    };
    rust_i18n::set_locale(locale);
}

pub fn label_for(locale: &str) -> &'static str {
    SUPPORTED
        .iter()
        .find(|(code, _)| *code == locale)
        .map(|(_, label)| *label)
        .unwrap_or("English (US)")
}

pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}