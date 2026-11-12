# UI guidelines - alignment, spacing and design constants

This document is the contributor and agent reference for the desktop UI
layout. It explains how the interface is aligned, how margins and padding are
chosen, and how to use, apply and create the design constants.

The source of truth for the values themselves is `src/ui/constants.rs`; this
document explains the rules around them.

## 1. Sources of truth

| Concern | Source | Rule |
| ------- | ------ | ---- |
| Layout metrics (spacing, sizes, padding, radii, font sizes) | `src/ui/constants.rs` | Use a named constant; never hardcode a layout number in a UI module. |
| Colors, text styles and theming | `src/ui/palette.rs` | Use `Palette`; colors do not belong in the constants file. |
| Icon artwork and path coordinates | `src/ui/icons.rs` | Artwork coordinates stay with the drawing functions. |
| Shared rows, popovers and labels | `src/ui/widgets.rs` | Reuse `menu_button`, `popover_frame`, `popover_frame_compact`, `section_label`. |
| Visual evidence | `reference/` and `screenshots/` | Compare with the pictures when a surface changes. |

The constants module is private to `ui`. It contains only documentation and
`const` declarations: no functions, types or state.

## 2. The design constants

### 2.1 Base scale

`src/ui/constants.rs` starts with an even spacing scale:

```text
SPACE_2, SPACE_4, SPACE_6, SPACE_8, SPACE_10, SPACE_12, SPACE_14, SPACE_16,
SPACE_18, SPACE_20, SPACE_22, SPACE_24, SPACE_26, SPACE_28, SPACE_30, SPACE_32,
SPACE_34, SPACE_36, SPACE_38, SPACE_40, SPACE_42, SPACE_44, SPACE_46, SPACE_48,
SPACE_52, SPACE_60, SPACE_64, SPACE_70, SPACE_72, SPACE_76, SPACE_80, SPACE_88,
SPACE_96, SPACE_120
```

Use a base token for a one-off gap or size. Use a semantic token when the value
names a component, repeats across surfaces, or would otherwise be guessed.

### 2.2 Semantic tokens

Semantic tokens alias the scale or another token:

```rust
pub(crate) const SIDEBAR_CONTENT_PADDING: f32 = SPACE_16;
pub(crate) const ROW_HEIGHT: f32 = SPACE_30;
pub(crate) const ROW_RADIUS: u8 = 6;
pub(crate) const ICON_SIZE: f32 = SPACE_16;
pub(crate) const SECTION_GAP: f32 = SPACE_18;
pub(crate) const MENU_ROW_HEIGHT: f32 = ROW_HEIGHT;
pub(crate) const SETTINGS_RAIL_PADDING_Y: f32 = SIDEBAR_CONTENT_PADDING;
pub(crate) const FONT_ROW: f32 = FONT_SMALL;
```

Shared values are declared once. In particular, the main sidebar and the
settings rail use the same `SIDEBAR_CONTENT_PADDING`, `ROW_HEIGHT`,
`ROW_RADIUS`, `ICON_SIZE` and `SECTION_GAP`.

| Shared token | Value | Used by |
| ------------ | ----- | ------- |
| `SIDEBAR_CONTENT_PADDING` | 16 | Main sidebar content and settings rail padding |
| `ROW_HEIGHT` | 30 | Sidebar rows, settings navigation rows, menu rows |
| `ROW_RADIUS` | 6 | Sidebar and settings navigation rows |
| `ICON_SIZE` | 16 | Sidebar and settings row glyphs |
| `SECTION_GAP` | 18 | Sidebar sections and settings navigation groups |
| `SECTION_LABEL_HEIGHT` | 24 | Sidebar and settings section labels |
| `FONT_ROW` | 13 | Sidebar, settings and menu row labels |
| `MENU_ROW_HEIGHT` | 30 | Menu rows |
| `SETTINGS_SEARCH_HEIGHT` | 30 | Settings rail search field |

### 2.3 Even numbers

Layout values are even so that half-values and nested arithmetic stay
predictable. The two documented exceptions are:

| Constant | Value | Why |
| -------- | ----- | --- |
| `FONT_SMALL` | `13.0` | 13 px is a common compact label size; font sizes are not layout arithmetic. `FONT_ROW` aliases it. |
| `STROKE_HAIRLINE` | `1.0` | Borders must stay crisp at 1x. `STROKE_EMPHASIS` is `2.0`. |

