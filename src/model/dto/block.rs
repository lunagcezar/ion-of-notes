//! v2 block DTOs and the domain-to-wire conversion.

use serde::{Deserialize, Serialize};

use super::super::{Block, BlockKind, RichText, UserRef, now_iso};

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

impl Block {
    /// Project one domain block to the v2 wire shape.
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

    /// Build a domain block from a v2 create/update payload.
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
