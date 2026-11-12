# spec-080 - Sidebar rendering decomposition

- Status: implemented (2026-11-12)
- Related: `spec-062`, `spec-072`
- Review sources: Claude and ChatGPT (oversized UI functions)

## Purpose and scope

`render_sidebar` had grown into a large function that combined the search
box, tab strip, customize mode, default list, section rendering and the fixed
bottom group. The pieces were already recognizable but not separated.

This spec extracts three cohesive functions while preserving the exact
behavior and layout.

Scope:

- `render_sidebar` becomes a small shell: search, scroll area, mode selection,
  bottom group;
- `render_sidebar_customize` owns the customize rows, tabs, drag/drop and
  add-shortcut/section controls;
- `render_sidebar_default` owns the Library/Settings rows and the favorite,
  private and custom sections;
- `render_sidebar_bottom` owns the account row, invite card and customize/done
  controls.

Out of scope:

- changing the sidebar design or constants;
- grouping `NotionApp` state;
- splitting `render_block` or `render_switcher`.

## Acceptance criteria

P1. **Given** the default sidebar, **when** it renders, **then** the Library,
Settings, favorites, private tree and custom sections are unchanged.

P2. **Given** customize mode, **when** it renders, **then** the tab strip,
rows, drag/drop targets and add controls are unchanged.

P3. **Given** the bottom group, **when** it renders, **then** the account row,
invite card and customize/done controls are unchanged.

P4. **Given** the full desktop test suite and fresh screenshots, **when** the
refactor is applied, **then** there is no visual or behavioral regression.

## Test scenarios

- `tests/ui.rs` (constants, palette, settings and sidebar helpers)
- `tests/fidelity.rs`
- `--screenshot` for the default and customize-sidebar states
- Full quality gate

## Design

- Free functions with an explicit `#[allow(clippy::too_many_arguments)]` keep
  the change mechanical and avoid a new state object.
- The functions share the existing module-level helpers (`nav_row`,
  `customize_row_actions`, `render_tree`, `section_label`).
- No command, constant or layout value changes.

## Plan / tasks

- [x] Write spec-080
- [x] Extract the three functions
- [x] Run the desktop tests and compare screenshots
- [x] Run the full gate and commit
