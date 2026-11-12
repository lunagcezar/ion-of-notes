//! Ranked search over page titles, block text and inline table rows.
//!
//! Search documents are cached per page. Mutating a page marks only that page
//! dirty, so a query after an edit re-indexes the edited page instead of every
//! page in the workspace (spec-109).

use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};

use super::{Page, SearchResult, Workspace};

/// One page's normalized search representation.
#[derive(Debug, Clone)]
pub(crate) struct SearchDocument {
    pub(crate) page_id: String,
    pub(crate) title: String,
    pub(crate) title_lower: String,
    pub(crate) body_text: String,
    pub(crate) body_lower: String,
}

impl SearchDocument {
    fn from_page(page: &Page) -> Self {
        let rows = page
            .rows
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let body_text = if rows.is_empty() {
            page.plain_text()
        } else {
            format!("{} {}", page.plain_text(), rows)
        };
        Self {
            page_id: page.id.clone(),
            title: page.title.clone(),
            title_lower: page.title.to_lowercase(),
            body_lower: body_text.to_lowercase(),
            body_text,
        }
    }
}

#[derive(Debug, Default)]
struct SearchCache {
    documents: HashMap<String, SearchDocument>,
    dirty: HashSet<String>,
    initialized: bool,
}

/// Runtime-only search cache. Cloning a workspace drops the cache so an undo
/// snapshot does not duplicate every indexed body (spec-109).
#[derive(Debug, Default)]
pub(crate) struct SearchCacheCell(RefCell<SearchCache>);

impl Clone for SearchCacheCell {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl SearchCacheCell {
    /// Mark one page for re-indexing.
    pub(crate) fn mark_dirty(&self, page_id: &str) {
        let mut cache = self.0.borrow_mut();
        cache.dirty.insert(page_id.to_string());
    }

    /// Drop a removed page from the cache.
    pub(crate) fn remove(&self, page_id: &str) {
        let mut cache = self.0.borrow_mut();
        cache.documents.remove(page_id);
        cache.dirty.remove(page_id);
    }

    fn documents<'a>(&'a self, pages: &[Page]) -> Ref<'a, HashMap<String, SearchDocument>> {
        {
            let mut cache = self.0.borrow_mut();
            if !cache.initialized {
                cache.documents.clear();
                cache.dirty = pages.iter().map(|page| page.id.clone()).collect();
                cache.initialized = true;
            }
            if !cache.dirty.is_empty() {
                let dirty = std::mem::take(&mut cache.dirty);
                for page_id in dirty {
                    match pages.iter().find(|page| page.id == page_id) {
                        Some(page) => {
                            cache
                                .documents
                                .insert(page_id, SearchDocument::from_page(page));
                        }
                        None => {
                            cache.documents.remove(&page_id);
                        }
                    }
                }
            }
        }
        Ref::map(self.0.borrow(), |cache| &cache.documents)
    }
}

impl Workspace {
    /// Case-insensitive search over titles, block text, and row names.
    /// Ranking: title prefix (3) > title contains (2) > body contains (1).
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let documents = self.search_cache.documents(&self.pages);
        let mut ranked: Vec<(u32, usize, SearchResult)> = Vec::new();
        for (index, page) in self.pages.iter().enumerate() {
            if page.archived {
                continue;
            }
            let Some(document) = documents.get(&page.id) else {
                continue;
            };
            let score = if document.title_lower.starts_with(&q) {
                3
            } else if document.title_lower.contains(&q) {
                2
            } else if document.body_lower.contains(&q) {
                1
            } else {
                0
            };
            if score == 0 {
                continue;
            }
            let snippet = if score >= 2 {
                document.title.clone()
            } else {
                excerpt(&document.body_text, &q)
            };
            ranked.push((
                score,
                index,
                SearchResult {
                    page_id: document.page_id.clone(),
                    page_title: document.title.clone(),
                    snippet,
                    score,
                },
            ));
        }
        ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        ranked.into_iter().map(|(_, _, r)| r).collect()
    }
}

/// A short case-insensitive excerpt around the first match.
fn excerpt(text: &str, query: &str) -> String {
    let lower = text.to_lowercase();
    let start = lower.find(query).unwrap_or(0);
    let clipped: String = text.chars().skip(start).take(90).collect();
    if clipped.chars().count() == 90 {
        format!("{clipped}...")
    } else {
        clipped
    }
}
