//! Pages, summaries, drafts and workspace invariant errors.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::block::{Block, BlockKind, count_blocks};
use super::comment::Comment;
use super::database::TableRow;
use super::now_label;
use super::property::PageProperty;
/// A page in the workspace tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub parent: Option<String>,
    pub favorite: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub available_offline: bool,
    #[serde(default)]
    pub show_toc: bool,
    #[serde(default)]
    pub discussions: bool,
    #[serde(default)]
    pub inline_comments: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub comments: Vec<Comment>,
    pub blocks: Vec<Block>,
    pub rows: Vec<TableRow>,
    #[serde(default)]
    pub data_source_id: Option<String>,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, PageProperty>,
    pub updated_at: String,
}

impl Page {
    pub fn new(id: impl Into<String>, title: impl Into<String>, icon: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            blocks: vec![Block::new(format!("{id}-b0"), BlockKind::Paragraph, "")],
            id,
            title: title.into(),
            icon: icon.into(),
            parent: None,
            favorite: false,
            locked: false,
            available_offline: false,
            show_toc: false,
            discussions: false,
            inline_comments: false,
            archived: false,
            comments: Vec::new(),
            rows: Vec::new(),
            data_source_id: None,
            properties: std::collections::BTreeMap::new(),
            updated_at: now_label(),
        }
    }

    /// Lifetime block usage of this page: the page itself counts as one block,
    /// plus every nested block (spec-037).
    pub fn block_count(&self) -> u64 {
        1 + count_blocks(&self.blocks)
    }

    /// Non-empty headings, in page order (used by the table of contents).
    pub fn headings(&self) -> Vec<&Block> {
        self.blocks
            .iter()
            .filter(|block| block.kind.is_heading() && !block.plain_text().trim().is_empty())
            .collect()
    }

    /// All block text joined into one searchable string.
    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| !b.kind.is_divider())
            .map(Block::plain_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Normalize legacy `text` blocks into rich text and recurse into children.
    pub fn normalize_blocks(&mut self) {
        for block in &mut self.blocks {
            block.normalize();
        }
    }

    pub fn summary(&self) -> PageSummary {
        PageSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            icon: self.icon.clone(),
            parent: self.parent.clone(),
            favorite: self.favorite,
        }
    }
}

/// A lightweight page description for lists and sidebars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSummary {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub parent: Option<String>,
    pub favorite: bool,
}

/// A ranked search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub page_id: String,
    pub page_title: String,
    pub snippet: String,
    pub score: u32,
}

/// The create-page request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageDraft {
    pub title: String,
    pub icon: String,
    pub parent: Option<String>,
}

/// Errors raised by workspace invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    DuplicateId(String),
    NotFound(String),
    InvalidParent(String),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceError::DuplicateId(id) => write!(f, "duplicate page id: {id}"),
            WorkspaceError::NotFound(id) => write!(f, "page not found: {id}"),
            WorkspaceError::InvalidParent(id) => write!(f, "unknown parent page: {id}"),
        }
    }
}

impl std::error::Error for WorkspaceError {}
