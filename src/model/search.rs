//! Ranked search over page titles, block text and inline table rows.

use super::{SearchResult, Workspace};

impl Workspace {
    /// Case-insensitive search over titles, block text, and row names.
    /// Ranking: title prefix (3) > title contains (2) > body contains (1).
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut ranked: Vec<(u32, usize, SearchResult)> = Vec::new();
        for (index, page) in self.pages.iter().enumerate() {
            if page.archived {
                continue;
            }
            let title = page.title.to_lowercase();
            let rows = page
                .rows
                .iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let body = format!("{} {}", page.plain_text(), rows).to_lowercase();
            let score = if title.starts_with(&q) {
                3
            } else if title.contains(&q) {
                2
            } else if body.contains(&q) {
                1
            } else {
                0
            };
            if score == 0 {
                continue;
            }
            let snippet = if score >= 2 {
                page.title.clone()
            } else {
                excerpt(&format!("{} {}", page.plain_text(), rows), &q)
            };
            ranked.push((
                score,
                index,
                SearchResult {
                    page_id: page.id.clone(),
                    page_title: page.title.clone(),
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
