# spec-049 - UI and client modularization

- Status: implemented (2026-11-11)
- Related: `docs/review-findings.md` section 9 (RF-M03..RF-M06),
  `docs/adr-004-modularization-decision.md`, `spec-046`, `spec-047`

## Purpose and scope

This is the deferred UI/client split from the modularity review. The behavior
is unchanged; the files that still held more than one responsibility are
divided along their real seams.

In scope:

- Split `ui/apply.rs` (the command state machine) into command-family modules
  behind one `apply_commands` entry point.
- Split `ui/dialogs/settings_pages.rs` into one module per settings page and
  move `section_heading`/`info_row` to `ui/widgets.rs`.
- Split `ui/render/content.rs` into page, block and remote-cursor modules while
  keeping `render_content` as the entry point.
- Extract `RetryPolicy`/`RetryDecision` into `client/retry.rs` and re-export
  them from `client.rs`.
- Update the architecture/docs map, review findings, spec index and changelog.

Out of scope:

- No behavior, protocol, persistence, DTO or visual change.
- No new crate, no `domain/ application/ infrastructure/` scaffold.
- No split of the thin `client.rs` resource wrappers by noun.
- No change to the test count or the public API.

## Evidence

- `docs/review-findings.md` section 9: RF-M03 deferred the command-family split
  to spec-049; RF-M04/RF-M05/RF-M06 name the settings pages, page-column render
  and retry policy as the remaining cohesive splits.
- At the start of this session: `ui/apply.rs` was 1,300 lines,
  `ui/dialogs/settings_pages.rs` 885 lines, `ui/render/content.rs` 798 lines.
- `tests/ui.rs`, `tests/fidelity.rs`, `tests/ui_crdt.rs`, `tests/screenshot.rs`
  and the `ION_OF_NOTES_SHOW` preview states are the behavior contract.
- `Command::forces_flush` and `Command::blocked_when_locked` already centralize
  the two command classifications (spec-046); this spec adds only the family
  routing needed to split the state machine.

## Acceptance criteria

1. **Given** a `Command`, **when** `apply_commands` runs, **then** it still
   checks flush/lock once and routes the command through `CommandFamily` to one
   family module; the same state transitions and follow-up commands occur.
2. **Given** the settings window, **when** any of the seven pages renders,
   **then** each page lives in its own module and the shared heading/info rows
   come from `ui/widgets.rs`.
3. **Given** the content area, **when** `render_content` runs, **then** page,
   block and remote-cursor rendering are separate modules and the entry point
   is unchanged.
4. **Given** the public client API, **when** `RetryPolicy`/`RetryDecision` are
   used, **then** they are re-exported from `client.rs` from `client/retry.rs`
   with the same behavior.
5. **Given** the full gate, **when** fmt, both clippy passes, the core and
   desktop tests, the docs checks and the preview screenshots run, **then** all
   are green and the window is unchanged.
6. **Given** the docs, **when** the spec completes, **then** `AGENTS.md`,
   architecture, review findings, README, spec index and changelog reflect the
   new module map.

## Test scenarios (written before implementation)

- **Compile-red step:** add the new module declarations before moving the
  functions; `cargo check --all-targets` fails with unresolved modules.
- **Existing suite:** `cargo test --all-targets --no-default-features` (229)
  and the four desktop test crates (26) stay green.
- **Command routing:** the compiler checks that `CommandFamily` covers every
  `Command` variant; a static audit confirms the union of family modules covers
  all variants except `NoOp`, which `apply_commands` handles directly.
- **Visual smoke:** `cargo run -- --screenshot shots/spec049.png` and the
  `ION_OF_NOTES_SHOW=settings-general|settings-people|settings-ai|settings-server|settings-sidebar|settings-offline`
  previews all render.
- **Lint/format:** `cargo clippy --all-targets -- -D warnings`,
  `cargo clippy --all-targets --no-default-features -- -D warnings` and
  `cargo fmt --check` are clean.
- **Docs:** `pwsh -File scripts/check-docs.ps1` passes.

## Design

```text
src/ui/apply.rs                  facade: apply_commands, shared helpers
src/ui/apply/
  navigation.rs                  pages, search, overlays, window, search panes
  library.rs                     Library open/close/tab
  sharing.rs                     share, invite, general access, inbox
  blocks.rs                      block editing, drag, to-do, table rows
  workspace.rs                   reset, undo/redo, duplicate, reload, copy
  tabs.rs                        tabs and the "+" switcher
  ai.rs                          Ollama assistant
  settings.rs                    settings, help, zoom, app menus, appearance
  page_actions.rs                page icon, actions, comments, versions, offline/lock
  crdt.rs                        CRDT text and awareness
  server.rs                      server login/connect/disconnect

src/ui/dialogs/settings_pages.rs facade
src/ui/dialogs/settings_pages/   preferences, workspace, sidebar, offline,
                                 people, ai, server
src/ui/render/content.rs         facade, render_content entry point
src/ui/render/content/           page, blocks, cursors
src/client/retry.rs              RetryPolicy, RetryDecision
```

`CommandFamily` lives in `ui/commands.rs` next to `forces_flush` and
`blocked_when_locked`; it is the single classification table for routing.
`apply_workspace` returns whether undo/redo ran so `apply_commands` can keep
the same undo-stack behavior. `settings_pages` and `content` keep their old
facade names, so callers do not change.

## Plan / tasks

- [x] Write this spec and record the compile-red step.
- [x] Extract `client/retry.rs` and re-export the public types.
- [x] Split the settings pages and move the two shared helpers.
- [x] Split `render/content` into page, blocks and cursors.
- [x] Add `CommandFamily` and split `ui/apply.rs` by family.
- [x] Run the existing tests, both clippy passes, fmt and the preview screenshots.
- [x] Update `AGENTS.md`, architecture, review findings, README, docs hub,
      spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
