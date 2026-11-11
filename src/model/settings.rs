//! Workspace-level settings persisted with the workspace document.

use serde::{Deserialize, Serialize};

fn default_id() -> String {
    "local-workspace".to_string()
}

fn default_icon() -> String {
    "N".to_string()
}

fn default_true() -> bool {
    true
}

/// How numeric values group thousands and mark the decimal separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NumberFormat {
    #[default]
    Automatic,
    DotThousands,
    CommaThousands,
}

/// How timestamps show the clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeFormat {
    #[default]
    Hour24,
    Hour12,
}

/// Settings shown in the General / Sidebar / People / Offline pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    #[serde(default = "default_id")]
    pub id: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default)]
    pub landing_page: Option<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub workspace_purpose: Option<String>,
    #[serde(default = "default_true")]
    pub sidebar_library: bool,
    #[serde(default = "default_true")]
    pub sidebar_settings: bool,
    #[serde(default = "default_true")]
    pub sidebar_inbox: bool,
    #[serde(default = "default_true")]
    pub sidebar_favorites: bool,
    #[serde(default = "default_true")]
    pub sidebar_private: bool,
    #[serde(default = "default_true")]
    pub sidebar_invite: bool,
    #[serde(default = "default_true")]
    pub sidebar_customize: bool,
    #[serde(default)]
    pub show_other_apps: bool,
    #[serde(default)]
    pub analytics: bool,
    #[serde(default)]
    pub offline_auto_download: bool,
    #[serde(default = "default_true")]
    pub people_directory: bool,
    #[serde(default = "default_true")]
    pub recent_activity: bool,
    #[serde(default = "default_true")]
    pub hover_cards: bool,
    #[serde(default)]
    pub number_format: NumberFormat,
    #[serde(default)]
    pub time_format: TimeFormat,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            id: default_id(),
            icon: default_icon(),
            landing_page: None,
            allowed_domains: Vec::new(),
            onboarding_completed: false,
            workspace_purpose: None,
            sidebar_library: true,
            sidebar_settings: true,
            sidebar_inbox: true,
            sidebar_favorites: true,
            sidebar_private: true,
            sidebar_invite: true,
            sidebar_customize: true,
            show_other_apps: false,
            analytics: false,
            offline_auto_download: false,
            people_directory: true,
            recent_activity: true,
            hover_cards: true,
            number_format: NumberFormat::Automatic,
            time_format: TimeFormat::Hour24,
        }
    }
}