Ratios use a percent integer rather than a fractional constant, for example
`SWITCHER_LIST_PERCENT = 56.0` with `(value / 100.0)` at the call site.

### 2.4 How to use a constant

Prefer the semantic name. Import the module with a short alias when a file uses
several values:

```rust
use crate::ui::constants as metrics;

ui.add_space(metrics::SPACE_12);
let row = Rect::from_min_size(pos, Vec2::new(width, metrics::ROW_HEIGHT));
ui.painter().text(
    row.min + Vec2::new(metrics::ROW_LABEL_INSET, metrics::ROW_HEIGHT / 2.0),
    Align2::LEFT_CENTER,
    label,
    FontId::proportional(metrics::FONT_ROW),
    palette.text,
);
```

For one or two values, an explicit import is fine:

```rust
use crate::ui::constants::{BREADCRUMB_HEIGHT, SPACE_6};
```

`egui::Margin` and `egui::CornerRadius` take small integer types, so cast at
the call site:

```rust
Margin::same(metrics::SPACE_12 as i8)
CornerRadius::same(metrics::ROW_RADIUS)
```

### 2.5 How to create a constant

1. **Decide the category.** Base scale, shell, sidebar/settings, content page,
   blocks, popovers/menus, fonts or strokes.
2. **Choose an even value.** Prefer a base-scale value. If the value is
   component-specific, name it after the component.
3. **Name it in `SCREAMING_SNAKE_CASE`.** Prefer a semantic name
   (`SETTINGS_SEARCH_WIDTH`) over a number name (`WIDTH_268`).
4. **Add a doc comment** when the name is not self-explanatory or the token is
   shared.
5. **Use it at the call site.** Remove the old literal.
6. **Run the policy tests:**
   `cargo test --features desktop --test ui ui_constants`.
7. **Update this document** if the token is shared or changes a surface rule.

Do not add a constant for:

- colors (use `Palette`);
- icon path coordinates (use `icons.rs`);
- text content;
- a value that is genuinely data-driven (a table column ratio, a measured text
  width, a user-provided size).

### 2.6 Policy tests

`tests/ui.rs` contains two guards:

- `ui_constants_are_declaration_only_and_even` parses
  `src/ui/constants.rs`, rejects functions/types/impls, requires the shared
  semantic names, and rejects odd `f32`/`u8` values except `FONT_SMALL` and
  `STROKE_HAIRLINE`.
- `shared_sidebar_metrics_are_used_by_both_sidebars` proves the main sidebar
  and settings dialog both reference `SIDEBAR_CONTENT_PADDING`.

## 3. Alignment rules

### 3.1 Vertical centering

- Use a fixed-height container and `Layout::left_to_right(Align::Center)` for
  a row.
- When painting manually, draw text at `rect.center().y` with
  `Align2::LEFT_CENTER`, or at `rect.min + Vec2::new(inset, ROW_HEIGHT / 2.0)`.
- Do not use `horizontal_centered` for a full-height row: it allocates the
  whole available height and makes the strip taller than intended.
- Placeholder text is centered on the input box center, not on a guessed
  baseline.

### 3.2 Horizontal insets

| Surface | Icon inset | Label inset | Notes |
| ------- | ---------- | ----------- | ----- |
| Main sidebar row | `ROW_ICON_INSET` (10) | `ROW_LABEL_INSET` (36) | Favorites, private tree, custom shortcuts. |
| Settings navigation row | `SETTINGS_NAV_ICON_INSET` (12) | `SETTINGS_NAV_LABEL_INSET` (38) | Rail only. |
| Section label | `SPACE_12` (12) | `SPACE_12` (12) | 24 px high; the main sidebar uses the vertical center, the shared dialog label uses a 14 px optical center. |
| Sidebar search | `SPACE_10` (10) | `SPACE_34` (34) | 34 px high. |
| Settings search | `SETTINGS_SEARCH_ICON_INSET` (8) | `SETTINGS_SEARCH_TEXT_INSET` (28) | 30 px high; width is rail minus 2 × padding. |

Keep the icon and label insets of a surface consistent. Do not mix the main
sidebar insets into the settings rail.

### 3.3 Right-aligned controls

Use `Layout::right_to_left(Align::Center)` for the trailing controls of a row
or breadcrumb. Keep the item spacing at `SPACE_4` or `SPACE_6` and reserve the
right padding explicitly with `ui.add_space(...)`.

