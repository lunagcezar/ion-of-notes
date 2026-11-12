//! Shared "Open in new tab…" and quick-search windows.
//!
//! Both surfaces build rows and then hand one spec to the single switcher
//! renderer, so the layout exists once (DRY, spec-013/spec-017).

use crate::model::{Page, Workspace};

use super::super::commands::{Command, NewTabChoice, new_tab_choices};
use super::super::palette::Palette;
use super::super::switcher::{SwitcherMode, SwitcherRow, SwitcherSpec, render_switcher};
use super::super::view::ViewState;

/// A breadcrumb-style suffix shown after a result title.
fn page_detail(ws: &Workspace, page: &Page) -> String {
    let trail = ws.breadcrumbs(&page.id);
    let parents: Vec<String> = trail[..trail.len().saturating_sub(1)].to_vec();
    let text = if parents.is_empty() {
        ws.name.clone()
    } else {
        parents.join(" / ")
    };
    format!("   — {text}")
}

fn page_in_scope(ws: &Workspace, page_id: &str, scope: &str) -> bool {
    let mut cursor = Some(page_id.to_string());
    let mut guard = 0;
    while let Some(id) = cursor {
        if id == scope {
            return true;
        }
        guard += 1;
        if guard > 64 {
            break;
        }
        cursor = ws.page(&id).and_then(|page| page.parent.clone());
    }
    false
}

pub fn page_matches(
    ws: &Workspace,
    page: &Page,
    query: &str,
    title_only: bool,
    created_by: Option<&str>,
    in_page: Option<&str>,
    has_comments: bool,
) -> bool {
    let query = query.trim().to_lowercase();
    if title_only && !query.is_empty() && !page.title.to_lowercase().contains(&query) {
        return false;
    }
    if let Some(name) = created_by
        && page.created_by != name
    {
        return false;
    }
    if let Some(scope) = in_page
        && !page_in_scope(ws, &page.id, scope)
    {
        return false;
    }
    if has_comments && page.comments.is_empty() {
        return false;
    }
    true
}

fn build_new_tab_rows(
    ws: &Workspace,
    view: &ViewState,
    choices: &[NewTabChoice],
) -> Vec<SwitcherRow> {
    choices
        .iter()
        .filter(|choice| match choice {
            NewTabChoice::Page(id) => ws.page(id).is_some_and(|page| {
                page_matches(
                    ws,
                    page,
                    &view.new_tab_query,
                    view.search.title_only,
                    view.search.created_by.as_deref(),
                    view.search.in_page.as_deref(),
                    view.search.has_comments,
                )
            }),
            _ => true,
        })
        .enumerate()
        .map(|(index, choice)| {
            let (label, icon, id, special, detail) = match choice {
                NewTabChoice::NewChat => (
                    "New chat".to_string(),
                    String::new(),
                    "new-chat".to_string(),
                    true,
                    String::new(),
                ),
                NewTabChoice::NewPage => (
                    "New page".to_string(),
                    String::new(),
                    "new-page".to_string(),
                    true,
                    String::new(),
                ),
                NewTabChoice::Page(id) => {
                    let page = ws.page(id);
                    (
                        page.map(|p| p.title.clone())
                            .unwrap_or_else(|| "Untitled".to_string()),
                        page.map(|p| p.icon.clone()).unwrap_or_default(),
                        id.clone(),
                        false,
                        page.map(|p| page_detail(ws, p)).unwrap_or_default(),
                    )
                }
            };
            let group = match index {
                0 | 1 => "",
                2 => "Today",
                _ => "Past week",
            };
            SwitcherRow {
                label,
                detail,
                group,
                icon,
                id,
                special,
            }
        })
        .collect()
}

fn build_search_rows(ws: &Workspace, view: &ViewState) -> Vec<SwitcherRow> {
    view.search
        .results
        .iter()
        .filter(|result| {
            ws.page(&result.page_id).is_some_and(|page| {
                page_matches(
                    ws,
                    page,
                    &view.search.query,
                    view.search.title_only,
                    view.search.created_by.as_deref(),
                    view.search.in_page.as_deref(),
                    view.search.has_comments,
                )
            })
        })
        .enumerate()
        .map(|(index, result)| {
            let page = ws.page(&result.page_id);
            SwitcherRow {
                label: result.page_title.clone(),
                detail: page.map(|p| page_detail(ws, p)).unwrap_or_default(),
                group: if index == 0 { "Today" } else { "Past week" },
                icon: page.map(|p| p.icon.clone()).unwrap_or_default(),
                id: result.page_id.clone(),
                special: false,
            }
        })
        .collect()
}

/// Build the shared spec, then hand it to the one switcher renderer (DRY).
#[allow(clippy::too_many_arguments)]
fn switcher_spec<'a>(
    id: &'static str,
    hint: &'a str,
    query: &'a str,
    rows: &'a [SwitcherRow],
    selected: usize,
    mode: SwitcherMode,
    ws: &'a Workspace,
    view: &'a ViewState,
) -> SwitcherSpec<'a> {
    SwitcherSpec {
        id,
        hint,
        query,
        rows,
        selected,
        highlight: view.highlight_pane,
        filters: view.filters,
        title_only: view.search.title_only,
        created_by: view.search.created_by.as_deref(),
        in_page: view.search.in_page.as_deref(),
        has_comments: view.search.has_comments,
        members: view.share_members.as_slice(),
        mode,
        ws,
    }
}

pub(crate) fn render_new_tab(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let choices = new_tab_choices(ws, &view.new_tab_query);
    let rows = build_new_tab_rows(ws, view, &choices);
    let spec = switcher_spec(
        "new_tab_switcher",
        "Open in new tab...",
        &view.new_tab_query,
        &rows,
        view.new_tab_selected,
        SwitcherMode::NewTab,
        ws,
        view,
    );
    render_switcher(ctx, &spec, palette, commands);
}

pub(crate) fn render_search(
    ctx: &egui::Context,
    ws: &Workspace,
    view: &ViewState,
    palette: &Palette,
    commands: &mut Vec<Command>,
) {
    let rows = build_search_rows(ws, view);
    let spec = switcher_spec(
        "quick_search",
        "Search...",
        &view.search.query,
        &rows,
        view.search.selected,
        SwitcherMode::Search,
        ws,
        view,
    );
    render_switcher(ctx, &spec, palette, commands);
}
