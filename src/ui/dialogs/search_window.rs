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

fn build_new_tab_rows(ws: &Workspace, choices: &[NewTabChoice]) -> Vec<SwitcherRow> {
    choices
        .iter()
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
    view.search_results
        .iter()
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
        mode,
        ws,
        ai_query: &view.ai_query,
        ai_status: &view.ai_status,
        ai_answer: view.ai_answer.as_deref(),
        ai_error: view.ai_error.as_deref(),
        ai_busy: view.ai_busy,
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
    let rows = build_new_tab_rows(ws, &choices);
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
        "Search or ask a question...",
        &view.search_query,
        &rows,
        view.search_selected,
        SwitcherMode::Search,
        ws,
        view,
    );
    render_switcher(ctx, &spec, palette, commands);
}
