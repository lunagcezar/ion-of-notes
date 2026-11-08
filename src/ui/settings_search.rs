//! Pure settings search matching (no egui, no state).
//!
//! The settings window keeps its navigation and rows in one place; this helper
//! decides whether a section title or description matches the query. Keeping
//! it pure makes the behavior testable without opening a window (spec-015).

/// Return true when the query is empty or appears in the title/description.
///
/// Matching is case-insensitive and trims the query, so typing spaces does not
/// hide every section.
pub fn settings_matches(query: &str, title: &str, description: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    title.to_lowercase().contains(&query) || description.to_lowercase().contains(&query)
}
