//! Databases, data sources, table rows and page-property input.

use serde::{Deserialize, Serialize};

use super::dto::ParentDto;
use super::page::Page;
use super::property::{PageProperty, Property};
use super::rich_text::RichText;
use super::user::UserRef;
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
