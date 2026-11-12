//! Block kinds, blocks, block DTOs and the block tree helpers.

use serde::{Deserialize, Serialize};

use super::rich_text::RichText;

/// The rendering and behavior variant of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Bulleted,
    Numbered,
    Todo,
    Quote,
    Callout,
    Code,
    Divider,
    Table,
    TableRow,
    Toggle,
    ChildPage,
    ColumnList,
    Column,
    Unsupported,
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
            BlockKind::Heading4 => "Heading 4",
            BlockKind::Bulleted => "Bulleted list",
            BlockKind::Numbered => "Numbered list",
            BlockKind::Todo => "To-do list",
            BlockKind::Quote => "Quote",
            BlockKind::Callout => "Callout",
            BlockKind::Code => "Code",
            BlockKind::Divider => "Divider",
            BlockKind::Table => "Table",
            BlockKind::TableRow => "Table row",
            BlockKind::Toggle => "Toggle",
            BlockKind::ChildPage => "Child page",
            BlockKind::ColumnList => "Column list",
            BlockKind::Column => "Column",
            BlockKind::Unsupported => "Unsupported block",
        }
    }

    pub fn is_divider(&self) -> bool {
        matches!(self, BlockKind::Divider)
    }

    pub fn is_heading(&self) -> bool {
        matches!(
            self,
            BlockKind::Heading1 | BlockKind::Heading2 | BlockKind::Heading3 | BlockKind::Heading4
        )
    }

    pub fn font_size(&self) -> f32 {
        match self {
            BlockKind::Heading1 => 30.0,
            BlockKind::Heading2 => 24.0,
            BlockKind::Heading3 => 19.0,
            BlockKind::Heading4 => 17.0,
            _ => 15.0,
        }
    }

    /// Map a v2 block `type` string to our UI-facing kind.
    pub fn from_api_type(value: &str) -> Self {
        match value {
            "paragraph" => BlockKind::Paragraph,
            "heading_1" => BlockKind::Heading1,
            "heading_2" => BlockKind::Heading2,
            "heading_3" => BlockKind::Heading3,
            "heading_4" => BlockKind::Heading4,
            "bulleted_list_item" => BlockKind::Bulleted,
            "numbered_list_item" => BlockKind::Numbered,
            "to_do" => BlockKind::Todo,
            "quote" => BlockKind::Quote,
            "callout" => BlockKind::Callout,
            "code" => BlockKind::Code,
            "divider" => BlockKind::Divider,
            "table" => BlockKind::Table,
            "table_row" => BlockKind::TableRow,
            "toggle" => BlockKind::Toggle,
            "child_page" => BlockKind::ChildPage,
            "column_list" => BlockKind::ColumnList,
            "column" => BlockKind::Column,
            _ => BlockKind::Unsupported,
        }
    }

    pub fn as_api_type(&self, unsupported_type: Option<&str>) -> String {
        match self {
            BlockKind::Paragraph => "paragraph",
            BlockKind::Heading1 => "heading_1",
            BlockKind::Heading2 => "heading_2",
            BlockKind::Heading3 => "heading_3",
            BlockKind::Heading4 => "heading_4",
            BlockKind::Bulleted => "bulleted_list_item",
            BlockKind::Numbered => "numbered_list_item",
            BlockKind::Todo => "to_do",
            BlockKind::Quote => "quote",
            BlockKind::Callout => "callout",
            BlockKind::Code => "code",
            BlockKind::Divider => "divider",
            BlockKind::Table => "table",
            BlockKind::TableRow => "table_row",
            BlockKind::Toggle => "toggle",
            BlockKind::ChildPage => "child_page",
            BlockKind::ColumnList => "column_list",
            BlockKind::Column => "column",
            BlockKind::Unsupported => unsupported_type.unwrap_or("unsupported"),
        }
        .to_string()
    }
}

fn default_block_object() -> String {
    "block".to_string()
}

