# spec-095 - SettingsState and SettingsView grouping

- Status: implemented (2026-11-12)
- Related: `spec-088`, `spec-092`, `spec-093`, `spec-094`, `spec-021`
- Review sources: Claude and ChatGPT (composable app state, narrow view models)

## Purpose and scope

The settings dialog state and the user's appearance preferences were flat on
`NotionApp` and repeated in `ViewState`. This spec groups them into
`SettingsState` (runtime) and `SettingsView` (presentation):

- open flag, selected page, search query and status message;
- language;
- theme (`dark`, `theme_dirty`);
- table view;
- font style, full width and small text;
- zoom factor and dirty flag.

`number_format` and `time_format` are workspace settings, so they appear only
in `SettingsView`, not in `SettingsState`.

Out of scope:

- changing the settings rail, search or any preference behavior;
- onboarding/templates windows;
- narrowing the remaining `ViewState` fields.

## Acceptance criteria

P1. **Given** the settings window, **when** it opens on any page, **then** the
selected page, query, status and search filtering are unchanged.

P2. **Given** a theme or zoom change, **when** the app redraws, **then** the
dirty flags apply the same theme and zoom as before.

P3. **Given** a language, font style, width or text-size change, **when** the
page renders, **then** the preference is unchanged.

P4. **Given** the existing UI/screenshot tests, **when** the fields move,
**then** they pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/fidelity.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- `SettingsState` mirrors the existing grouped-state convention.
- `SettingsView` is built once per frame from runtime state plus workspace
  settings.
- No behavior moves: only field paths and two structs change.

## Plan / tasks

- [x] Add `SettingsState` and `SettingsView`
- [x] Update the field references
- [x] Run the full gate and commit
