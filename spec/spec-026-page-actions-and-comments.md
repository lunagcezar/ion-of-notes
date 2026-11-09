# spec-026 — Page actions and comments

## Purpose and scope

The page-actions menu still had placeholders, and pages had no table of
contents, discussions, inline-comment flag or archive state. This spec wires
the page-level actions that can be implemented without a new backend service.

Scope:

- Copy page contents as markdown.
- Persisted page flags: `show_toc`, `discussions`, `inline_comments`,
  `archived`.
- A rendered table of contents from heading blocks.
- Page-level discussions: list, add and delete comments.
- Archive/restore with sidebar and Library filtering.
- Status feedback for the remaining placeholder actions (Move to, Updates &
  analytics, Suggest edits, Notify me, More).
- A `ION_OF_NOTES_SHOW=discussions` preview.

Out of scope:

- block-anchored inline comments (the flag is persisted, but anchors are not
  implemented);
- real notifications, analytics or suggest-edit workflows;
- a move-to picker;
- server-side comment notifications.

## Evidence

- `screenshots/picture-13.png` / `picture-14.png`: page actions with
  Appearance & layout, Table of contents, Page discussions, Inline comments,
  Archive and More.
- `src/ui/dialogs/page_actions.rs`: previous placeholder commands.
- `src/model.rs`: `Page` already persists per-page flags such as `locked` and
  `available_offline`.

## Acceptance criteria

1. **Given** a page, **when** Copy page contents is clicked, **then** the page
   markdown is copied and a status message is shown.
2. **Given** Table of contents is enabled, **when** the page renders, **then**
   non-empty headings are listed in page order with indentation by level.
3. **Given** Page discussions is enabled, **when** the page renders, **then**
   existing comments are listed and a new comment can be added or deleted.
4. **Given** Inline comments is enabled, **when** the page renders, **then**
   the setting is persisted and a clear note explains that block anchors are
   not implemented yet.
5. **Given** Archive is clicked, **when** the page is archived, **then** it is
   hidden from Favorites, Recents, Shared, Private and the sidebar tree, and
   appears in the Library Archive view.
6. **Given** a placeholder action, **when** it is clicked, **then** the menu
   shows a status message instead of silently doing nothing.
7. **Given** `ION_OF_NOTES_SHOW=discussions`, **when** the screenshot runs,
   **then** the TOC and discussions state is rendered.

## Test scenarios (written before implementation)

- `tests/model.rs`:
  - page action flags + comments JSON round-trip;
  - heading filtering for the TOC;
  - favorites and search ignore archived pages.
- `tests/ui.rs`: Library filtering excludes archived pages and the Archive
  view includes them.
- The page-actions and discussions screenshots are checked manually.

## Design

- `Page` gains `show_toc`, `discussions`, `inline_comments`, `archived` and
  `comments: Vec<Comment>` with serde defaults.
- `Comment { id, author, text, created_at }` is a serde-friendly model type.
- `Page::headings()` returns non-empty heading blocks for the TOC.
- `Workspace::favorites` and search skip archived pages.
- `pages_for_view` excludes archived pages from normal views and adds
  `Archive`.
- `Command` gains `CopyPageContents`, the four page toggles, comment
  input/add/delete and `SetPageStatus`.
- `render_page` renders the TOC and page discussions; `render_page_actions`
  renders the toggles and status.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] Page flags, `Comment`, headings, archive filtering
- [x] Commands/state and page-action toggles
- [x] TOC and discussions UI
- [x] Docs, changelog, build/test/clippy/fmt/screenshots, commit
