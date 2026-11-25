# spec-147 - Viewport-limited sidebar tree

- Status: implemented (2026-11-25)
- Related: `spec-119`, `docs/performance-roadmap.md`
- Review source: 2026-11-25 ChatGPT latency review, item 3.2

## Purpose and scope

The sidebar recursed over every expanded page on every frame, including rows far
outside the visible region. It also called `Workspace::children` for every page
to test for children.

### Flat rows

`ui::visible_tree_rows(workspace, expanded)` walks the non-archived, expanded
tree once and returns `TreeRow { page_id, depth, has_children, open }` in display
order. The renderer consumes this list.

### Rendering

`render_tree` allocates each row's height but only paints and interacts with
rows that intersect `ui.clip_rect()`:

- rows above the viewport are allocated and skipped;
- rows below it are allocated as one remaining block so the scroll range stays
  correct;
- only visible rows create click/hover responses.

This is manual virtualization inside the existing outer scroll area, so there is
no nested `ScrollArea`.

## Acceptance criteria

P1. **Given** a collapsed tree, **then** child rows are absent from the flat list.

P2. **Given** an expanded parent, **then** children appear at `depth + 1` and the
parent reports `has_children`.

P3. **Given** an archived page, **then** it is absent from the list.

P4. **Given** a large tree scrolled to the middle, **then** only intersecting rows
are painted while the scroll range covers all rows.

## Test scenarios

- `tests/ui.rs`
  (`sidebar_tree_rows_follow_expansion_and_skip_archived_pages`).
- Existing sidebar/navigation suites and the screenshot check.

## Plan / tasks

- [x] Add the pure flattening function.
- [x] Render only rows intersecting the clip rect.
- [x] Keep the scroll range for skipped rows.
- [x] Add the test.
