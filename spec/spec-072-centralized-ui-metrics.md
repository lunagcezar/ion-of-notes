# spec-072 - Centralized UI metrics

- Status: implemented (2026-11-12)
- Related: `spec-013`, `spec-049`, `spec-061`, `spec-069`
- Reference: `reference/picture-1.png` .. `reference/picture-98.png`,
  `reference/app-bar.png`, `reference/customize-sidebar.png`

## Purpose and scope

Layout values (margins, padding, sizes, radii, font sizes and section gaps)
are scattered as numeric literals across the UI modules. The same values are
repeated in the main sidebar and the settings navigation rail, which makes the
two surfaces drift apart and makes a spacing change a search-and-replace task.

This spec centralizes the structural UI metrics in one constants-only module,
gives shared values one semantic name, and rounds layout values to even
numbers so the arithmetic stays predictable.

Scope:

- one `src/ui/constants.rs` module containing only `const` declarations and
  documentation (no functions, types or state);
- a base spacing scale plus semantic tokens for the shell, sidebar, settings,
  content page, blocks, popovers and shared rows;
- the main sidebar and the settings rail share
  `SIDEBAR_CONTENT_PADDING`, `ROW_HEIGHT`, `ROW_RADIUS`, `ICON_SIZE` and
  `SECTION_GAP`;
- replace the structural literals in the shell, sidebar, settings, content,
  blocks, menus, shared widgets and the settings pages;
- round layout values to even numbers (hairline strokes are the documented
  exception).

Out of scope:

- palette colors and icon artwork coordinates;
- data-driven dimensions (page column width, table column widths, text
  measurements);
- changing the visual design beyond the even-number rounding.

## Acceptance criteria

P1. **Given** the UI source, **when** a layout metric is needed, **then** it
is available as a named constant in `src/ui/constants.rs` and no UI module
declares its own spacing/size constant.

P2. **Given** the constants module, **when** it is inspected, **then** it
contains only documentation and `const` declarations; every numeric spacing,
size, padding or radius value is even, and every font value is even except the
documented `FONT_SMALL` 13 px token; `STROKE_HAIRLINE` is the other documented
odd value.

P3. **Given** the main sidebar and the settings rail, **when** their content
insets are rendered, **then** both use the same `SIDEBAR_CONTENT_PADDING`
token (and the shared row/icon/section tokens where they match).

P4. **Given** the shell, **when** the tab strip, sidebar, settings dialog,
breadcrumb, blocks or popovers are laid out, **then** their structural values
come from the constants module rather than local literals.

P5. **Given** the full test gate, **when** it runs, **then** the existing
behavior is preserved, all tests pass, and fresh screenshots show no layout
regression.

## Test scenarios

- `ui_constants_are_declaration_only_and_even`: the constants source contains
  no `fn`, `struct`, `enum` or `impl`; every numeric `f32`/`u8` layout value is
  even except `STROKE_HAIRLINE` and the 13 px `FONT_SMALL` token; required
  semantic names exist.
- `shared_sidebar_metrics_are_used_by_both_sidebars`: the main sidebar and the
  settings dialog both reference `SIDEBAR_CONTENT_PADDING`.
- Existing `tests/ui.rs`, `tests/fidelity.rs` and desktop tests stay green.
- `--screenshot` states for the default window, settings, customize sidebar
  and selection are compared with the reference pictures.

## Design

- `src/ui/constants.rs` is a flat module grouped by comments. Names use
  `SCREAMING_SNAKE_CASE` (Rust API Guidelines / RFC 430); values are logical
  points; radii are `u8` because egui's `CornerRadius` takes `u8`.
- Best-practice choices: one private module instead of a global constant
  namespace, semantic tokens aliasing a small base scale (design-token
  practice), and no behavior in the constants file.
- Base scale: `SPACE_2` .. `SPACE_120` (even). Semantic tokens alias the scale
  where a component needs a name (`SIDEBAR_CONTENT_PADDING = SPACE_16`).
- Shared values are declared once and reused by the main sidebar and the
  settings rail instead of being redefined.
- Font sizes move to `FONT_MICRO`, `FONT_CAPTION`, `FONT_SMALL`, `FONT_BODY`,
  `FONT_ROW`, `FONT_SUBTITLE`, `FONT_TITLE`, `FONT_PAGE_TITLE` and
  `FONT_CLOSE`. `FONT_SMALL = 13.0` is the documented odd font token because
  13 px is a common compact label size and font sizes are not layout
  arithmetic; `FONT_ROW` aliases it.
- `STROKE_HAIRLINE = 1.0` is the only odd layout value: borders must stay
  crisp at 1x. `STROKE_EMPHASIS = 2.0` covers selected/drag outlines.
- The module is private to `ui`; no public API is added.

## Plan / tasks

- [x] Write spec-072 and the failing constants policy test (red)
- [x] Add `src/ui/constants.rs` and re-export the module from `ui.rs`
- [x] Replace shell metrics in `render/chrome.rs`
- [x] Replace sidebar and settings metrics; share `SIDEBAR_CONTENT_PADDING`
- [x] Replace content, block, menu, widget, switcher and settings-page metrics
- [x] Round layout values to even numbers and run fmt/clippy/tests
- [x] Update `CHANGELOG.md`, `AGENTS.md` and `docs/architecture.md`
- [x] Screenshot check and commit as spec-072
