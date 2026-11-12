# spec-096 - Narrow settings view models

- Status: implemented (2026-11-12)
- Related: `spec-088`, `spec-092`, `spec-095`, `spec-017`
- Review sources: Claude and ChatGPT (narrow view models, interface segregation)

## Purpose and scope

The settings pages received the whole `ViewState` even though each page reads
only one feature slice. This spec narrows their parameters:

- `render_ai_settings` takes `&AiView`;
- `render_server_settings` takes `&ServerView`;
- `render_preferences` takes `&SettingsView` and `&ServerView`;
- `render_workspace_settings` takes `&SettingsView`;
- `render_people_settings` takes `&SharingView` and `&SettingsView`.

The single settings dispatcher in `dialogs/settings.rs` still receives the full
snapshot and passes the slices down.

Out of scope:

- narrowing sidebar, content, chrome, library and database renderers;
- changing any settings layout or behavior.

## Acceptance criteria

P1. **Given** the AI settings page, **when** it renders, **then** it reads only
the AI slice and the output is unchanged.

P2. **Given** the Server settings page, **when** it renders, **then** it reads
only the server slice and the output is unchanged.

P3. **Given** the Preferences, General and People pages, **when** they render,
**then** they receive only the settings/sharing slices they use.

P4. **Given** the existing UI/screenshot tests, **when** the signatures change,
**then** they pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/fidelity.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- Narrowing happens at the page boundary, where the dependency is explicit.
- No new trait, no generic render context.
- The dispatcher remains the only caller, so call-site churn stays small.

## Plan / tasks

- [x] Narrow the five settings page signatures
- [x] Update the dispatcher call sites
- [x] Run the full gate and commit
