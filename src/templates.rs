//! Starter templates and the first-run onboarding choices (spec-027).
//!
//! Templates are pure data: a title, icon, block blueprint and optional table
//! rows. `to_page` builds a fresh `Page` with stable per-page block ids.

use crate::model::{Block, BlockKind, Page, TableRow};

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub title: &'static str,
    pub blocks: &'static [(BlockKind, &'static str)],
    pub rows: &'static [(&'static str, &'static str, &'static str, &'static str)],
}

impl Template {
    /// Build a page from this template with the supplied page id.
    pub fn to_page(&self, id: impl Into<String>) -> Page {
        let id = id.into();
        let mut page = Page::new(id.clone(), self.title, self.icon);
        page.blocks = self
            .blocks
            .iter()
            .enumerate()
            .map(|(index, (kind, text))| Block::new(format!("{id}-b{index}"), *kind, *text))
            .collect();
        page.rows = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, (name, status, assignee, due))| {
                TableRow::new(format!("{id}-r{index}"), *name, *status, *assignee, *due)
            })
            .collect();
        page
    }
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "work-project",
        name: "Work project",
        description: "Track goals, tasks and owners.",
        icon: "◆",
        title: "Project tracker",
        blocks: &[
            (BlockKind::Heading1, "Project tracker"),
            (BlockKind::Paragraph, "A shared plan for the team."),
            (BlockKind::Heading2, "Goals"),
            (BlockKind::Bulleted, "Ship the first milestone"),
            (BlockKind::Todo, "Review the plan"),
            (BlockKind::Heading2, "Notes"),
            (BlockKind::Paragraph, ""),
        ],
        rows: &[
            ("Draft the brief", "In progress", "Ava", "Nov 10"),
            ("Review the design", "Not started", "Luna", "Nov 12"),
        ],
    },
    Template {
        id: "personal-notes",
        name: "Personal notes",
        description: "A calm place for daily notes.",
        icon: "✎",
        title: "Personal notes",
        blocks: &[
            (BlockKind::Heading1, "Personal notes"),
            (BlockKind::Paragraph, "Write down what matters today."),
            (BlockKind::Heading2, "Today"),
            (BlockKind::Todo, "Plan the day"),
            (BlockKind::Bulleted, "Read for 20 minutes"),
            (BlockKind::Quote, "Small steps add up."),
        ],
        rows: &[],
    },
    Template {
        id: "school-course",
        name: "Course notes",
        description: "Keep lectures, research and tasks together.",
        icon: "✦",
        title: "Course notes",
        blocks: &[
            (BlockKind::Heading1, "Course notes"),
            (BlockKind::Paragraph, "Lecture notes and study tasks."),
            (BlockKind::Heading2, "Lecture"),
            (BlockKind::Paragraph, ""),
            (BlockKind::Heading2, "Study tasks"),
            (BlockKind::Todo, "Review chapter 1"),
            (BlockKind::Todo, "Finish the assignment"),
        ],
        rows: &[],
    },
    Template {
        id: "meeting-notes",
        name: "Meeting notes",
        description: "Agenda, decisions and follow-ups.",
        icon: "☑",
        title: "Meeting notes",
        blocks: &[
            (BlockKind::Heading1, "Meeting notes"),
            (BlockKind::Paragraph, "Date and attendees."),
            (BlockKind::Heading2, "Agenda"),
            (BlockKind::Bulleted, ""),
            (BlockKind::Heading2, "Decisions"),
            (BlockKind::Paragraph, ""),
            (BlockKind::Heading2, "Follow-ups"),
            (BlockKind::Todo, "Send the summary"),
        ],
        rows: &[],
    },
    Template {
        id: "blank",
        name: "Blank page",
        description: "Start from an empty page.",
        icon: "P",
        title: "Untitled",
        blocks: &[(BlockKind::Paragraph, "")],
        rows: &[],
    },
];

/// All starter templates.
pub fn catalog() -> &'static [Template] {
    TEMPLATES
}

/// Find a template by id.
pub fn find(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|template| template.id == id)
}

/// The template used by an onboarding choice.
pub fn template_for_purpose(purpose: &str) -> Option<&'static Template> {
    let id = match purpose {
        "work" => "work-project",
        "personal" => "personal-notes",
        "school" => "school-course",
        _ => return None,
    };
    find(id)
}
