//! Workspace export: Notion-style markdown tree and People folder.

use crate::model::{BlockKind, Page, Workspace};

use super::{ExportFile, Member};

// ---------------------------------------------------------------------------
// Workspace export (Notion-style markdown tree + People folder)
// ---------------------------------------------------------------------------

fn clean(value: &str) -> String {
    value.trim().to_string()
}

/// Percent-encode spaces in a relative link path, matching the export format.
fn encode_path(value: &str) -> String {
    value.replace(' ', "%20")
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Render one page as markdown. A "folder" page additionally lists links to
/// its child pages inside its own markdown file.
pub fn page_markdown(page: &Page) -> String {
    let mut out = format!("# {}\n\n", page.title);
    for block in &page.blocks {
        let line = match block.kind {
            BlockKind::Paragraph | BlockKind::Callout => format!("{}\n\n", block.text),
            BlockKind::Heading1 => format!("# {}\n\n", block.text),
            BlockKind::Heading2 => format!("## {}\n\n", block.text),
            BlockKind::Heading3 => format!("### {}\n\n", block.text),
            BlockKind::Heading4 => format!("#### {}\n\n", block.text),
            BlockKind::Bulleted => format!("- {}\n", block.text),
            BlockKind::Numbered => format!("1. {}\n", block.text),
            BlockKind::Todo => format!(
                "- [{}] {}\n",
                if block.checked { "x" } else { " " },
                block.text
            ),
            BlockKind::Quote => format!("> {}\n\n", block.text),
            BlockKind::Code => format!("```\n{}\n```\n\n", block.text),
            BlockKind::Divider => "---\n\n".to_string(),
            BlockKind::Table
            | BlockKind::TableRow
            | BlockKind::Toggle
            | BlockKind::ChildPage
            | BlockKind::ColumnList
            | BlockKind::Column
            | BlockKind::Unsupported => format!("{}\n\n", block.plain_text()),
        };
        out.push_str(&line);
    }
    if !page.rows.is_empty() {
        out.push_str("\n| Name | Status | Assignee | Due |\n| --- | --- | --- | --- |\n");
        for row in &page.rows {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                row.name, row.status, row.assignee, row.due
            ));
        }
    }
    out
}

/// The People folder markdown: a link to the CSV and links to each person file,
/// followed by the view's `filters:` and `sort:` definition.
pub fn people_markdown(members: &[Member]) -> String {
    let mut md = String::from("# People\n\n");
    md.push_str("[People](People_all.csv)\n\n");
    for member in members {
        let target = format!("People/{} {}.md", clean(&member.name), member.id);
        md.push_str(&format!("[{}]({})\n\n", member.name, encode_path(&target)));
    }
    md.push_str("\nfilters: \nMembership Type\nsort: \nName: ascending\n");
    md
}

/// The `_all.csv` companion of a database / People folder.
pub fn people_csv(members: &[Member]) -> String {
    let mut csv = String::from("Name,Email,Membership Type\n");
    for member in members {
        csv.push_str(&format!(
            "{},{},{}\n",
            csv_escape(&member.name),
            csv_escape(&member.email),
            member.role.label()
        ));
    }
    csv
}

fn emit_page(workspace: &Workspace, page: &Page, dir: &str, files: &mut Vec<ExportFile>) {
    let name = clean(&page.title);
    let filename = format!("{} {}.md", name, page.id);
    let children = workspace.children(Some(&page.id));
    let mut content = page_markdown(page);
    if !children.is_empty() {
        // A folder page links to its sub-folders / sub-files in its own file.
        let child_dir = format!("{dir}/{name}");
        for child in &children {
            let target = format!("{}/{} {}.md", name, clean(&child.title), child.id);
            content.push_str(&format!("\n[{}]({})\n", child.title, encode_path(&target)));
            emit_page(workspace, child, &child_dir, files);
        }
    }
    files.push(ExportFile::new(format!("{dir}/{filename}"), content));
}

fn emit_people(members: &[Member], files: &mut Vec<ExportFile>) {
    files.push(ExportFile::new("People.md", people_markdown(members)));
    files.push(ExportFile::new("People_all.csv", people_csv(members)));
    for member in members {
        files.push(ExportFile::new(
            format!("People/{} {}.md", clean(&member.name), member.id),
            format!(
                "# {}\n\nMembership Type: {}\nPerson: {}\n",
                member.name,
                member.role.label(),
                member.name
            ),
        ));
    }
}

/// Build the whole workspace export as a flat list of files.
///
/// Each page becomes a markdown file named `Title <id>.md`; a page with
/// children is also a folder whose markdown lists links to its children.
/// A separate `People` folder records membership and access, with a companion
/// `_all.csv`, exactly as a workspace export does.
pub fn export_tree(workspace: &Workspace, members: &[Member]) -> Vec<ExportFile> {
    let mut files = Vec::new();
    let root = clean(&workspace.name);

    // Workspace root "folder" page: links to every top-level page.
    let mut root_md = format!("# {}\n\n", workspace.name);
    for page in workspace.children(None) {
        let target = format!("{}/{} {}.md", root, clean(&page.title), page.id);
        root_md.push_str(&format!("[{}]({})\n\n", page.title, encode_path(&target)));
    }
    files.push(ExportFile::new(format!("{} workspace.md", root), root_md));

    for page in workspace.children(None) {
        emit_page(workspace, page, &root, &mut files);
    }

    emit_people(members, &mut files);

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}
