//! Pure workspace data model. No I/O, no UI types.
//!
//! `Workspace` owns an ordered set of `Page`s. Pages form a tree through
//! `parent`, hold editable `Block`s, and may carry inline `TableRow`s for a
//! simple database view.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
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

/// An ISO-8601 timestamp for the v2 API.
pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// Rich-text annotations (spec-032).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotations {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub code: bool,
    pub color: String,
}

impl Default for Annotations {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            strikethrough: false,
            underline: false,
            code: false,
            color: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextLink {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<TextLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equation {
    pub expression: String,
}

/// A mention target. This is our v2 shape, not a byte-for-byte Notion clone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mention {
    Page { id: String },
    User { id: String },
    Date { start: String, end: Option<String> },
}

/// One rich-text object: text, mention or equation with annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichText {
    Text {
        text: TextContent,
        annotations: Annotations,
        plain_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    },
    Mention {
        mention: Mention,
        annotations: Annotations,
        plain_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    },
    Equation {
        equation: Equation,
        annotations: Annotations,
        plain_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        href: Option<String>,
    },
}

impl RichText {
    pub fn text(content: impl Into<String>) -> Self {
        let content = content.into();
        Self::Text {
            text: TextContent {
                content: content.clone(),
                link: None,
            },
            annotations: Annotations::default(),
            plain_text: content,
            href: None,
        }
    }

    pub fn with_link(content: impl Into<String>, url: impl Into<String>) -> Self {
        let content = content.into();
        let url = url.into();
        Self::Text {
            text: TextContent {
                content: content.clone(),
                link: Some(TextLink { url: url.clone() }),
            },
            annotations: Annotations::default(),
            plain_text: content,
            href: Some(url),
        }
    }

    pub fn equation(expression: impl Into<String>) -> Self {
        let expression = expression.into();
        Self::Equation {
            equation: Equation {
                expression: expression.clone(),
            },
            annotations: Annotations::default(),
            plain_text: expression,
            href: None,
        }
    }

    pub fn plain_text(&self) -> &str {
        match self {
            RichText::Text { plain_text, .. }
            | RichText::Mention { plain_text, .. }
            | RichText::Equation { plain_text, .. } => plain_text,
        }
    }

    pub fn set_plain_text(&mut self, value: impl Into<String>) {
        let value = value.into();
        match self {
            RichText::Text {
                text, plain_text, ..
            } => {
                text.content = value.clone();
                *plain_text = value;
            }
            RichText::Mention { plain_text, .. } | RichText::Equation { plain_text, .. } => {
                *plain_text = value;
            }
        }
    }

    pub fn is_plain_text(&self) -> bool {
        matches!(self, RichText::Text { text, .. } if text.link.is_none())
    }
}

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

    pub fn to_dto(&self, page_id: &str) -> BlockDto {
        let parent = match &self.parent_block {
            Some(block_id) => ParentDto::Block {
                block_id: block_id.clone(),
            },
            None => ParentDto::Page {
                page_id: page_id.to_string(),
            },
        };
        let created_time = if self.created_time.is_empty() {
            now_iso()
        } else {
            self.created_time.clone()
        };
        let last_edited_time = if self.last_edited_time.is_empty() {
            created_time.clone()
        } else {
            self.last_edited_time.clone()
        };
        BlockDto {
            object: "block".to_string(),
            id: self.id.clone(),
            parent,
            created_time,
            created_by: self.created_by.clone().map(UserRef::new),
            last_edited_time,
            last_edited_by: self.last_edited_by.clone().map(UserRef::new),
            in_trash: self.in_trash,
            has_children: self.has_children(),
            r#type: self.kind.as_api_type(self.unsupported_type.as_deref()),
            rich_text: self.effective_rich_text(),
            checked: (self.kind == BlockKind::Todo).then_some(self.checked),
            collapsed: (self.kind == BlockKind::Toggle).then_some(self.collapsed),
            table_width: self.table_width,
            has_column_header: (self.kind == BlockKind::Table).then_some(self.has_column_header),
            has_row_header: (self.kind == BlockKind::Table).then_some(self.has_row_header),
            cells: (self.kind == BlockKind::TableRow).then(|| self.cells.clone()),
            child_page_title: self.child_page_title.clone(),
            unsupported_type: self.unsupported_type.clone(),
        }
    }

    pub fn from_input(
        input: BlockInput,
        id: String,
        parent_block: Option<String>,
        actor: Option<String>,
    ) -> Self {
        let type_name = if input.r#type.is_empty() {
            "paragraph".to_string()
        } else {
            input.r#type.clone()
        };
        let kind = BlockKind::from_api_type(&type_name);
        let mut rich_text = input.rich_text;
        if rich_text.is_empty()
            && let Some(text) = input.text
        {
            rich_text = vec![RichText::text(text)];
        }
        let text = rich_text
            .iter()
            .map(RichText::plain_text)
            .collect::<Vec<_>>()
            .join("");
        let now = now_iso();
        let mut children = Vec::new();
        for child in input.children {
            let child_id = uuid::Uuid::new_v4().to_string();
            children.push(Block::from_input(
                child,
                child_id,
                Some(id.clone()),
                actor.clone(),
            ));
        }
        let unsupported_type = input.unsupported_type.or_else(|| {
            if kind == BlockKind::Unsupported {
                Some(type_name.clone())
            } else {
                None
            }
        });
        Self {
            object: "block".to_string(),
            id,
            kind,
            text,
            rich_text,
            checked: input.checked,
            collapsed: input.collapsed,
            children,
            parent_block,
            created_time: now.clone(),
            created_by: actor.clone(),
            last_edited_time: now,
            last_edited_by: actor,
            in_trash: false,
            unsupported_type,
            table_width: input.table_width,
            has_column_header: input.has_column_header,
            has_row_header: input.has_row_header,
            cells: input.cells,
            child_page_title: input.child_page_title,
        }
    }
}

