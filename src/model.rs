//! Pure workspace data model. No I/O, no UI types.
//!
//! `Workspace` owns an ordered set of `Page`s. Pages form a tree through
//! `parent`, hold editable `Block`s, and may carry inline `TableRow`s for a
//! simple database view.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub name: String,
    pub pages: Vec<Page>,
    pub version: u64,
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            pages: Vec::new(),
            version: 0,
        }
    }

    /// A small, neutral demo workspace used on first start.
    pub fn seed(name: &str) -> Self {
        fn make(id: &str, title: &str, icon: &str, parent: Option<&str>) -> Page {
            let mut p = Page::new(id, title, icon);
            p.parent = parent.map(str::to_string);
            p
        }

        let mut ws = Workspace::new(name);

        // Neutral demo content shaped like the pictured document page: a
        // callout, bold section headings, body paragraphs and a short list.
        // An empty icon is the default open-book artwork. The other pages use
        // customizable glyphs so the choice is also visible in the sidebar,
        // breadcrumbs and tabs (spec-013).
        let mut home = make("p-home", "Home", "", None);
        home.favorite = true;
        home.blocks = vec![
            Block::new(
                "h1",
                BlockKind::Callout,
                "A shared workspace for notes, projects and tasks. Sign in to your self-hosted server: changes sync across devices, with sharing and live editing for the whole team.",
            ),
            Block::new("h2", BlockKind::Heading1, "Overview"),
            Block::new(
                "h3",
                BlockKind::Paragraph,
                "Pages are grouped into a simple tree. Use the sidebar to move between them, and click any line to edit it in place.",
            ),
            Block::new(
                "h4",
                BlockKind::Paragraph,
                "Press Ctrl+K for quick search, or type / inside a block to change its type. Changes are saved automatically as you work.",
            ),
            Block::new("h5", BlockKind::Heading1, "Documentation"),
            Block::new("h6", BlockKind::Bulleted, "01 · About this project"),
            Block::new("h7", BlockKind::Bulleted, "02 · Search and references"),
        ];
        ws.add_page(home).expect("seed: home");

        let mut start = make("p-start", "Getting Started", "✦", None);
        start.blocks = vec![
            Block::new("s1", BlockKind::Heading1, "Getting Started"),
            Block::new(
                "s2",
                BlockKind::Paragraph,
                "This is a short guide to the workspace. Edit any line by clicking on it.",
            ),
            Block::new("s3", BlockKind::Heading2, "Things to try"),
            Block::new("s4", BlockKind::Todo, "Rename this page"),
            Block::new("s5", BlockKind::Todo, "Create a child page"),
            Block::new("s6", BlockKind::Todo, "Open the Tasks table"),
            Block::new(
                "s7",
                BlockKind::Quote,
                "Small steps, written down, add up.",
            ),
        ];
        ws.add_page(start).expect("seed: getting started");

        let mut projects = make("p-projects", "Projects", "◆", None);
        projects.blocks = vec![
            Block::new("p1", BlockKind::Heading1, "Projects"),
            Block::new(
                "p2",
                BlockKind::Paragraph,
                "Group related pages together. A page can be nested under another page.",
            ),
            Block::new("p3", BlockKind::Heading3, "Active work"),
            Block::new("p4", BlockKind::Bulleted, "Tasks"),
            Block::new("p5", BlockKind::Bulleted, "Notes"),
        ];
        ws.add_page(projects).expect("seed: projects");

        let mut tasks = make("p-tasks", "Tasks", "☑", Some("p-projects"));
        tasks.favorite = true;
        tasks.blocks = vec![
            Block::new("t1", BlockKind::Heading1, "Tasks"),
            Block::new(
                "t2",
                BlockKind::Paragraph,
                "A simple table to track work. Switch between the Table and Board views.",
            ),
        ];
        tasks.rows = vec![
            TableRow::new("r1", "Draft the project brief", "In progress", "Ava", "Nov 06"),
            TableRow::new("r2", "Review the design spec", "Done", "Luna", "Nov 04"),
            TableRow::new("r3", "Set up the data model", "Not started", "Ava", "Nov 10"),
            TableRow::new("r4", "Write the first spec", "In progress", "Luna", "Nov 07"),
            TableRow::new("r5", "Polish the sidebar", "Not started", "Milo", "Nov 12"),
        ];
        ws.add_page(tasks).expect("seed: tasks");

        let mut notes = make("p-notes", "Notes", "✎", None);
        notes.blocks = vec![
            Block::new("n1", BlockKind::Heading1, "Notes"),
            Block::new(
                "n2",
                BlockKind::Paragraph,
                "A scratch pad for ideas that do not belong anywhere yet.",
            ),
            Block::new(
                "n3",
                BlockKind::Code,
                "let idea = \"write it down\";",
            ),
            Block::new("n4", BlockKind::Divider, ""),
            Block::new(
                "n5",
                BlockKind::Paragraph,
                "Add a new block with the plus button on the left.",
            ),
        ];
        ws.add_page(notes).expect("seed: notes");

        ws
    }

    pub fn page(&self, id: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == id)
    }

    pub fn page_mut(&mut self, id: &str) -> Option<&mut Page> {
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
        Ok(removed)
    }

    pub fn children(&self, parent: Option<&str>) -> Vec<&Page> {
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

    /// Case-insensitive search over titles, block text, and row names.
    /// Ranking: title prefix (3) > title contains (2) > body contains (1).
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut ranked: Vec<(u32, usize, SearchResult)> = Vec::new();
        for (index, page) in self.pages.iter().enumerate() {
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

    pub fn bump_version(&mut self) {
        self.version = self.version.saturating_add(1);
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
