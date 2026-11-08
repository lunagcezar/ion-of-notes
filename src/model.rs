//! Pure workspace data model. No I/O, no UI types.
//!
//! `Workspace` owns an ordered set of `Page`s. Pages form a tree through
//! `parent`, hold editable `Block`s, and may carry inline `TableRow`s for a
//! simple database view.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

mod diff;
mod index;
mod search;
mod seed;
mod settings;

pub use diff::PageDiff;
use index::WorkspaceIndex;
pub use settings::WorkspaceSettings;

/// A short, human-friendly timestamp used for display only.
pub fn now_label() -> String {
    Utc::now().format("%b %d, %Y %H:%M").to_string()
}

/// The rendering and behavior variant of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    Bulleted,
    Numbered,
    Todo,
    Quote,
    Callout,
    Code,
    Divider,
}

impl BlockKind {
    /// All variants, in the order shown in the slash-command menu.
    pub fn all() -> &'static [BlockKind] {
        &[
            BlockKind::Paragraph,
            BlockKind::Heading1,
            BlockKind::Heading2,
            BlockKind::Heading3,
            BlockKind::Bulleted,
            BlockKind::Numbered,
            BlockKind::Todo,
            BlockKind::Quote,
            BlockKind::Callout,
            BlockKind::Code,
            BlockKind::Divider,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            BlockKind::Paragraph => "Text",
            BlockKind::Heading1 => "Heading 1",
            BlockKind::Heading2 => "Heading 2",
            BlockKind::Heading3 => "Heading 3",
            BlockKind::Bulleted => "Bulleted list",
            BlockKind::Numbered => "Numbered list",
            BlockKind::Todo => "To-do list",
            BlockKind::Quote => "Quote",
            BlockKind::Callout => "Callout",
            BlockKind::Code => "Code",
            BlockKind::Divider => "Divider",
        }
    }

    pub fn is_divider(&self) -> bool {
        matches!(self, BlockKind::Divider)
    }

    pub fn is_heading(&self) -> bool {
        matches!(
            self,
            BlockKind::Heading1 | BlockKind::Heading2 | BlockKind::Heading3
        )
    }

    pub fn font_size(&self) -> f32 {
        match self {
            BlockKind::Heading1 => 30.0,
            BlockKind::Heading2 => 24.0,
            BlockKind::Heading3 => 19.0,
            _ => 15.0,
        }
    }
}

/// One editable content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub kind: BlockKind,
    pub text: String,
    pub checked: bool,
    pub collapsed: bool,
}

impl Block {
    pub fn new(id: impl Into<String>, kind: BlockKind, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            text: text.into(),
            checked: false,
            collapsed: false,
        }
    }

    pub fn set_kind(&mut self, kind: BlockKind) {
        self.kind = kind;
    }

    /// Flip the completed state of a to-do block, returning the new value.
    pub fn toggle_todo(&mut self) -> bool {
        self.checked = !self.checked;
        self.checked
    }
}

/// One row of an inline table database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub assignee: String,
    pub due: String,
}

impl TableRow {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        status: impl Into<String>,
        assignee: impl Into<String>,
        due: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: status.into(),
            assignee: assignee.into(),
            due: due.into(),
        }
    }
}

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
    pub blocks: Vec<Block>,
    pub rows: Vec<TableRow>,
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
            rows: Vec::new(),
            updated_at: now_label(),
        }
    }

    /// All block text joined into one searchable string.
    pub fn plain_text(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| !b.kind.is_divider() && !b.text.is_empty())
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
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

/// The whole workspace: one JSON document on disk.
///
/// The private index is derived state: it is skipped by serde, rebuilt after
/// deserialization and after add/remove, and ignored by equality.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Workspace {
    pub name: String,
    pub pages: Vec<Page>,
    pub version: u64,
    #[serde(default)]
    pub settings: WorkspaceSettings,
    #[serde(skip)]
    index: WorkspaceIndex,
}

impl PartialEq for Workspace {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.pages == other.pages
            && self.version == other.version
            && self.settings == other.settings
    }
}