/// A partial user reference in block metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRef {
    pub object: String,
    pub id: String,
}

impl UserRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            object: "user".to_string(),
            id: id.into(),
        }
    }
}

/// A block's parent object in the v2 API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParentDto {
    #[serde(rename = "page_id")]
    Page { page_id: String },
    #[serde(rename = "block_id")]
    Block { block_id: String },
    #[serde(rename = "database_id")]
    Database { database_id: String },
    #[serde(rename = "data_source_id")]
    DataSource { data_source_id: String },
    #[serde(rename = "workspace")]
    Workspace { workspace: bool },
}

/// The v2 block representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDto {
    pub object: String,
    pub id: String,
    pub parent: ParentDto,
    pub created_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<UserRef>,
    pub last_edited_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_edited_by: Option<UserRef>,
    pub in_trash: bool,
    pub has_children: bool,
    pub r#type: String,
    #[serde(default)]
    pub rich_text: Vec<RichText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_column_header: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_row_header: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cells: Option<Vec<Vec<RichText>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_page_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported_type: Option<String>,
}

/// A create/update block payload for v2 endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BlockInput {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub rich_text: Vec<RichText>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub collapsed: bool,
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
    #[serde(default)]
    pub children: Vec<BlockInput>,
    #[serde(default)]
    pub unsupported_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockChildrenRequest {
    #[serde(default)]
    pub children: Vec<BlockInput>,
}

/// A data-source property type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Title,
    RichText,
    Number,
    Select,
    MultiSelect,
    Status,
    Date,
    Checkbox,
    Url,
    Email,
    PhoneNumber,
    People,
    Files,
    Relation,
    Formula,
    Rollup,
    CreatedTime,
    CreatedBy,
    LastEditedTime,
    LastEditedBy,
    UniqueId,
}

