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

/// How a sidebar section gets its contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SidebarSectionSource {
    #[default]
    Custom,
    Favorites,
    Private,
}

/// One shortcut inside a custom sidebar section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarShortcut {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub page_id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub hidden: bool,
}

/// One sidebar section (a group inside a tab).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarSection {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub source: SidebarSectionSource,
    #[serde(default)]
    pub shortcuts: Vec<SidebarShortcut>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default = "default_tab_id")]
    pub tab: String,
}

fn default_tab_id() -> String {
    "workspace".to_string()
}

/// One sidebar tab: a named group of sections and shortcuts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarTab {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
}

/// The persisted sidebar layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarLayout {
    #[serde(default = "default_tabs")]
    pub tabs: Vec<SidebarTab>,
    #[serde(default = "default_tab_id")]
    pub active_tab: String,
    #[serde(default)]
    pub sections: Vec<SidebarSection>,
}

fn default_tabs() -> Vec<SidebarTab> {
    vec![SidebarTab {
        id: "workspace".to_string(),
        title: "Workspace".to_string(),
    }]
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            tabs: default_tabs(),
            active_tab: "workspace".to_string(),
            sections: vec![
                SidebarSection {
                    id: "favorites".to_string(),
                    title: "Favorites".to_string(),
                    source: SidebarSectionSource::Favorites,
                    shortcuts: Vec::new(),
                    hidden: false,
                    tab: "workspace".to_string(),
                },
                SidebarSection {
                    id: "private".to_string(),
                    title: "Private".to_string(),
                    source: SidebarSectionSource::Private,
                    shortcuts: Vec::new(),
                    hidden: false,
                    tab: "workspace".to_string(),
                },
            ],
        }
    }
}

impl SidebarLayout {
    pub fn visible_sections(&self) -> impl Iterator<Item = &SidebarSection> {
        self.sections
            .iter()
            .filter(|section| !section.hidden && section.tab == self.active_tab)
    }

    pub fn section_mut(&mut self, id: &str) -> Option<&mut SidebarSection> {
        self.sections.iter_mut().find(|section| section.id == id)
    }

    pub fn add_tab(&mut self) {
        let id = format!("tab-{}", self.tabs.len());
        let title = format!("Tab {}", self.tabs.len());
        self.tabs.push(SidebarTab {
            id: id.clone(),
            title,
        });
        self.active_tab = id;
    }

    pub fn switch_tab(&mut self, id: &str) -> bool {
        if self.tabs.iter().any(|tab| tab.id == id) {
            self.active_tab = id.to_string();
            true
        } else {
            false
        }
    }

    pub fn remove_tab(&mut self, id: &str) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        self.tabs.remove(index);
        self.sections.retain(|section| section.tab != id);
        if self.active_tab == id
            && let Some(first) = self.tabs.first()
        {
            self.active_tab = first.id.clone();
        }
        true
    }

    /// Add a shortcut to the first custom section in the active tab, creating
    /// a Shortcuts section when none exists yet.
    pub fn add_shortcut(&mut self, page_id: &str, label: &str) {
        let tab = self.active_tab.clone();
        let index = self.sections.iter().position(|section| {
            section.tab == tab && section.source == SidebarSectionSource::Custom
        });
        let index = index.unwrap_or_else(|| {
            self.sections.push(SidebarSection {
                id: format!("shortcuts-{tab}"),
                title: "Shortcuts".to_string(),
                source: SidebarSectionSource::Custom,
                shortcuts: Vec::new(),
                hidden: false,
                tab: tab.clone(),
            });
            self.sections.len() - 1
        });
        let section = &mut self.sections[index];
        if section.shortcuts.iter().any(|item| item.page_id == page_id) {
            return;
        }
        section.shortcuts.push(SidebarShortcut {
            id: format!("{page_id}-{}", section.shortcuts.len()),
            page_id: page_id.to_string(),
            label: label.to_string(),
            hidden: false,
        });
    }

    pub fn add_section(&mut self, title: &str, source: SidebarSectionSource) {
        let id = format!("section-{}", self.sections.len());
        let tab = self.active_tab.clone();
        self.sections.push(SidebarSection {
            id,
            title: title.to_string(),
            source,
            shortcuts: Vec::new(),
            hidden: false,
            tab,
        });
    }
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
    #[serde(default)]
    pub sidebar_layout: SidebarLayout,
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
            sidebar_layout: SidebarLayout::default(),
        }
    }
}
