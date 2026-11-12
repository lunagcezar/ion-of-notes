# spec-098 - Group sidebar commands

- Status: implemented (2026-11-12)
- Related: `spec-049`, `spec-080`, `spec-013`
- Review sources: Claude and ChatGPT (`Command` is one large flat family)

## Purpose and scope

`Command` carried twelve sidebar-specific variants next to the page, block,
tab, settings and server commands. This spec groups them into
`SidebarCmd` and one `Command::Sidebar(SidebarCmd)` variant:

- toggle the sidebar;
- expand/favorite a page;
- hide a section or shortcut;
- move a section or shortcut;
- toggle a navigation item;
- add, switch or remove a sidebar tab.

`CommandFamily` still routes the command to the module that owns the
transition: navigation for toggle/expand/favorite, settings for layout.

Out of scope:

- grouping page, block, tab, settings and server commands;
- changing sidebar behavior, persistence or layout;
- changing the shortcut table.

## Acceptance criteria

P1. **Given** a sidebar toggle, expand or favorite action, **when** it is
applied, **then** navigation behavior is unchanged.

P2. **Given** a sidebar layout action (hide, move, tab), **when** it is applied,
**then** `WorkspaceSettings::sidebar_layout` changes as before.

P3. **Given** the command-family dispatch, **when** a sidebar command is
classified, **then** it reaches the same module as before.

P4. **Given** the existing UI tests, **when** the variants move, **then** they
pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/fidelity.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- `SidebarCmd` is a nested enum; `Command::Sidebar` is the only new variant.
- `family()` matches the nested variants, so dispatch modules do not change.
- `forces_flush` and `blocked_when_locked` match the nested favorite variant.
- No behavior moves: the same match arms receive the same data.

## Plan / tasks

- [x] Add `SidebarCmd` and `Command::Sidebar`
- [x] Convert construction sites and matches
- [x] Remove the flat variants
- [x] Run the full gate and commit
