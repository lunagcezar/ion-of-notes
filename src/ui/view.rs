//! The immutable view snapshot passed to the pure render modules.
//!
//! `ViewState` is built once per frame from `NotionApp`; render functions only
//! read it. Keeping the shape separate from the state machine keeps rendering
//! independent of command handling (SRP, spec-017).

use super::commands::{FontStyle, IconTab, SidebarMenu, WorkspaceDialogMode};
use crate::collab::{Access, Member, PageVersion};
use crate::model::{SearchResult, WorkspaceSummary};
use crate::server::realtime::Awareness;
use egui::Pos2;
use std::collections::HashSet;
use std::sync::Arc;

/// Search-specific presentation data.
pub(super) struct SearchView {
    pub(super) open: bool,
    pub(super) query: String,
    pub(super) results: Arc<Vec<SearchResult>>,
    pub(super) title_only: bool,
    pub(super) created_by: Option<String>,
    pub(super) in_page: Option<String>,
    pub(super) has_comments: bool,
    pub(super) selected: usize,
}

/// Sharing-specific presentation data.
pub(super) struct SharingView {
    pub(super) open: bool,
    pub(super) members: Arc<Vec<Member>>,
    pub(super) invite_input: String,
    pub(super) invite_error: Option<String>,
    pub(super) invite_status: Option<String>,
    pub(super) general_access: Access,
}

/// AI assistant presentation data.
pub(super) struct AiView {
    pub(super) answer: Option<String>,
    pub(super) error: Option<String>,
    pub(super) busy: bool,
    pub(super) status: String,
    pub(super) checking: bool,
    pub(super) connected: bool,
    pub(super) endpoint: String,
    pub(super) model: String,
    pub(super) prefer_cli: bool,
}

/// Server login presentation data.
pub(super) struct ServerView {
    pub(super) url: String,
    pub(super) email: String,
    pub(super) password: String,
    pub(super) status: Option<String>,
    pub(super) connected: bool,
    pub(super) busy: bool,
}

/// A snapshot of non-workspace app state passed to pure render functions.
pub(super) struct ViewState {
    pub(super) current_page: String,
    pub(super) page_icon: String,
    pub(super) remote_cursors: Vec<Awareness>,
    pub(super) server: ServerView,
    pub(super) tabs: Vec<String>,
    pub(super) active_tab: usize,
    pub(super) sidebar_open: bool,
    pub(super) expanded: HashSet<String>,
    pub(super) search: SearchView,
    pub(super) settings_open: bool,
    pub(super) onboarding_open: bool,
    pub(super) templates_open: bool,
    pub(super) settings_page: String,
    pub(super) settings_query: String,
    pub(super) settings_status: Option<String>,
    pub(super) language: String,
    pub(super) number_format: crate::model::NumberFormat,
    pub(super) time_format: crate::model::TimeFormat,
    pub(super) dark: bool,
    pub(super) table_view: bool,
    pub(super) library_open: bool,
    pub(super) library_tab: String,
    pub(super) library_shared: Arc<HashSet<String>>,
    pub(super) library_owner: String,
    pub(super) sharing: SharingView,
    pub(super) inbox_open: bool,
    pub(super) inbox_unread: usize,
    pub(super) slash_for: Option<String>,
    pub(super) slash_pos: Pos2,
    pub(super) slash_query: String,
    pub(super) slash_dismissed: Option<String>,
    pub(super) sidebar_customize: bool,
    pub(super) sidebar_menu: Option<SidebarMenu>,
    pub(super) sidebar_layout: crate::model::SidebarLayout,
    pub(super) workspaces: Vec<WorkspaceSummary>,
    pub(super) active_workspace: String,
    pub(super) workspace_dialog: Option<WorkspaceDialogMode>,
    pub(super) workspace_name: String,
    pub(super) workspace_icon: String,
    pub(super) new_tab_open: bool,
    pub(super) new_tab_query: String,
    pub(super) new_tab_selected: usize,
    pub(super) ai: AiView,
    pub(super) shortcuts_open: bool,
    pub(super) highlight_pane: bool,
    pub(super) filters: bool,
    pub(super) icon_picker_open: bool,
    pub(super) icon_picker_tab: IconTab,
    pub(super) icon_picker_query: String,
    pub(super) page_actions_open: bool,
    pub(super) page_status: Option<String>,
    pub(super) comment_input: String,
    pub(super) version_history_open: bool,
    pub(super) versions: Arc<Vec<PageVersion>>,
    pub(super) version_selected: usize,
    pub(super) font_style: FontStyle,
    pub(super) full_width: bool,
    pub(super) small_text: bool,
    pub(super) available_offline: bool,
    pub(super) lock_page: bool,
    pub(super) workspace_menu_open: bool,
    pub(super) app_menu_open: bool,
    pub(super) compose_menu_open: bool,
    pub(super) grid_menu_open: bool,
    pub(super) drag_from: Option<String>,
    pub(super) drag_to: Option<String>,
    pub(super) drag_before: bool,
    pub(super) focus_block: Option<String>,
    pub(super) page_selection: Vec<String>,
    pub(super) block_selecting: bool,
}
