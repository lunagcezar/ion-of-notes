# spec-063 - Sidebar menus, breadcrumbs and gutter alignment

- Status: implemented (2026-11-12)
- Related: `spec-013`, `spec-059`, `spec-062`
- Reference: `reference/app-menu.png`, `reference/customize-sidebar.png`

## Purpose and scope

Four alignment defects around the sidebar and the editor gutter:

1. The workspace switcher opens far above the account row.
2. The four-point menu opens far from its control.
3. The breadcrumb row is not aligned with the page container.
4. The block plus/six-dot gutter controls are not vertically centered on the
   text line.
5. The app menu only shows a section chevron for the active section.

## Evidence

The review after specs 058-062 reported each item directly. `app-menu.png`
shows every major section with a right chevron in its idle state. The
customize-sidebar pictures show the account row and four-point control at the
bottom of the sidebar.

Current seams:

| Area | Code |
| ---- | ---- |
| Workspace/grid menus | `src/ui/dialogs/menus.rs` |
| Sidebar bottom group | `src/ui/render/sidebar.rs` |
| Breadcrumb row | `src/ui/render/content/page.rs` |
| Block gutter | `src/ui/render/content/blocks.rs` |
| App menu sections | `src/ui/dialogs/menus.rs` |

## Acceptance criteria

B1. **Given** the workspace switcher, **when** it opens, **then** its bottom
edge is a few points above the account row, not hundreds of points above it.

B2. **Given** the four-point menu, **when** it opens, **then** it is anchored
above the four-point control with the same small gap and does not overlap the
account row.

B3. **Given** the breadcrumb row, **when** it renders, **then** its leading
glyph/text use the same content padding as the page header and the row is
vertically centered in its strip.

B4. **Given** a block line, **when** the gutter renders, **then** the plus and
six-dot controls are vertically centered on the first text line, including
headings and multi-line blocks.

B5. **Given** the app menu, **when** it is open, **then** every section shows a
right chevron in both idle and hovered states.

## Test scenarios

- Pure menu-anchor helper returns a bottom edge inside the sidebar bottom
  group for both menus.
- A gutter-centering helper returns the same center as the first text line for
  paragraph and heading font sizes.
- Screenshots: `menus-bottom`, `app-menu-idle`, `breadcrumb-row`,
  `block-gutter`.
- Full gate.

## Design

- Add `bottom_menu_pos(screen, height, gap)` and use it for both menus; anchor
  the grid menu to the right edge of the four-point control.
- Give the breadcrumb row an explicit horizontal padding constant shared with
  the page header and center it vertically.
- Center the gutter controls using the first-line height rather than a fixed
  22-point top alignment.
- Render `›` for every app-menu section; keep the active section highlighted.
