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

/// Translate one key in the current locale.
///
/// Keeping this helper in one place means the UI can pass dynamic label keys
/// (for example a settings tab name) without repeating the macro at every call
/// site (spec-028).
pub fn t(key: &str) -> String {
    let locale = rust_i18n::locale().to_string();
    let key = format!("{locale}.{key}");
    crate::_rust_i18n_translate(&locale, &key).to_string()
}

/// Translate a key that contains a `%{count}` placeholder.
pub fn t_count(key: &str, count: usize) -> String {
    t(key).replace("%{count}", &count.to_string())
}
