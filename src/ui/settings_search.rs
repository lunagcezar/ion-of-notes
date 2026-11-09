//! Pure settings search matching and the settings row table (no egui, no
//! state).
//!
//! The settings window keeps its navigation and rows in one place; this helper
//! decides whether a section or row matches the query and owns the row catalog
//! used by both filtering and rendering. Keeping it pure makes the behavior
//! testable without opening a window (spec-015, spec-016 A9).

/// One searchable row inside a settings page.
///
/// `title` and `description` are the canonical English search text; the render
/// functions may display a translated label instead, but the catalog stays the
/// single source of truth for matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsRow {
    pub id: &'static str,
    pub section: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// Return true when the query is empty or appears in the title/description.
///
/// Matching is case-insensitive and trims the query, so typing spaces does not
/// hide every section.
pub fn settings_matches(query: &str, title: &str, description: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    title.to_lowercase().contains(&query) || description.to_lowercase().contains(&query)
}

const PREFERENCES_ROWS: &[SettingsRow] = &[
    SettingsRow {
        id: "theme",
        section: "Appearance",
        title: "Theme",
        description: "Choose a theme for this device (Ctrl+Shift+L)",
    },
    SettingsRow {
        id: "high_contrast",
        section: "Appearance",
        title: "High contrast  Beta",
        description: "Increase contrast for improved visibility",
    },
    SettingsRow {
        id: "enter_new_line",
        section: "Input options",
        title: "Use Enter to add a new line",
        description: "Applies to chat and comments. Press Ctrl+Enter to send.",
    },
    SettingsRow {
        id: "language",
        section: "Language & time",
        title: "Language",
        description: "Choose the language used by the interface",
    },
    SettingsRow {
        id: "number_format",
        section: "Language & time",
        title: "Number format",
        description: "Choose how numbers and currencies are formatted",
    },
    SettingsRow {
        id: "time_format",
        section: "Language & time",
        title: "Time format",
        description: "Choose 12-hour or 24-hour time display",
    },
    SettingsRow {
        id: "server",
        section: "Workspace & sync",
        title: "Self-hosted server",
        description: "Documents, members and permissions live on your server and sync to every signed-in device.",
    },
    SettingsRow {
        id: "accounts",
        section: "Workspace & sync",
        title: "Accounts",
        description: "Workspace owners and guests",
    },
    SettingsRow {
        id: "sharing",
        section: "Workspace & sync",
        title: "Sharing",
        description: "Invite by email with full, edit or view access",
    },
    SettingsRow {
        id: "sync",
        section: "Workspace & sync",
        title: "Sync",
        description: "Versioned changes across devices",
    },
    SettingsRow {
        id: "realtime",
        section: "Workspace & sync",
        title: "Real-time",
        description: "Live co-editing over WebSocket",
    },
    SettingsRow {
        id: "export",
        section: "Workspace & sync",
        title: "Export",
        description: "Markdown tree and People CSV",
    },
    SettingsRow {
        id: "license",
        section: "Workspace & sync",
        title: "License",
        description: "Open source under the MIT license",
    },
    SettingsRow {
        id: "restore_demo",
        section: "Workspace & sync",
        title: "Restore demo workspace",
        description: "Reset this workspace to the seeded demo content",
    },
];

const GENERAL_ROWS: &[SettingsRow] = &[
    SettingsRow {
        id: "workspace_name",
        section: "Workspace settings",
        title: "Workspace name",
        description: "Rename this workspace",
    },
    SettingsRow {
        id: "icon",
        section: "Workspace settings",
        title: "Icon",
        description: "Choose the workspace glyph",
    },
    SettingsRow {
        id: "landing_page",
        section: "Workspace settings",
        title: "Landing page",
        description: "Choose the first page shown",
    },
    SettingsRow {
        id: "allowed_domains",
        section: "Allowed domains",
        title: "Allowed domains",
        description: "Restrict sign-ups to email domains",
    },
    SettingsRow {
        id: "get_started",
        section: "Allowed domains",
        title: "Get started",
        description: "Open the onboarding questions",
    },
    SettingsRow {
        id: "members_access",
        section: "Members & access",
        title: "Members & access",
        description: "Invite people and manage access",
    },
    SettingsRow {
        id: "data_export",
        section: "Data export",
        title: "Data export",
        description: "Export pages as Markdown and people as CSV",
    },
];

