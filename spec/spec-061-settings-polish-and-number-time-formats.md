# spec-061 - Settings polish and number/time formats

- Status: implemented (2026-11-11)
- Related: `spec-013`, `spec-015`, `spec-017`, `spec-057`
- Reference: `reference/picture-15.png` .. `reference/picture-29.png`
  (preferences window), `reference/share.png` (dialog spacing)
- Contract docs: `docs/data-model.md`, `docs/requirements.md`

## Purpose and scope

The settings window still has four visible defects and two preferences that do
nothing:

1. The close "×" sits on the right border instead of inside the dialog padding.
2. The navigation rail has no vertical (top/bottom) padding.
3. The content scrollbar sits against the right border instead of inside a
   margin.
4. The number-format and time-format rows show values but do not change any
   displayed number or timestamp.

This spec makes the dialog spacing explicit and implements the two formats as
persisted workspace settings applied through one formatting helper.

Out of scope:

- A new settings window design or a different rail.
- Localizing every string; only number and time formatting are added.
- A calendar/date-picker widget.

## Evidence

The review after `spec-057` reported:

1. "The x icon from the settings dialog is still next to the right border."
2. "The settings dialog sidebar doesn't have [padding on the] y axis."
3. "The overflow scrollbar for the settings content in its dialog must be in
   the right border, not close of the border (with margin)."
4. "In settings > number format and time format doesn't work."

Current code seams:

| Area | Code |
| ---- | ---- |
| Settings dialog and rail | `src/ui/dialogs/settings.rs` |
| Preferences rows | `src/ui/dialogs/settings_pages/preferences.rs` |
| Settings model | `src/model/settings.rs` |
| Library and version dates | `src/ui/render/library.rs`, `src/ui/dialogs/page_actions.rs` |
| Offline dates and comments | `src/ui/dialogs/settings_pages/offline.rs`, `src/ui/render/content/page.rs` |
| Settings search | `src/ui/settings_search.rs` |

## Settings model

Add two persisted fields to `WorkspaceSettings`:

```text
number_format: NumberFormat   // Automatic | DotThousands | CommaThousands
time_format: TimeFormat       // Hour24 | Hour12
```

Defaults are `Automatic` and `Hour24` so existing workspaces keep their current
appearance. Both are serialized with `#[serde(default)]` and included in the
workspace diff, so a change uses the normal full-replace persistence path.

## Acceptance criteria

### Dialog spacing

S1. **Given** the settings dialog, **when** it renders, **then** the close "×"
is inside the dialog's top-right padding (at least 16 points from both edges),
with its hover well fully inside the rounded frame.

S2. **Given** the settings rail, **when** it renders, **then** the search field
and the first section label start at the same top padding, and the last row has
the same bottom padding; the left and right rail padding stay equal.

S3. **Given** the content pane, **when** it overflows, **then** the scrollbar
track is inset from the dialog's right border by a visible margin and does not
touch or overlap the border.

S4. **Given** the dialog is resized or the content is short, **when** the
scrollbar is hidden, **then** the content width does not jump and the right
margin stays stable.

### Number format

S5. **Given** the Number format row, **when** a format is chosen, **then**
numeric table cells, counts and numeric summaries render with the chosen
thousands/decimal separators.

S6. **Given** `Automatic`, **when** the app locale changes, **then** the number
format follows the locale default.

S7. **Given** `DotThousands` or `CommaThousands`, **when** the app restarts,
**then** the choice is restored and the displayed numbers match it.

### Time format

S8. **Given** the Time format row, **when** `12-hour` is chosen, **then** every
displayed timestamp shows a 12-hour clock with AM/PM.

S9. **Given** `24-hour`, **when** a timestamp renders, **then** it shows a
24-hour clock without AM/PM.

S10. **Given** a timestamp that cannot be parsed, **when** it renders, **then**
the original string is shown unchanged (no panic, no empty cell).

S11. **Given** either format, **when** Settings is reopened or the app restarts,
**then** the selected value is shown and applied.

### Informational rows

S12. **Given** a settings row that is intentionally read-only (high contrast,
Enter-to-add-line, sync, sharing, realtime), **when** it renders, **then** it
shows a plain value or description without a dropdown chevron that implies an
unimplemented menu.

## Test scenarios (written before implementation)

- **Pure formatting:** `format_number` for 0, 1234, 1234.5, negative values and
  all three formats; `format_timestamp` for ISO and the legacy
  `MMM DD, YYYY HH:MM` shape, both time formats, and an unparseable string.
- **Settings round-trip:** JSON and SQLite persist `number_format` and
  `time_format`; the workspace diff reports a settings change.
- **UI state:** selecting a format updates the preview immediately; reopening
  Settings keeps the choice.
- **Screenshots:** `settings-preferences-12h`, `settings-preferences-24h`,
  `settings-preferences-comma`, `settings-scrollbar`.
- **Regression:** existing settings search/row tests and the full gate.

## Design

- Define `SETTINGS_PADDING`, `SETTINGS_RAIL_PADDING_Y` and
  `SETTINGS_SCROLLBAR_MARGIN` constants in `dialogs/settings.rs`. Use the same
  values for the rail, the close control and the content inset so the three
  cannot drift.
- Place the close control at
  `right - SETTINGS_PADDING - radius`, `top + SETTINGS_PADDING + radius`.
- Give the rail an explicit top and bottom `add_space(SETTINGS_RAIL_PADDING_Y)`.
- Wrap the content `ScrollArea` in a `ui.allocate_ui` whose width is
  `available - SETTINGS_SCROLLBAR_MARGIN`; the scrollbar then sits inside the
  margin rather than on the border.
- Add a pure `ui/format.rs` module with `format_number` and
  `format_timestamp`, re-exported from `ui.rs` for integration tests. It is
  presentation code, not domain logic.
- Replace direct `updated_at` labels with `format_timestamp(&value, view.time_format)`
  and numeric table cells with `format_number(value, view.number_format)`.
- Keep `Automatic` locale-aware by delegating to the active locale's separators
  in one place; the other two formats are explicit.

## Plan / tasks

- [x] Add the settings fields, enums and serde defaults.
- [x] Add the pure formatter module and tests.
- [x] Apply the formatter to Library, version history, offline and comments.
- [x] Fix the close/rail/scrollbar padding with constants.
- [x] Add preview states and screenshots.
- [x] Update data model/requirements and run the full gate.

## Decisions and open questions

- Decision: `Automatic` follows the locale; the explicit formats override it.
- Decision: unparseable timestamps are passed through unchanged.
- Open: whether number format should also affect the table's "Created by"
  counts or only numeric cells. The default is numeric cells and counts.
- Open: whether the rail should scroll independently. The default is a fixed
  rail with its own bottom padding.