impl Eq for Workspace {}

#[derive(Deserialize)]
struct WorkspaceData {
    name: String,
    pages: Vec<Page>,
    version: u64,
    #[serde(default)]
    settings: WorkspaceSettings,
}

impl<'de> Deserialize<'de> for Workspace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = WorkspaceData::deserialize(deserializer)?;
        Ok(Self {
            index: WorkspaceIndex::build(&data.pages),
            name: data.name,
            pages: data.pages,
            version: data.version,
            settings: data.settings,
        })
    }
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pages: Vec::new(),
            version: 0,
            settings: WorkspaceSettings::default(),
            index: WorkspaceIndex::default(),
        }
    }

    pub fn page(&self, id: &str) -> Option<&Page> {
        let position = if self.index.len() == self.pages.len() {
            self.index.position(id)
        } else {
            None
        };
        if let Some(position) = position
            && let Some(page) = self.pages.get(position)
            && page.id == id
        {
            return Some(page);
        }
        self.pages.iter().find(|p| p.id == id)
    }

    pub fn page_mut(&mut self, id: &str) -> Option<&mut Page> {
        let position = if self.index.len() == self.pages.len() {
            self.index.position(id)
        } else {
            None
        };
        if let Some(position) = position
            && self.pages.get(position).is_some_and(|page| page.id == id)
        {
            return self.pages.get_mut(position);
        }
        self.pages.iter_mut().find(|p| p.id == id)
    }

    pub fn add_page(&mut self, page: Page) -> Result<(), WorkspaceError> {
        if self.pages.iter().any(|p| p.id == page.id) {
            return Err(WorkspaceError::DuplicateId(page.id));
        }
        if let Some(parent) = page.parent.as_deref()
            && self.page(parent).is_none()
        {
            return Err(WorkspaceError::InvalidParent(parent.to_string()));
        }
        self.pages.push(page);
        self.index = WorkspaceIndex::build(&self.pages);
        Ok(())
    }

    /// Remove a page and every descendant, returning all removed ids.
    pub fn remove_page(&mut self, id: &str) -> Result<Vec<String>, WorkspaceError> {
        if self.page(id).is_none() {
            return Err(WorkspaceError::NotFound(id.to_string()));
        }
        let mut doomed: HashSet<String> = HashSet::new();
        doomed.insert(id.to_string());
        loop {
            let before = doomed.len();
            for page in &self.pages {
                if let Some(parent) = page.parent.as_deref()
                    && doomed.contains(parent)
                {
                    doomed.insert(page.id.clone());
                }
            }
            if doomed.len() == before {
                break;
            }
        }
        let removed: Vec<String> = self
            .pages
            .iter()
            .filter(|p| doomed.contains(&p.id))
            .map(|p| p.id.clone())
            .collect();
        self.pages.retain(|p| !doomed.contains(&p.id));
        self.index = WorkspaceIndex::build(&self.pages);
        Ok(removed)
    }

    pub fn children(&self, parent: Option<&str>) -> Vec<&Page> {
        if self.index.len() == self.pages.len() {
            let ids = self.index.children(parent);
            let mut children = Vec::with_capacity(ids.len());
            for id in ids {
                if let Some(page) = self.page(id) {
                    children.push(page);
                }
            }
            if children.len() == ids.len() {
                return children;
            }
        }
        self.pages
            .iter()
            .filter(|p| p.parent.as_deref() == parent)
            .collect()
    }

    /// Titles from the root page down to (and including) the given page.
    pub fn breadcrumbs(&self, id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut cursor = Some(id.to_string());
        while let Some(current) = cursor {
            if !visited.insert(current.clone()) {
                break;
            }
            match self.page(&current) {
                Some(page) => {
                    chain.push(page.title.clone());
                    cursor = page.parent.clone();
                }
                None => break,
            }
        }
        chain.reverse();
        chain
    }

    pub fn favorites(&self) -> Vec<&Page> {
        self.pages.iter().filter(|p| p.favorite).collect()
    }

    pub fn bump_version(&mut self) {
        self.version = self.version.saturating_add(1);
    }
}
