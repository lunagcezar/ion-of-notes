//! A tiny cache key for search results (no egui, no I/O).
//!
//! The quick-search window used to call the server on every frame. The cache
//! remembers the last query and workspace version so an unchanged frame is a
//! cache hit. It is pure and tested in `tests/ui.rs` (spec-015).

/// The last search query and workspace version that produced the results.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchCache {
    key: Option<(String, u64)>,
}

impl SearchCache {
    /// True when the results must be fetched again.
    ///
    /// The first call always refreshes. After that, the query or the workspace
    /// version must change. No allocation happens on a cache hit.
    pub fn needs_refresh(&mut self, query: &str, version: u64) -> bool {
        if let Some((cached_query, cached_version)) = &self.key
            && *cached_version == version
            && cached_query == query
        {
            return false;
        }
        self.key = Some((query.to_string(), version));
        true
    }
}