### 3.4 Popovers and menus

- Anchor a popover to the actual control rect when one is available (the
  workspace switcher and four-point menu read their button rects from `ctx`
  temp data).
- Use `pivot(Align2::LEFT_BOTTOM)` and `fixed_pos(anchor.left(), anchor.top())`;
  `POPOVER_GAP` documents the zero gap.
- The app menu and the customize-sidebar add menus sit in fixed chrome, so
  they use the documented `APP_MENU_*` and `SIDEBAR_MENU_*` offsets instead of
  a row rect.
- Use `popover_frame` for roomy menus and `popover_frame_compact` for context
  popovers.
- Menu rows use `MENU_ROW_HEIGHT` (30), `MENU_ROW_RADIUS` (6), `MENU_TEXT_SIZE`
  (14) and `MENU_HINT_SIZE` (12).

## 4. Surface metrics

### 4.1 Shell and tab strip

| Element | Token | Value |
| ------- | ----- | ----- |
| Title bar height | `TITLE_BAR_HEIGHT` | 36 |
| Sidebar width | `SIDEBAR_WIDTH` | 266 |
| Compact app bar | `APP_BAR_COMPACT_WIDTH` | 92 |
| Title button | `TITLE_BUTTON_SIZE` | 26 |
| Window control width | `WINDOW_CONTROL_WIDTH` | 38 |
| Tab | `TAB_WIDTH` × `TAB_HEIGHT` | 190 × 28 |
| Tab radius | `TAB_RADIUS` | 6 |
| Tab icon | `TAB_ICON_INSET`, `TAB_ICON_Y` | 10, 6 |
| Tab label inset | `TAB_LABEL_INSET` | 34 |
| Tab close | `TAB_CLOSE_SIZE`, `TAB_CLOSE_INSET` | 18, 14 |
| Tab gap | `TAB_GAP` | 6 |

### 4.2 Main sidebar

| Element | Token | Value |
| ------- | ----- | ----- |
| Content padding | `SIDEBAR_CONTENT_PADDING` | 16 |
| Search height | `SIDEBAR_SEARCH_HEIGHT` | 34 |
| Row height | `ROW_HEIGHT` | 30 |
| Row radius | `ROW_RADIUS` | 6 |
| Icon size | `ICON_SIZE` | 16 |
| Section gap | `SECTION_GAP` | 18 |
| Section label height | `SECTION_LABEL_HEIGHT` | 24 |
| Account row | `SIDEBAR_ACCOUNT_HEIGHT` | 40 |
| Invite card | `SIDEBAR_INVITE_HEIGHT` | 76 |
| Done button | `SIDEBAR_DONE_HEIGHT` | 32 |
| Customize row | `SIDEBAR_CUSTOMIZE_HEIGHT` | 28 |
| Shortcut row | `SIDEBAR_SHORTCUT_HEIGHT` | 28 |
| Bottom reserve | `SIDEBAR_BOTTOM_RESERVE` | 176 |

### 4.3 Settings dialog

| Element | Token | Value |
| ------- | ----- | ----- |
| Rail width | `SETTINGS_RAIL_WIDTH` | 300 |
| Dialog inset | `DIALOG_INSET` | 40 |
| Dialog radius | `DIALOG_RADIUS` | 10 |
| Rail padding | `SETTINGS_RAIL_PADDING_Y` | 16 |
| Rail search | `SETTINGS_SEARCH_WIDTH` × `SETTINGS_SEARCH_HEIGHT` | 268 × 30 |
| Content padding | `SETTINGS_CONTENT_PADDING` | 24 |
| Content minimum width | `SETTINGS_CONTENT_MIN_WIDTH` | 320 |
| Close control | `SETTINGS_CLOSE_SIZE` | 28 |
| Close inset | `SETTINGS_PADDING` + `SETTINGS_CLOSE_INSET` | 20 + 42 |
| Dialog min/max width | `SETTINGS_MIN_WIDTH`, `SETTINGS_MAX_WIDTH` | 720, 1200 |
| Dialog min/max height | `SETTINGS_MIN_HEIGHT`, `SETTINGS_MAX_HEIGHT` | 420, 720 |

The rail background spans the full dialog height and uses the dialog radius on
its left corners. The scrollbar sits on the rail border; the content pane's
scrollbar sits on the dialog border. Do not add an extra right margin for a
scrollbar.

