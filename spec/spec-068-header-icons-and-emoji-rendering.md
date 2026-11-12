# spec-068 - Header alignment, icon fallback and emoji rendering

- Status: implemented (2026-11-12)
- Related: `spec-057`, `spec-060`, `spec-063`, `spec-065`
- Reference: `reference/app-bar.png`, `reference/share.png`

## Purpose and scope

Three visual regressions:

1. The breadcrumb row was moved horizontally to the page content column. The
   request was vertical alignment with the breadcrumb container, not a
   horizontal move. The share/favorite/ellipsis controls must share the same
   vertical center as the crumbs.
2. Pages created with an empty icon are stored with `"P"`, so the title-letter
   fallback never runs. Untitled/aaaa pages show `P` instead of `U`/`A`.
3. Emoji glyphs are still low in their icon wells and the picker cells. The
   emoji search/scroll still feels slow.

## Acceptance criteria

H1. **Given** the breadcrumb row, **when** it renders, **then** the crumbs and
the Share/favorite/ellipsis controls are vertically centered on the same row,
and the row starts at the content-pane padding (not the page column).

H2. **Given** a page created with an empty icon, **when** it renders, **then**
the icon is the uppercase first title letter (`U` for Untitled, `A` for aaaa),
not `P`.

H3. **Given** an emoji page icon or an emoji cell, **when** it renders, **then**
the glyph ink is vertically centered in its well/cell.

H4. **Given** the emoji picker, **when** the pointer scrolls or the filter
changes, **then** only visible rows are built, tooltips do not allocate for
every cell, and the filtered index list is cached per query.

## Test scenarios

- `page_icon_letter` already covers the fallback; add a server create-page test
  proving an empty draft icon stays empty.
- Screenshots: `header-row`, `emoji-grid`, `icon-fallback-untitled`.
- Full gate.

## Design

- Revert the breadcrumb horizontal padding to the content-pane value and give
  the row a fixed height with centered layout for both crumbs and controls.
- Remove the server-side `"P"` default; `Page::new` and the title-letter
  fallback own the display default.
- Center text glyphs with the font row height plus a small optical lift; use
  the same helper for page icons and emoji cells.
- Keep the virtualized picker and cached indices; remove per-cell
  `on_hover_text` and avoid rebuilding display strings inside the visible-row
  loop.