const SIDEBAR_ROWS: &[SettingsRow] = &[
    SettingsRow {
        id: "library",
        section: "Sidebar items",
        title: "Library",
        description: "Show the Library entry",
    },
    SettingsRow {
        id: "settings",
        section: "Sidebar items",
        title: "Settings",
        description: "Show the Settings entry",
    },
    SettingsRow {
        id: "inbox",
        section: "Sidebar items",
        title: "Inbox",
        description: "Show the Inbox entry",
    },
    SettingsRow {
        id: "favorites",
        section: "Sidebar items",
        title: "Favorites",
        description: "Show the Favorites entry",
    },
    SettingsRow {
        id: "private",
        section: "Sidebar items",
        title: "Private",
        description: "Show the Private entry",
    },
    SettingsRow {
        id: "invite_card",
        section: "Sidebar items",
        title: "Invite members card",
        description: "Show the invite card",
    },
    SettingsRow {
        id: "customize",
        section: "Sidebar items",
        title: "Customize sidebar",
        description: "Show the customize control",
    },
    SettingsRow {
        id: "other_apps",
        section: "Sidebar items",
        title: "Show other apps in sidebar",
        description: "Show other app shortcuts",
    },
    SettingsRow {
        id: "export",
        section: "Export",
        title: "Export",
        description: "Export workspace and members",
    },
    SettingsRow {
        id: "analytics",
        section: "Analytics",
        title: "Save analytics",
        description: "Store workspace analytics preferences",
    },
];

const OFFLINE_ROWS: &[SettingsRow] = &[
    SettingsRow {
        id: "auto_download",
        section: "Offline",
        title: "Automatic downloads",
        description: "Download pages for offline use",
    },
    SettingsRow {
        id: "offline_pages",
        section: "Offline",
        title: "Offline pages",
        description: "Search pages available offline",
    },
];

const PEOPLE_ROWS: &[SettingsRow] = &[
    SettingsRow {
        id: "people_directory",
        section: "Directory",
        title: "People directory",
        description: "Show the workspace directory",
    },
    SettingsRow {
        id: "recent_activity",
        section: "Directory",
        title: "Show recent activity",
        description: "Show recent member activity",
    },
    SettingsRow {
        id: "hover_cards",
        section: "Directory",
        title: "Hover cards",
        description: "Show profile cards on hover",
    },
    SettingsRow {
        id: "members",
        section: "Members",
        title: "Members",
        description: "People with access to this workspace",
    },
    SettingsRow {
        id: "danger_zone",
        section: "Danger zone",
        title: "Delete workspace",
        description: "Permanently delete this workspace",
    },
    SettingsRow {
        id: "workspace_id",
        section: "Danger zone",
        title: "Workspace ID",
        description: "Identifier for this self-hosted workspace",
    },
];

const AI_ROWS: &[SettingsRow] = &[
    SettingsRow {
        id: "connection",
        section: "Connection",
        title: "Connection status",
        description: "Check whether Ollama is reachable",
    },
    SettingsRow {
        id: "endpoint",
        section: "Connection",
        title: "Service endpoint",
        description: "host:port on your network",
    },
    SettingsRow {
        id: "model",
        section: "Connection",
        title: "Model",
        description: "Model name sent to Ollama",
    },
    SettingsRow {
        id: "prefer_cli",
        section: "Connection",
        title: "Prefer the ollama CLI",
        description: "Use the CLI when it is in PATH",
    },
    SettingsRow {
        id: "try_it",
        section: "Try it",
        title: "Ask a test question",
        description: "Send a short prompt to the configured model",
    },
    SettingsRow {
        id: "shortcut",
        section: "Try it",
        title: "Shortcut",
        description: "Shift+Ctrl+J opens the assistant",
    },
];

/// The row catalog for one settings page. Pages without rows use the generic
/// section description.
pub fn rows_for(page: &str) -> &'static [SettingsRow] {
    match page {
        "Preferences" => PREFERENCES_ROWS,
        "General" => GENERAL_ROWS,
        "Sidebar" => SIDEBAR_ROWS,
        "Offline" => OFFLINE_ROWS,
        "People" => PEOPLE_ROWS,
        "AI assistant" => AI_ROWS,
        _ => &[],
    }
}

/// Rows inside one page that match the query, in display order.
pub fn visible_rows(page: &str, query: &str) -> Vec<&'static SettingsRow> {
    rows_for(page)
        .iter()
        .filter(|row| settings_matches(query, row.title, row.description))
        .collect()
}

/// Whether a settings section should stay in the rail: either the section
/// itself matches, or at least one row inside it matches.
pub fn section_matches(query: &str, page: &str, description: &str) -> bool {
    settings_matches(query, page, description) || !visible_rows(page, query).is_empty()
}
