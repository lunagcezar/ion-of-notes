//! Pure Library view filtering (spec-024).
//!
//! The Library tabs are presentation views over the same page list. Keeping
//! the filtering pure makes the behavior testable without egui.

use crate::model::Page;
use std::collections::HashSet;

/// Pages shown for a Library tab.
///
/// `shared` contains page ids that have a page-specific share (the general
/// `"*"` rule is deliberately ignored here). Unknown tabs are placeholder
/// views with no pages yet.
pub fn pages_for_view<'a>(pages: &'a [Page], tab: &str, shared: &HashSet<String>) -> Vec<&'a Page> {
    match tab {
        "Favorites" => pages.iter().filter(|page| page.favorite).collect(),
        "Shared" => pages
            .iter()
            .filter(|page| shared.contains(&page.id))
            .collect(),
        "Private" => pages
            .iter()
            .filter(|page| !shared.contains(&page.id))
            .collect(),
        "Recents" | "" => pages.iter().collect(),
        _ => Vec::new(),
    }
}
