//! Pure workspace data model. No I/O, no UI types.
//!
//! `Workspace` owns an ordered set of `Page`s. Pages form a tree through
//! `parent`, hold editable `Block`s, and may carry inline `TableRow`s for a
//! simple database view.
//!
//! This root file is a facade: one module per domain group, re-exported here so
//! `crate::model::*` and the integration tests keep one stable surface.

use chrono::Utc;

mod block;
mod catalog;
mod comment;
mod database;
mod diff;
mod file;
mod index;
mod page;
mod property;
mod rich_text;
mod search;
mod seed;
mod settings;
mod user;
mod view;
mod workspace;

pub use block::{Block, BlockChildrenRequest, BlockDto, BlockInput, BlockKind, ParentDto};
pub use catalog::{WorkspaceCatalog, WorkspaceSummary};
pub use comment::{
    AttachmentInput, Comment, CommentDisplayName, CommentDisplayNameDto, CommentDto, CommentInput,
};
pub use database::{
    DataSource, DataSourceInput, DataSourceRef, Database, DatabaseInput, PagePropertiesInput,
    TableRow,
};
pub use diff::PageDiff;
pub use file::{CommentAttachment, FileObject, FileUpload, FileUploadInput, UploadStatus};
pub use page::{Page, PageDraft, PageSummary, SearchResult, WorkspaceError};
pub use property::{
    DateValue, FileRef, FormulaResult, OptionRef, PageProperty, PageRef, Property, PropertyConfig,
    PropertyType, PropertyValue, RollupResult, SelectOption, StatusGroup,
};
pub use rich_text::{Annotations, Equation, Mention, RichText, TextContent, TextLink};
pub use settings::{
    NumberFormat, SidebarLayout, SidebarSection, SidebarSectionSource, SidebarShortcut, TimeFormat,
    WorkspaceSettings,
};
pub use user::{Bot, BotOwner, Person, UserDto, UserRef, UserType, WorkspaceLimits};
pub use view::{
    Filter, Sort, SortDirection, View, ViewConfig, ViewGroup, ViewInput, ViewQueryInput,
    ViewQueryResult, ViewType,
};
pub use workspace::Workspace;

/// A short, human-friendly timestamp used for display only.
pub fn now_label() -> String {
    Utc::now().format("%b %d, %Y %H:%M").to_string()
}

/// An ISO-8601 timestamp for the v2 API.
pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
