//! Workspace tree behavior, derived index and JSON deserialization.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use super::block::{Block, ParentDto, find_block_in, find_block_in_mut};
use super::comment::CommentDto;
use super::database::{DataSource, DataSourceRef, Database};
use super::file::FileUpload;
use super::index::WorkspaceIndex;
use super::now_iso;
use super::page::{Page, WorkspaceError};
use super::property::{
    DateValue, OptionRef, PageProperty, Property, PropertyConfig, PropertyType, PropertyValue,
    SelectOption, StatusGroup, resolve_property, value_as_string,
};
use super::rich_text::RichText;
use super::settings::WorkspaceSettings;
use super::view::{
    Filter, Sort, View, ViewConfig, ViewGroup, ViewType, compare_pages, matches_filter,
};
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
    #[serde(default)]
    pub comments: Vec<CommentDto>,
    #[serde(default)]
    pub file_uploads: Vec<FileUpload>,
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
            && self.comments == other.comments
            && self.file_uploads == other.file_uploads
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
    #[serde(default)]
    comments: Vec<CommentDto>,
    #[serde(default)]
    file_uploads: Vec<FileUpload>,
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
            comments: data.comments,
            file_uploads: data.file_uploads,
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
            comments: Vec::new(),
            file_uploads: Vec::new(),
            index: WorkspaceIndex::default(),
        }
    }

    /// Lifetime block usage of the whole workspace (spec-037).
    pub fn block_count(&self) -> u64 {
        self.pages.iter().map(Page::block_count).sum()
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
