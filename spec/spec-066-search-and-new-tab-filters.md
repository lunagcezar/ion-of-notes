# spec-066 - Search and new-tab filters

- Status: implemented (2026-11-12)
- Related: `spec-013`, `spec-015`, `spec-017`, `spec-060`
- Reference: `reference/filter-1.png`, `reference/filter-2.png`,
  `reference/filter-3.png`

## Purpose and scope

The search (`Ctrl+K`) and new-tab windows show a filter bar with static labels
only. This spec makes the filters real and removes the AI strip from both
windows until the AI feature is implemented properly.

Filters:

- **Title only** — match only the page title, not the body snippet.
- **Created by** — choose a member.
- **In** — choose a page; include that page and its descendants.
- **+ Filter** — add a Has comments filter (and remove it).
- **Show/hide filters** — toggle the whole filter bar, with a real glyph and
  tooltip.

## Evidence

`filter-1.png` shows the filter bar and the Created by people popover.
`filter-2.png` shows the In pages popover. `filter-3.png` shows the
"Show filters" tooltip and the bottom-right filter toggle. None of the pictures
show an Ask AI strip inside the switcher.

Current seams:

| Area | Code |
| ---- | ---- |
| Shared switcher | `src/ui/switcher.rs` |
| Search/new-tab rows | `src/ui/dialogs/search_window.rs` |
| Search commands/state | `src/ui/commands.rs`, `src/ui/apply/navigation.rs` |
| Page model | `src/model/page.rs` |

## Data model

Add an additive `created_by: String` to `Page` with
`#[serde(default = "default_creator")]`, defaulting to `"Workspace Owner"`.
Seed pages and new pages use that default until accounts assign a real author.

## Acceptance criteria

B15. **Given** the filter bar, **when** it is shown, **then** it contains
Title only, Created by, In and + Filter.

B16. **Given** Title only, **when** it is enabled, **then** results whose title
does not contain the query are hidden.

B17. **Given** Created by, **when** a member is chosen, **then** only pages
whose `created_by` matches are shown; clearing restores all.

B18. **Given** In, **when** a page is chosen, **then** only that page and its
descendants are shown; clearing restores all.

B19. **Given** + Filter, **when** it is enabled, **then** a Has comments filter
is applied and can be removed from the chip.

B20. **Given** the show/hide button, **when** it is clicked, **then** the bar
toggles, the button uses a visible filter glyph and has a "Show filters" /
"Hide filters" tooltip.

B21. **Given** the search/new-tab window, **when** it renders, **then** there is
no Ask AI strip, button, status, answer or error.

B22. **Given** a filter is active, **when** the query changes, **then** the
filters remain active and the result count updates.

## Test scenarios

- Pure `filter_rows` cases: title-only, created-by, scope (page + descendants),
  has-comments, combined filters, clear each.
- Search query change does not call `SetAiQuery`.
- `SwitcherSpec` has no AI fields and the switcher renders no AI strip.
- Screenshots: `filter-title`, `filter-created`, `filter-in`, `filter-hidden`,
  `filter-combined`.
- Full gate.

## Design

- Add filter fields to app state and `ViewState`:
  `search_title_only`, `search_created_by: Option<String>`,
  `search_in: Option<String>`, `search_has_comments`, and the existing
  `filters` visibility flag.
- Add a pure `filter_rows(rows, ws, filters)` helper in `search_window.rs`.
  Rows carry the page id, title, snippet, creator and comment count so the
  predicate is testable without egui.
- Use compact dropdowns for Created by and In (member list and page list),
  a toggle chip for Title only, and a chip for Has comments. Keep the layout
  close to `filter-1`/`filter-2`.
- Replace the corrupted filter glyph with `draw_sliders` and keep the bottom
  filter toggle.
- Remove `ai_query`, `ai_status`, `ai_answer`, `ai_error` and `ai_busy` from
  `SwitcherSpec`; remove the AI strip and the `SetAiQuery` side effect.
