# spec-017 — UI modularization refactor

## Purpose and scope

This is a **pure refactor** of the UI layer. Three files had grown into
multi-responsibility modules:

| Before | Lines | Responsibilities mixed |
| ------ | ----- | ---------------------- |
| `src/ui/dialogs.rs` | 2029 | settings, settings pages, sharing, inbox, page actions, version history, menus, shortcut sheet, search/new-tab windows, shared widgets |
| `src/ui/app.rs` | 1508 | state struct, construction, command shapes, view snapshot, input translation, command application, eframe shell |
| `src/ui/render.rs` | 1335 | window chrome, sidebar, page/blocks, Library, inline database |

The refactor splits them **by coherent responsibility**, not by an arbitrary
line-count threshold (ADR-004). No behavior, public API, visual style or
screenshot changes are intended.

## Evidence

- `docs/adr-004-modularization-decision.md`: split by responsibility, not line
  count; avoid one-struct-per-file fragmentation.
- `AGENTS.md` (SRP, DRY, visibility rules, `ui.rs` facade).
- The existing test suite (85 tests) and `--screenshot` are the safety net.
- `spec/spec-016-architecture-follow-up-and-observation.md` A5 planned this
  split; this spec executes it and extends it to the two other large UI files.

## Result

### `ui/app.rs` → 389 lines

- `ui/app.rs`: `NotionApp` struct, `new()`, `eframe::App` shell and the
  `ION_OF_NOTES_SHOW` preview hook.
- `ui/apply.rs` (823 lines): command application and state transitions.
- `ui/commands.rs` (182 lines): `Command`, `IconTab`, `FontStyle`,
  `NewTabChoice`, `new_tab_choices`.
- `ui/view.rs` (69 lines): `ViewState`.
- `ui/input.rs` (148 lines): egui key events → shortcut actions.

### `ui/dialogs.rs` → 21-line facade

- `ui/dialogs/settings.rs` (389 lines): settings window, rail, search filter.
- `ui/dialogs/settings_pages.rs` (502 lines): Preferences, Workspace, Sidebar,
  Offline, People, AI assistant.
- `ui/dialogs/sharing.rs` (288 lines): Share popover and Inbox.
- `ui/dialogs/page_actions.rs` (333 lines): page actions and version history.
- `ui/dialogs/menus.rs` (331 lines): app / workspace / grid / slash menus.
- `ui/dialogs/help.rs` (90 lines): shortcut sheet.
- `ui/dialogs/search_window.rs` (163 lines): shared search / new-tab windows.
- `ui/widgets.rs` (87 lines): shared popover frame, menu row, toggle and
  section label.

### `ui/render.rs` → 16-line facade

- `ui/render/chrome.rs` (112 lines): tab strip and window controls.
- `ui/render/sidebar.rs` (359 lines): search, nav rows, favorites, page tree,
  bottom account / invite / customize group.
- `ui/render/content.rs` (582 lines): breadcrumbs, page header, blocks,
  floating controls.
- `ui/render/library.rs` (146 lines): Library table and tabs.
- `ui/render/database.rs` (196 lines): inline table and board database.

## Acceptance criteria

1. **Given** the UI modules, **when** they are split, **then** `ui.rs` remains
   the facade and `ui::app` keeps one import point for render/dialog functions.
2. **Given** a render or dialog module, **when** it is imported, **then** it
   depends on `commands`, `view`, `palette`, `icons` or `widgets` directly, not
   on the whole state machine.
3. **Given** the refactor, **when** the app runs, **then** behavior and visuals
   do not change; the existing screenshot is unchanged.
4. **Given** the full suite, **when** `cargo build`, `cargo test --all-targets`
   and `cargo clippy --all-targets -- -D warnings` run, **then** all are green.
5. **Given** a file with more than one responsibility, **when** it is split,
   **then** the split is by responsibility, not by line count; a cohesive
   400–800-line file is acceptable.

## Test scenarios (written before the refactor)

- The existing 85 integration tests are the behavior contract; no test changes
  are needed for a pure move.
- The red step is a compile failure when the new module declarations exist
  before the code is moved.
- `cargo clippy --all-targets -- -D warnings` must stay clean after the split.
- `cargo run -- --screenshot shots/spec017.png` must produce the same default
  window as the pre-refactor screenshot.

## Plan / tasks

- [x] Create `ui/commands.rs`, `ui/view.rs`, `ui/input.rs`; keep `app.rs` as
  the struct/eframe shell.
- [x] Move command application to `ui/apply.rs`.
- [x] Create `ui/widgets.rs` for shared popover/menu/toggle/label primitives.
- [x] Split `ui/dialogs.rs` into a facade plus settings, settings pages,
  sharing, page actions, menus, help and search-window modules.
- [x] Split `ui/render.rs` into a facade plus chrome, sidebar, content, library
  and database modules.
- [x] Keep `ui.rs` as the facade; update `AGENTS.md` and `README.md`.
- [x] `cargo build`, `cargo test --all-targets`, clippy, screenshot.

## Notes

- `ui/apply.rs` remains the largest UI file (823 lines) because it is one
  coherent command-application switch. Splitting it further by command category
  would create artificial boundaries; revisit only if a real second
  responsibility appears.
- No behavior, protocol, persistence or visual change is part of this spec.
- The next feature spec can start from the smaller, single-purpose modules
  instead of the three original monoliths.