impl PropertyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyType::Title => "title",
            PropertyType::RichText => "rich_text",
            PropertyType::Number => "number",
            PropertyType::Select => "select",
            PropertyType::MultiSelect => "multi_select",
            PropertyType::Status => "status",
            PropertyType::Date => "date",
            PropertyType::Checkbox => "checkbox",
            PropertyType::Url => "url",
            PropertyType::Email => "email",
            PropertyType::PhoneNumber => "phone_number",
            PropertyType::People => "people",
            PropertyType::Files => "files",
            PropertyType::Relation => "relation",
            PropertyType::Formula => "formula",
            PropertyType::Rollup => "rollup",
            PropertyType::CreatedTime => "created_time",
            PropertyType::CreatedBy => "created_by",
            PropertyType::LastEditedTime => "last_edited_time",
            PropertyType::LastEditedBy => "last_edited_by",
            PropertyType::UniqueId => "unique_id",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectOption {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl SelectOption {
    pub fn new(id: impl Into<String>, name: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color: color.into(),
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub option_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PropertyConfig {
    #[default]
    None,
    Number {
        format: String,
    },
    Select {
        options: Vec<SelectOption>,
    },
    MultiSelect {
        options: Vec<SelectOption>,
    },
    Status {
        default_option: Option<String>,
        options: Vec<SelectOption>,
        groups: Vec<StatusGroup>,
    },
    Relation {
        data_source_id: String,
        single_property: bool,
    },
    Formula {
        expression: String,
    },
    Rollup {
        relation_property_id: String,
        rollup_property_id: String,
        function: String,
    },
    UniqueId {
        prefix: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub r#type: PropertyType,
    #[serde(default)]
    pub config: PropertyConfig,
}

impl Property {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        r#type: PropertyType,
        config: PropertyConfig,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            r#type,
            config,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateValue {
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

impl DateValue {
    pub fn new(start: impl Into<String>) -> Self {
        Self {
            start: start.into(),
            end: None,
            time_zone: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionRef {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRef {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRef {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub expiry_time: Option<String>,
    #[serde(default)]
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormulaResult {
    String(Option<String>),
    Number(Option<i64>),
    Boolean(bool),
    Date(Option<DateValue>),
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollupResult {
    Number(Option<i64>),
    Date(Option<DateValue>),
    Array(Vec<PropertyValue>),
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyValue {
    Title(Vec<RichText>),
    RichText(Vec<RichText>),
    Number(Option<i64>),
    Select(Option<OptionRef>),
    MultiSelect(Vec<OptionRef>),
    Status(Option<OptionRef>),
    Date(Option<DateValue>),
    Checkbox(bool),
    Url(Option<String>),
    Email(Option<String>),
    PhoneNumber(Option<String>),
    People(Vec<UserRef>),
    Files(Vec<FileRef>),
    Relation(Vec<PageRef>),
    Formula(FormulaResult),
    Rollup(RollupResult),
    CreatedTime(String),
    CreatedBy(UserRef),
    LastEditedTime(String),
    LastEditedBy(UserRef),
    UniqueId { number: u64, prefix: Option<String> },
}

impl PropertyValue {
    /// Apply the v2 clear rule for a property type.
    pub fn clear(kind: PropertyType) -> Self {
        match kind {
            PropertyType::Title => PropertyValue::Title(Vec::new()),
            PropertyType::RichText => PropertyValue::RichText(Vec::new()),
            PropertyType::MultiSelect => PropertyValue::MultiSelect(Vec::new()),
            PropertyType::People => PropertyValue::People(Vec::new()),
            PropertyType::Files => PropertyValue::Files(Vec::new()),
            PropertyType::Relation => PropertyValue::Relation(Vec::new()),
            PropertyType::Checkbox => PropertyValue::Checkbox(false),
            PropertyType::Number => PropertyValue::Number(None),
            PropertyType::Select => PropertyValue::Select(None),
            PropertyType::Status => PropertyValue::Status(None),
            PropertyType::Date => PropertyValue::Date(None),
            PropertyType::Url => PropertyValue::Url(None),
            PropertyType::Email => PropertyValue::Email(None),
            PropertyType::PhoneNumber => PropertyValue::PhoneNumber(None),
            PropertyType::Formula => PropertyValue::Formula(FormulaResult::Unsupported),
            PropertyType::Rollup => PropertyValue::Rollup(RollupResult::Unsupported),
            PropertyType::CreatedTime => PropertyValue::CreatedTime(String::new()),
            PropertyType::CreatedBy => PropertyValue::CreatedBy(UserRef::new("")),
            PropertyType::LastEditedTime => PropertyValue::LastEditedTime(String::new()),
            PropertyType::LastEditedBy => PropertyValue::LastEditedBy(UserRef::new("")),
            PropertyType::UniqueId => PropertyValue::UniqueId {
                number: 0,
                prefix: None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageProperty {
    pub id: String,
    pub r#type: PropertyType,
    pub value: PropertyValue,
}

impl PageProperty {
    pub fn new(id: impl Into<String>, r#type: PropertyType, value: PropertyValue) -> Self {
        Self {
            id: id.into(),
            r#type,
            value,
        }
    }

    /// One item per value for list-like properties, one item otherwise.
    pub fn items(&self) -> Vec<PageProperty> {
        let make = |value: PropertyValue| PageProperty {
            id: self.id.clone(),
            r#type: self.r#type,
            value,
        };
        match &self.value {
            PropertyValue::Title(values) => values
                .iter()
                .map(|value| make(PropertyValue::Title(vec![value.clone()])))
                .collect(),
            PropertyValue::RichText(values) => values
                .iter()
                .map(|value| make(PropertyValue::RichText(vec![value.clone()])))
                .collect(),
            PropertyValue::People(values) => values
                .iter()
                .map(|value| make(PropertyValue::People(vec![value.clone()])))
                .collect(),
            PropertyValue::Relation(values) => values
                .iter()
                .map(|value| make(PropertyValue::Relation(vec![value.clone()])))
                .collect(),
            _ => vec![self.clone()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSourceRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Database {
    pub id: String,
    pub title: Vec<RichText>,
    #[serde(default)]
    pub data_sources: Vec<DataSourceRef>,
    pub parent: ParentDto,
    #[serde(default)]
    pub is_inline: bool,
    #[serde(default)]
    pub created_time: String,
    #[serde(default)]
    pub created_by: Option<UserRef>,
    #[serde(default)]
    pub last_edited_time: String,
    #[serde(default)]
    pub last_edited_by: Option<UserRef>,
    #[serde(default)]
    pub in_trash: bool,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSource {
    pub id: String,
    pub title: Vec<RichText>,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, Property>,
    pub parent: ParentDto,
    pub database_parent: ParentDto,
    #[serde(default)]
    pub rows: Vec<Page>,
    #[serde(default)]
    pub created_time: String,
    #[serde(default)]
    pub created_by: Option<UserRef>,
    #[serde(default)]
    pub last_edited_time: String,
    #[serde(default)]
    pub last_edited_by: Option<UserRef>,
    #[serde(default)]
    pub in_trash: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DatabaseInput {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub parent_page_id: Option<String>,
    #[serde(default)]
    pub is_inline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DataSourceInput {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, Property>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PagePropertiesInput {
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, PageProperty>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ViewType {
    #[default]
    Table,
    Board,
    Calendar,
    List,
    Gallery,
    Timeline,
    Form,
    Chart,
    Map,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sort {
    pub property: String,
    pub direction: SortDirection,
}

impl Sort {
    pub fn new(property: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            property: property.into(),
            direction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operator", rename_all = "snake_case")]
pub enum Filter {
    And { filters: Vec<Filter> },
    Or { filters: Vec<Filter> },
    Equals { property: String, value: String },
    NotEquals { property: String, value: String },
    Contains { property: String, value: String },
    IsEmpty { property: String },
    IsNotEmpty { property: String },
    GreaterThan { property: String, value: i64 },
    LessThan { property: String, value: i64 },
    Checked { property: String, value: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewConfig {
    #[default]
    Table,
    Board {
        group_by: String,
    },
    Calendar {
        date_property: String,
    },
    List,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub id: String,
    pub parent: ParentDto,
    #[serde(default)]
    pub data_source_id: Option<String>,
    pub name: String,
    pub r#type: ViewType,
    #[serde(default)]
    pub filter: Option<Filter>,
    #[serde(default)]
    pub sorts: Option<Vec<Sort>>,
    #[serde(default)]
    pub configuration: Option<ViewConfig>,
    #[serde(default)]
    pub created_time: String,
    #[serde(default)]
    pub created_by: Option<UserRef>,
    #[serde(default)]
    pub last_edited_time: String,
    #[serde(default)]
    pub last_edited_by: Option<UserRef>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub data_source_id: Option<String>,
    #[serde(default)]
    pub r#type: ViewType,
    #[serde(default)]
    pub filter: Option<Filter>,
    #[serde(default)]
    pub sorts: Option<Vec<Sort>>,
    #[serde(default)]
    pub configuration: Option<ViewConfig>,
}

impl Default for ViewInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_source_id: None,
            r#type: ViewType::Table,
            filter: None,
            sorts: None,
            configuration: Some(ViewConfig::Table),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ViewQueryInput {
    #[serde(default)]
    pub filter: Option<Filter>,
    #[serde(default)]
    pub sorts: Option<Vec<Sort>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewQueryResult {
    pub view_id: String,
    pub r#type: ViewType,
    pub groups: Vec<ViewGroup>,
    pub pages: Vec<Page>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl ViewQueryResult {
    pub fn new(
        view: &View,
        groups: Vec<ViewGroup>,
        pages: Vec<Page>,
        has_more: bool,
        next_cursor: Option<String>,
    ) -> Self {
        Self {
            view_id: view.id.clone(),
            r#type: view.r#type,
            groups,
            pages,
            has_more,
            next_cursor,
        }
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

/// One page-level discussion comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub text: String,
    pub created_at: String,
}

impl Comment {
    pub fn new(
        id: impl Into<String>,
        author: impl Into<String>,
        text: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            text: text.into(),
            created_at: created_at.into(),
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
    #[serde(default)]
    pub databases: Vec<Database>,
    #[serde(default)]
    pub data_sources: Vec<DataSource>,
    #[serde(default)]
    pub views: Vec<View>,
    #[serde(skip)]
    index: WorkspaceIndex,
}

impl PartialEq for Workspace {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.pages == other.pages
            && self.version == other.version
            && self.settings == other.settings
            && self.databases == other.databases
            && self.data_sources == other.data_sources
            && self.views == other.views
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
    #[serde(default)]
    databases: Vec<Database>,
    #[serde(default)]
    data_sources: Vec<DataSource>,
    #[serde(default)]
    views: Vec<View>,
}

impl<'de> Deserialize<'de> for Workspace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = WorkspaceData::deserialize(deserializer)?;
        let mut pages = data.pages;
        for page in &mut pages {
            page.normalize_blocks();
        }
        Ok(Self {
            index: WorkspaceIndex::build(&pages),
            name: data.name,
            pages,
            version: data.version,
            settings: data.settings,
            databases: data.databases,
            data_sources: data.data_sources,
            views: data.views,
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
            databases: Vec::new(),
            data_sources: Vec::new(),
            views: Vec::new(),
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

    /// Find a block anywhere in the page tree.
    pub fn block(&self, id: &str) -> Option<&Block> {
        self.pages
            .iter()
            .find_map(|page| find_block_in(&page.blocks, id))
    }

    /// Find a block anywhere in the page tree for mutation.
    pub fn block_mut(&mut self, id: &str) -> Option<&mut Block> {
        self.pages
            .iter_mut()
            .find_map(|page| find_block_in_mut(&mut page.blocks, id))
    }

    /// The page that contains a block.
    pub fn page_id_for_block(&self, id: &str) -> Option<String> {
        self.pages
            .iter()
            .find(|page| find_block_in(&page.blocks, id).is_some())
            .map(|page| page.id.clone())
    }

    /// Direct children of a block, or the top-level blocks of a page id.
    pub fn block_children(&self, id: &str) -> Option<&[Block]> {
        if let Some(page) = self.page(id) {
            return Some(&page.blocks);
        }
        self.block(id).map(|block| block.children.as_slice())
    }

    /// Mutable direct children of a block, or a page's top-level blocks.
    pub fn block_children_mut(&mut self, id: &str) -> Option<&mut Vec<Block>> {
        if let Some(index) = self.pages.iter().position(|page| page.id == id) {
            return Some(&mut self.pages[index].blocks);
        }
        for page in &mut self.pages {
            if let Some(block) = find_block_in_mut(&mut page.blocks, id) {
                return Some(&mut block.children);
            }
        }
        None
    }

    pub fn database(&self, id: &str) -> Option<&Database> {
        self.databases.iter().find(|database| database.id == id)
    }

    pub fn database_mut(&mut self, id: &str) -> Option<&mut Database> {
        self.databases.iter_mut().find(|database| database.id == id)
    }

    pub fn data_source(&self, id: &str) -> Option<&DataSource> {
        self.data_sources
            .iter()
            .find(|data_source| data_source.id == id)
    }

    pub fn data_source_mut(&mut self, id: &str) -> Option<&mut DataSource> {
        self.data_sources
            .iter_mut()
            .find(|data_source| data_source.id == id)
    }

    pub fn view(&self, id: &str) -> Option<&View> {
        self.views.iter().find(|view| view.id == id)
    }

    pub fn view_mut(&mut self, id: &str) -> Option<&mut View> {
        self.views.iter_mut().find(|view| view.id == id)
    }

    /// Create one default table view per data source. Idempotent.
    pub fn ensure_views(&mut self) {
        for data_source in &self.data_sources {
            let view_id = format!("view-{}", data_source.id);
            if self.views.iter().any(|view| view.id == view_id) {
                continue;
            }
            let now = now_iso();
            self.views.push(View {
                id: view_id,
                parent: data_source.parent.clone(),
                data_source_id: Some(data_source.id.clone()),
                name: "Table".to_string(),
                r#type: ViewType::Table,
                filter: None,
                sorts: None,
                configuration: Some(ViewConfig::Table),
                created_time: now.clone(),
                created_by: None,
                last_edited_time: now,
                last_edited_by: None,
                url: String::new(),
            });
        }
    }

    /// Apply a view's filter and sorts to its data-source rows.
    pub fn query_view_rows(
        &self,
        view: &View,
        filter: Option<&Filter>,
        sorts: Option<&[Sort]>,
    ) -> Vec<Page> {
        let Some(data_source_id) = view.data_source_id.as_deref() else {
            return Vec::new();
        };
        let Some(data_source) = self.data_source(data_source_id) else {
            return Vec::new();
        };
        let filter = filter.or(view.filter.as_ref());
        let sorts = sorts.or(view.sorts.as_deref());
        let mut rows: Vec<Page> = data_source
            .rows
            .iter()
            .filter(|row| filter.is_none_or(|filter| matches_filter(data_source, row, filter)))
            .cloned()
            .collect();
        if let Some(sorts) = sorts {
            rows.sort_by(|a, b| compare_pages(data_source, a, b, sorts));
        }
        rows
    }

    pub fn group_view_rows(&self, view: &View, rows: &[Page]) -> Vec<ViewGroup> {
        let Some(data_source_id) = view.data_source_id.as_deref() else {
            return Vec::new();
        };
        let Some(data_source) = self.data_source(data_source_id) else {
            return Vec::new();
        };
        let Some(config) = view.configuration.as_ref() else {
            return Vec::new();
        };
        let property_name = match config {
            ViewConfig::Board { group_by } => group_by,
            ViewConfig::Calendar { date_property } => date_property,
            _ => return Vec::new(),
        };
        let Some(property) = resolve_property(data_source, property_name) else {
            return Vec::new();
        };
        let mut groups: Vec<ViewGroup> = Vec::new();
        for row in rows {
            let value = row.properties.get(&property.id);
            let (name, color) = match value {
                Some(PageProperty {
                    value: PropertyValue::Status(Some(option)),
                    ..
                })
                | Some(PageProperty {
                    value: PropertyValue::Select(Some(option)),
                    ..
                }) => (option.name.clone(), option.color.clone()),
                Some(property) => (value_as_string(&property.value), "default".to_string()),
                None => ("No value".to_string(), "gray".to_string()),
            };
            if let Some(group) = groups.iter_mut().find(|group| group.name == name) {
                group.pages.push(row.clone());
            } else {
                let id = format!("{}-{}", view.id, name.to_lowercase().replace(' ', "-"));
                groups.push(ViewGroup {
                    id,
                    name,
                    color,
                    pages: vec![row.clone()],
                });
            }
        }
        groups
    }

    /// A page in the tree or a row inside a data source.
    pub fn page_or_row(&self, id: &str) -> Option<&Page> {
        self.page(id).or_else(|| {
            self.data_sources
                .iter()
                .flat_map(|data_source| data_source.rows.iter())
                .find(|page| page.id == id)
        })
    }

    pub fn page_or_row_mut(&mut self, id: &str) -> Option<&mut Page> {
        if let Some(index) = self.pages.iter().position(|page| page.id == id) {
            return Some(&mut self.pages[index]);
        }
        for data_source in &mut self.data_sources {
            if let Some(index) = data_source.rows.iter().position(|page| page.id == id) {
                return Some(&mut data_source.rows[index]);
            }
        }
        None
    }

    /// Create a default database/data source for pages that still use the
    /// legacy inline `TableRow` shape. Idempotent.
    pub fn ensure_data_sources(&mut self) {
        for page in &self.pages {
            if page.rows.is_empty() {
                continue;
            }
            let database_id = format!("db-{}", page.id);
            let data_source_id = format!("ds-{}", page.id);
            if self
                .data_sources
                .iter()
                .any(|data_source| data_source.id == data_source_id)
            {
                continue;
            }

            let now = now_iso();
            let status_options = vec![
                SelectOption::new("opt-not-started", "Not started", "gray"),
                SelectOption::new("opt-in-progress", "In progress", "blue"),
                SelectOption::new("opt-done", "Done", "green"),
            ];
            let properties = BTreeMap::from([
                (
                    "title".to_string(),
                    Property::new("title", "Name", PropertyType::Title, PropertyConfig::None),
                ),
                (
                    "status".to_string(),
                    Property::new(
                        "status",
                        "Status",
                        PropertyType::Status,
                        PropertyConfig::Status {
                            default_option: Some("Not started".to_string()),
                            options: status_options.clone(),
                            groups: vec![
                                StatusGroup {
                                    id: "group-todo".to_string(),
                                    name: "To-do".to_string(),
                                    color: "gray".to_string(),
                                    option_ids: vec!["opt-not-started".to_string()],
                                },
                                StatusGroup {
                                    id: "group-progress".to_string(),
                                    name: "In progress".to_string(),
                                    color: "blue".to_string(),
                                    option_ids: vec!["opt-in-progress".to_string()],
                                },
                                StatusGroup {
                                    id: "group-complete".to_string(),
                                    name: "Complete".to_string(),
                                    color: "green".to_string(),
                                    option_ids: vec!["opt-done".to_string()],
                                },
                            ],
                        },
                    ),
                ),
                (
                    "assignee".to_string(),
                    Property::new(
                        "assignee",
                        "Assignee",
                        PropertyType::RichText,
                        PropertyConfig::None,
                    ),
                ),
                (
                    "due".to_string(),
                    Property::new("due", "Due", PropertyType::Date, PropertyConfig::None),
                ),
            ]);

            let rows = page
                .rows
                .iter()
                .map(|row| {
                    let mut row_page = Page::new(format!("row-{}", row.id), row.name.clone(), "R");
                    row_page.blocks.clear();
                    row_page.data_source_id = Some(data_source_id.clone());
                    let status_id = format!("opt-{}", row.status.to_lowercase().replace(' ', "-"));
                    row_page.properties.insert(
                        "title".to_string(),
                        PageProperty::new(
                            "title",
                            PropertyType::Title,
                            PropertyValue::Title(vec![RichText::text(row.name.clone())]),
                        ),
                    );
                    row_page.properties.insert(
                        "status".to_string(),
                        PageProperty::new(
                            "status",
                            PropertyType::Status,
                            PropertyValue::Status(Some(OptionRef {
                                id: status_id,
                                name: row.status.clone(),
                                color: "blue".to_string(),
                            })),
                        ),
                    );
                    row_page.properties.insert(
                        "assignee".to_string(),
                        PageProperty::new(
                            "assignee",
                            PropertyType::RichText,
                            PropertyValue::RichText(vec![RichText::text(row.assignee.clone())]),
                        ),
                    );
                    row_page.properties.insert(
                        "due".to_string(),
                        PageProperty::new(
                            "due",
                            PropertyType::Date,
                            PropertyValue::Date(Some(DateValue::new(row.due.clone()))),
                        ),
                    );
                    row_page
                })
                .collect();

            if self.database(&database_id).is_none() {
                self.databases.push(Database {
                    id: database_id.clone(),
                    title: vec![RichText::text(page.title.clone())],
                    data_sources: vec![DataSourceRef {
                        id: data_source_id.clone(),
                        name: page.title.clone(),
                    }],
                    parent: ParentDto::Page {
                        page_id: page.id.clone(),
                    },
                    is_inline: true,
                    created_time: now.clone(),
                    created_by: None,
                    last_edited_time: now.clone(),
                    last_edited_by: None,
                    in_trash: false,
                    url: String::new(),
                });
            }
            self.data_sources.push(DataSource {
                id: data_source_id.clone(),
                title: vec![RichText::text(page.title.clone())],
                properties,
                parent: ParentDto::Database {
                    database_id: database_id.clone(),
                },
                database_parent: ParentDto::Page {
                    page_id: page.id.clone(),
                },
                rows,
                created_time: now.clone(),
                created_by: None,
                last_edited_time: now,
                last_edited_by: None,
                in_trash: false,
            });
        }
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
        self.pages
            .iter()
            .filter(|page| page.favorite && !page.archived)
            .collect()
    }

    pub fn bump_version(&mut self) {
        self.version = self.version.saturating_add(1);
    }
}

fn find_block_in<'a>(blocks: &'a [Block], id: &str) -> Option<&'a Block> {
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

fn find_block_in_mut<'a>(blocks: &'a mut [Block], id: &str) -> Option<&'a mut Block> {
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

fn resolve_property<'a>(data_source: &'a DataSource, key: &str) -> Option<&'a Property> {
    data_source.properties.get(key).or_else(|| {
        data_source
            .properties
            .values()
            .find(|property| property.id == key)
    })
}

fn value_as_string(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Title(values) | PropertyValue::RichText(values) => values
            .iter()
            .map(RichText::plain_text)
            .collect::<Vec<_>>()
            .join(""),
        PropertyValue::Number(value) => value.map(|n| n.to_string()).unwrap_or_default(),
        PropertyValue::Select(option) | PropertyValue::Status(option) => option
            .as_ref()
            .map(|option| option.name.clone())
            .unwrap_or_default(),
        PropertyValue::MultiSelect(options) => options
            .iter()
            .map(|option| option.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Date(date) => date
            .as_ref()
            .map(|date| date.start.clone())
            .unwrap_or_default(),
        PropertyValue::Checkbox(value) => value.to_string(),
        PropertyValue::Url(value)
        | PropertyValue::Email(value)
        | PropertyValue::PhoneNumber(value) => value.clone().unwrap_or_default(),
        PropertyValue::People(users) => users
            .iter()
            .map(|user| user.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Files(files) => files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Relation(pages) => pages
            .iter()
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        PropertyValue::Formula(result) => match result {
            FormulaResult::String(value) => value.clone().unwrap_or_default(),
            FormulaResult::Number(value) => value.map(|n| n.to_string()).unwrap_or_default(),
            FormulaResult::Boolean(value) => value.to_string(),
            FormulaResult::Date(value) => value
                .as_ref()
                .map(|date| date.start.clone())
                .unwrap_or_default(),
            FormulaResult::Unsupported => String::new(),
        },
        PropertyValue::Rollup(result) => match result {
            RollupResult::Number(value) => value.map(|n| n.to_string()).unwrap_or_default(),
            RollupResult::Date(value) => value
                .as_ref()
                .map(|date| date.start.clone())
                .unwrap_or_default(),
            RollupResult::Array(values) => values
                .iter()
                .map(value_as_string)
                .collect::<Vec<_>>()
                .join(", "),
            RollupResult::Unsupported => String::new(),
        },
        PropertyValue::CreatedTime(value) | PropertyValue::LastEditedTime(value) => value.clone(),
        PropertyValue::CreatedBy(user) | PropertyValue::LastEditedBy(user) => user.id.clone(),
        PropertyValue::UniqueId { number, prefix } => {
            format!("{}{number}", prefix.clone().unwrap_or_default())
        }
    }
}

fn value_is_empty(value: &PropertyValue) -> bool {
    match value {
        PropertyValue::Title(values) | PropertyValue::RichText(values) => values.is_empty(),
        PropertyValue::MultiSelect(values) => values.is_empty(),
        PropertyValue::People(values) => values.is_empty(),
        PropertyValue::Files(values) => values.is_empty(),
        PropertyValue::Relation(values) => values.is_empty(),
        PropertyValue::Number(value) => value.is_none(),
        PropertyValue::Select(value) | PropertyValue::Status(value) => value.is_none(),
        PropertyValue::Date(value) => value.is_none(),
        PropertyValue::Checkbox(value) => !value,
        PropertyValue::Url(value)
        | PropertyValue::Email(value)
        | PropertyValue::PhoneNumber(value) => value.as_deref().is_none_or(str::is_empty),
        PropertyValue::Formula(FormulaResult::Unsupported)
        | PropertyValue::Rollup(RollupResult::Unsupported) => true,
        _ => value_as_string(value).is_empty(),
    }
}

fn value_as_number(value: &PropertyValue) -> Option<i64> {
    match value {
        PropertyValue::Number(value) => *value,
        PropertyValue::Formula(FormulaResult::Number(value)) => *value,
        PropertyValue::Rollup(RollupResult::Number(value)) => *value,
        PropertyValue::UniqueId { number, .. } => i64::try_from(*number).ok(),
        _ => value_as_string(value).parse().ok(),
    }
}

fn matches_filter(data_source: &DataSource, row: &Page, filter: &Filter) -> bool {
    match filter {
        Filter::And { filters } => filters
            .iter()
            .all(|filter| matches_filter(data_source, row, filter)),
        Filter::Or { filters } => filters
            .iter()
            .any(|filter| matches_filter(data_source, row, filter)),
        Filter::Equals { property, value } => property_value(data_source, row, property)
            .is_some_and(|actual| value_as_string(actual).eq_ignore_ascii_case(value)),
        Filter::NotEquals { property, value } => property_value(data_source, row, property)
            .is_none_or(|actual| !value_as_string(actual).eq_ignore_ascii_case(value)),
        Filter::Contains { property, value } => property_value(data_source, row, property)
            .is_some_and(|actual| {
                value_as_string(actual)
                    .to_lowercase()
                    .contains(&value.to_lowercase())
            }),
        Filter::IsEmpty { property } => {
            property_value(data_source, row, property).is_none_or(value_is_empty)
        }
        Filter::IsNotEmpty { property } => {
            property_value(data_source, row, property).is_some_and(|actual| !value_is_empty(actual))
        }
        Filter::GreaterThan { property, value } => property_value(data_source, row, property)
            .and_then(value_as_number)
            .is_some_and(|number| number > *value),
        Filter::LessThan { property, value } => property_value(data_source, row, property)
            .and_then(value_as_number)
            .is_some_and(|number| number < *value),
        Filter::Checked { property, value } => property_value(data_source, row, property)
            .is_some_and(
                |actual| matches!(actual, PropertyValue::Checkbox(checked) if checked == value),
            ),
    }
}

fn property_value<'a>(
    data_source: &DataSource,
    row: &'a Page,
    property: &str,
) -> Option<&'a PropertyValue> {
    let property = resolve_property(data_source, property)?;
    row.properties
        .get(&property.id)
        .map(|property| &property.value)
}

fn compare_pages(data_source: &DataSource, a: &Page, b: &Page, sorts: &[Sort]) -> Ordering {
    for sort in sorts {
        let left = property_value(data_source, a, &sort.property);
        let right = property_value(data_source, b, &sort.property);
        let ordering = match (left, right) {
            (Some(left), Some(right)) => match (value_as_number(left), value_as_number(right)) {
                (Some(left), Some(right)) => left.cmp(&right),
                _ => value_as_string(left).cmp(&value_as_string(right)),
            },
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };
        let ordering = match sort.direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}