/// One editable content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    #[serde(default = "default_block_object")]
    pub object: String,
    pub id: String,
    pub kind: BlockKind,
    /// Compatibility projection used by the egui editor and v1 API.
    pub text: String,
    #[serde(default)]
    pub rich_text: Vec<RichText>,
    pub checked: bool,
    pub collapsed: bool,
    #[serde(default)]
    pub children: Vec<Block>,
    #[serde(default)]
    pub parent_block: Option<String>,
    #[serde(default)]
    pub created_time: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub last_edited_time: String,
    #[serde(default)]
    pub last_edited_by: Option<String>,
    #[serde(default)]
    pub in_trash: bool,
    #[serde(default)]
    pub unsupported_type: Option<String>,
    #[serde(default)]
    pub table_width: Option<u32>,
    #[serde(default)]
    pub has_column_header: bool,
    #[serde(default)]
    pub has_row_header: bool,
    #[serde(default)]
    pub cells: Vec<Vec<RichText>>,
    #[serde(default)]
    pub child_page_title: Option<String>,
}

impl Block {
    pub fn new(id: impl Into<String>, kind: BlockKind, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            object: "block".to_string(),
            id: id.into(),
            kind,
            rich_text: vec![RichText::text(text.clone())],
            text,
            checked: false,
            collapsed: false,
            children: Vec::new(),
            parent_block: None,
            created_time: String::new(),
            created_by: None,
            last_edited_time: String::new(),
            last_edited_by: None,
            in_trash: false,
            unsupported_type: None,
            table_width: None,
            has_column_header: false,
            has_row_header: false,
            cells: Vec::new(),
            child_page_title: None,
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

    /// Rich text after applying the legacy `text` projection.
    ///
    /// If the UI edited `text`, it wins: a multi-object rich text is reduced
    /// to one plain text object rather than serving stale content.
    pub fn effective_rich_text(&self) -> Vec<RichText> {
        let joined = self
            .rich_text
            .iter()
            .map(RichText::plain_text)
            .collect::<Vec<_>>()
            .join("");
        if self.rich_text.is_empty() || joined != self.text {
            vec![RichText::text(self.text.clone())]
        } else {
            self.rich_text.clone()
        }
    }

    /// Plain text from the compatibility field, falling back to rich text.
    pub fn plain_text(&self) -> String {
        if self.text.is_empty() && !self.rich_text.is_empty() {
            self.rich_text
                .iter()
                .map(RichText::plain_text)
                .collect::<Vec<_>>()
                .join("")
        } else {
            self.text.clone()
        }
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Keep the legacy `text` field and the rich text array in sync.
    ///
    /// The UI edits `text`; v2 writes rich text. A single plain text object is
    /// treated as the compatibility projection. Rich content with links,
    /// mentions, equations or multiple objects is preserved.
    pub fn normalize(&mut self) {
        if self.rich_text.is_empty() {
            self.rich_text = vec![RichText::text(self.text.clone())];
        } else if self.text.is_empty() {
            self.text = self
                .rich_text
                .iter()
                .map(RichText::plain_text)
                .collect::<Vec<_>>()
                .join("");
        } else if self.rich_text.len() == 1
            && self.rich_text[0].is_plain_text()
            && self.rich_text[0].plain_text() != self.text
        {
            self.rich_text[0].set_plain_text(self.text.clone());
        }
        for child in &mut self.children {
            child.normalize();
        }
    }

    /// After a v2 rich-text write, update the compatibility text field.
    pub fn sync_text_from_rich_text(&mut self) {
        self.text = self
            .rich_text
            .iter()
            .map(RichText::plain_text)
            .collect::<Vec<_>>()
            .join("");
    }
}

pub(crate) fn walk_blocks(blocks: &[Block], visit: &mut impl FnMut(&Block)) {
    for block in blocks {
        visit(block);
        walk_blocks(&block.children, visit);
    }
}

pub(crate) fn walk_blocks_mut(blocks: &mut [Block], visit: &mut impl FnMut(&mut Block)) {
    for block in blocks {
        visit(block);
        walk_blocks_mut(&mut block.children, visit);
    }
}

pub(super) fn count_blocks(blocks: &[Block]) -> u64 {
    let mut count = 0;
    walk_blocks(blocks, &mut |_| count += 1);
    count
}

pub(super) fn find_block_in<'a>(blocks: &'a [Block], id: &str) -> Option<&'a Block> {
    for block in blocks {
        if block.id == id {
            return Some(block);
        }
        if let Some(found) = find_block_in(&block.children, id) {
            return Some(found);
        }
    }
    None
}

pub(super) fn find_block_in_mut<'a>(blocks: &'a mut [Block], id: &str) -> Option<&'a mut Block> {
    for block in blocks {
        if block.id == id {
            return Some(block);
        }
        if let Some(found) = find_block_in_mut(&mut block.children, id) {
            return Some(found);
        }
    }
    None
}
