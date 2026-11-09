//! Locale completeness and translation lookup tests (spec-028).

use ion_of_notes::i18n;
use std::collections::HashSet;
use std::path::Path;

fn keys(path: &str) -> HashSet<String> {
    let text = std::fs::read_to_string(Path::new(path)).unwrap();
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || !line.contains(':') {
                return None;
            }
            Some(line.split_once(':').unwrap().0.trim().to_string())
        })
        .filter(|key| !matches!(key.as_str(), "en" | "pt-BR" | "eo"))
        .collect()
}

#[test]
fn every_english_key_exists_in_all_locales() {
    let en = keys("locales/en.yml");
    let pt = keys("locales/pt-BR.yml");
    let eo = keys("locales/eo.yml");

    let missing_pt: Vec<_> = en.difference(&pt).cloned().collect();
    let missing_eo: Vec<_> = en.difference(&eo).cloned().collect();
    assert!(missing_pt.is_empty(), "missing pt-BR keys: {missing_pt:?}");
    assert!(missing_eo.is_empty(), "missing eo keys: {missing_eo:?}");
}

#[test]
fn translations_switch_with_the_locale() {
    i18n::set_locale("en");
    assert_eq!(i18n::t("library"), "Library");
    assert_eq!(i18n::current_locale(), "en");

    i18n::set_locale("pt-BR");
    assert_eq!(i18n::t("library"), "Biblioteca");
    assert_eq!(i18n::label_for("pt-BR"), "Português (Brasil)");

    i18n::set_locale("eo");
    assert_eq!(i18n::t("library"), "Biblioteko");

    i18n::set_locale("unknown");
    assert_eq!(i18n::current_locale(), "en");
    assert_eq!(i18n::t("library"), "Library");
}
