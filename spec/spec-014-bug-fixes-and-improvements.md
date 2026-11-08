# spec-014 — Bug fixes and improvements

## Purpose and scope

Address a batch of reported bugs and implement requested improvements: editor
behavior, sidebar/header polish, modal interactions, settings functionality,
page locking/offline, icon picker, internationalization groundwork, and
workspace/library UI wiring. All changes follow TDD (red→green), SDD, DRY,
Clean Architecture, and update docs/changelog.

## Evidence

- User report listing 20 issues with attachments (UI references in
  `attachments/picture-38.png`–`picture-72.png`).
- Existing implementation in `app/src/ui/*`, `model.rs`, `server.rs`,
  `collab.rs`.

## Acceptance criteria

1. **Given** an empty non-divider block with focus, **when** the user presses
   Backspace, **then** the block is removed and focus moves appropriately
   (keeping at least one block on the page).
2. **Given** the block editor, **when** the mouse moves over other blocks while
   typing, **then** input focus is not cancelled (stable layout/IDs for text
   edits).
3. **Given** the six-dot block handle, **when** dragged over another block and
   released, **then** the block reorders to the target position.
4. **Given** settings search text, **when** typed, **then** settings sections
   and rows matching title/description are shown/filtered.
5. **Given** a locked page, **when** editing is attempted, **then** text edits
   and structural changes are disabled.
6. **Given** a modal window (search, new tab, settings), **when** the user
   clicks outside it, **then** the modal closes.
7. **Given** the page icon picker, **when** Emoji tab is selected, **then**
   actual emoji glyphs are available alongside symbols; icon/upload tabs work.
8. **Given** the sidebar, **when** rendered, **then** it has proper padding and
   the workspace/account row exposes a dropdown with workspace actions.
9. **Given** the title bar, **when** rendered, **then** sidebar toggle sizing
   is compact and the pin-like quick-note button is replaced by an app menu
   button opening File/Edit/View/History/Window/Help.
10. **Given** library tabs, **when** clicked, **then** the list filters
    accordingly (functional tabs).
11. **Given** the bottom-right grid/four-point icon area, **when** clicked,
    **then** a menu with Library/Skills/Trash etc. appears.

## Test scenarios (before implementation)

- `tests/ui.rs` / `tests/model.rs` / new targeted tests: block deletion helper,
  settings search matching, lock-page edit guard logic where pure.
- Existing tests must remain green.

## Plan / tasks

- [x] Spec + tests (red where feasible)
- [x] Fix backspace-on-empty block deletion (render/app)
- [x] Fix hover layout shift stealing focus (stable block row layout/IDs)
- [x] Improve block drag reorder reliability
- [x] Settings search filtering — the navigation rail filters (spec-015); the
  row-level filter inside the open section is still **open** as spec-016 A9.
- [x] Lock page enforcement in UI
- [x] Click-outside-to-close for modals
- [x] Icon picker emoji catalog improvements
- [x] Sidebar padding, workspace dropdown
- [x] Title bar: compact toggle, app menu replacing pin icon
- [x] Library functional tabs (completed in `spec-024`)
- [x] Four-point/grid menu
- [x] i18n groundwork (rust-i18n) + language setting wiring; coverage expanded
  in `spec-028`
- [x] Version history/offline/agent/customize-sidebar deeper features: version
  history and offline were completed in `spec-015`/`spec-026`; the remaining
  AI/agent surfaces stay explicitly out of scope
- [x] Docs + changelog + build + screenshot checks