### 4.4 Breadcrumb and content page

| Element | Token | Value |
| ------- | ----- | ----- |
| Breadcrumb height | `BREADCRUMB_HEIGHT` | 28 |
| Breadcrumb padding | `BREADCRUMB_PADDING` | 16 |
| Breadcrumb gap | `BREADCRUMB_GAP` | 6 |
| Breadcrumb icon | `BREADCRUMB_ICON_SIZE` | 16 |
| Breadcrumb label inset | `BREADCRUMB_LABEL_INSET` | 24 |
| Breadcrumb action | `BREADCRUMB_ACTION_SIZE` × `BREADCRUMB_ACTION_HEIGHT` | 26 × 24 |
| Side margin | `CONTENT_SIDE_MARGIN` | 96 |
| Column min/max | `CONTENT_COLUMN_MIN`, `CONTENT_COLUMN_MAX` | 360, 708 |
| Gutter | `CONTENT_GUTTER` | 48 |
| Top/bottom space | `CONTENT_TOP_SPACE`, `CONTENT_BOTTOM_SPACE` | 88, 80 |
| Page icon | `PAGE_ICON_SIZE` | 72 |
| Page title | `FONT_PAGE_TITLE` | 44 |
| Title gap | `PAGE_TITLE_GAP` | 30 |

### 4.5 Blocks

| Element | Token | Value |
| ------- | ----- | ----- |
| Row height | `BLOCK_ROW_HEIGHT` | 30 |
| Control | `BLOCK_CONTROL_WIDTH` × `BLOCK_CONTROL_HEIGHT` | 22 × 22 |
| Control gap | `BLOCK_CONTROL_GAP` | 4 |
| Block gap | `BLOCK_GAP` | 14 |
| Heading gap | `BLOCK_HEADING_GAP` | 22 |
| Body/small text | `BLOCK_BODY_SIZE`, `BLOCK_SMALL_SIZE` | 16, 14 |
| Callout/code padding | `BLOCK_CALLOUT_PADDING`, `BLOCK_CODE_PADDING` | 12, 12 |
| Divider height | `BLOCK_DIVIDER_HEIGHT` | 14 |

### 4.6 Fonts and strokes

| Token | Value | Use |
| ----- | ----- | --- |
| `FONT_MICRO` | 10 | Tiny hints and badges. |
| `FONT_CAPTION` | 12 | Section labels, secondary text. |
| `FONT_SMALL` / `FONT_ROW` | 13 | Compact row labels. |
| `FONT_BODY` | 14 | Body and menu labels. |
| `FONT_SUBTITLE` | 18 | Dialog subtitles. |
| `FONT_TITLE` | 28 | Settings/page section titles. |
| `FONT_PAGE_TITLE` | 44 | Page title. |
| `STROKE_HAIRLINE` | 1 | Borders and separators. |
| `STROKE_EMPHASIS` | 2 | Selected/drag outlines. |

## 5. Checklist for a UI change

- [ ] Is every new layout value a named constant from
      `src/ui/constants.rs`?
- [ ] Does a shared value already exist (`ROW_HEIGHT`, `SIDEBAR_CONTENT_PADDING`,
      `SECTION_GAP`, `ICON_SIZE`, `FONT_ROW`)?
- [ ] Is the value even, or is it a documented `FONT_SMALL`/`STROKE_HAIRLINE`
      exception?
- [ ] Is the row vertically centered with `Align2::LEFT_CENTER` or
      `ROW_HEIGHT / 2.0`?
- [ ] Are the icon and label insets correct for this surface?
- [ ] Is the popover anchored to the control rect?
- [ ] Does the scrollbar sit on the container border without an extra margin?
- [ ] Did `cargo test --features desktop --test ui ui_constants` pass?
- [ ] Did `cargo fmt`, clippy and the full quality gate pass?
- [ ] Did you compare a fresh `--screenshot` with the reference pictures?

## 6. Related documents

- `spec/spec-072-centralized-ui-metrics.md` - the spec that introduced the
  constants module.
- `docs/architecture.md` - where `ui/constants.rs` sits in the module tree.
- `docs/requirements.md` - NFR-06 (colors, fonts and layout metrics).
- `src/ui/constants.rs` - the executable source of truth.
- `src/ui/palette.rs` - colors, text styles and theming.
- `src/ui/widgets.rs` - shared row, popover and label primitives.
