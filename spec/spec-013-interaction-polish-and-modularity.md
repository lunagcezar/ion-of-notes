# spec-013 — Notion-like interaction polish, two bug fixes, and modularity

## Purpose and scope

One working session covering, in order:

1. Two reported bugs.
2. A set of Notion-like interactions shown in `attachments/picture-3.png` …
   `attachments/picture-37.png`, implemented "as far as possible" with the
   current clean-room model.
3. A short architecture answer to the question about long `src` files, DDD and
   subfiles, recorded as `docs/adr-004-modularization-decision.md` and in
   `AGENTS.md`.
4. A git commit, as the standing automation rule requires.

Out of scope: real image decoding/upload, real drag-and-drop of files between
pages, accounts/sessions on the wire, and any content copied from the original
program (clean room).

## Evidence used

- `reference/picture-1.png` (default page), `reference/picture-13.png`
  (page actions), `reference/picture-15.png` (preferences chrome).
- `attachments/picture-3.png`, `picture-37.png`: icon picker with
  Emoji / Icons / Upload tabs, a filter field, Remove, and a glyph grid.
- `attachments/picture-4.png` … `picture-8.png`: search / new-tab window with
  Today / Past week groups, a highlight (preview) pane, show/hide highlight
  pane, show/hide filters, and a footer of keyboard hints.
- `attachments/picture-12.png` … `picture-17.png`: the "..." page actions and
  settings menu (Copy link, Duplicate, Move to, Appearance & layout,
  Available offline / Lock page toggles, More, Last edited footer).
- `attachments/picture-20.png` … `picture-23.png`: a customized page icon
  replicated into the sidebar, breadcrumbs and tab strip; compact top bar
  without a person/settings control.
- `attachments/picture-24.png` … `picture-37.png`: the block "+" and six-dot
  drag handle, Alt-click to add above, the categorized block menu.
- `attachments/picture-18.png`, `picture-19.png`, `picture-28.png`: sidebar
  customization (show/hide sidebar is the in-scope part).

## Architecture answer (see ADR-004)

The `tcg-matchmaker` worker tree is **feature-first modularization** (one
folder per feature, each with `index/repository/router/service`), not DDD.
That is a good *code-modularization* alternative for a many-feature backend.
For this crate the decision stays the pragmatic **Modular Monolith** of
ADR-002: split a file when it has **more than one coherent responsibility**,
never by a line-count threshold or "one struct per file"; use selective DDD
only where the domain has real invariants. Concretely this session extracted
two cohesive modules rather than a `domain/ application/ infrastructure/`
scaffold: `ui/switcher.rs` (one shared search/new-tab window) and
`ui/page_icon.rs` (the page-icon feature), keeping `ui.rs` as the facade.

## Acceptance criteria (Given / When / Then)

### Bugs

- **Given** the app window has just lost focus or the pointer is outside the
  window, **When** key events arrive, **Then** no global shortcut is fired and
  ordinary keystrokes reach the focused text editor (pure guard
  `shortcuts::shortcut_context_ok` + tests).
- **Given** more sidebar pages than fit vertically, **When** the sidebar is
  drawn, **Then** the page list scrolls inside a `ScrollArea` while the search
  box and the bottom account/invite group stay fixed.

### Interactions

- **Given** the search window or the "+" new-tab window, **When** either is
  opened, **Then** both use the same shared renderer (hint field, Today/Past
  week groups, preview pane, footer hints) — DRY.
- **Given** the search/new-tab window, **When** the highlight toggle is used,
  **Then** the preview pane is shown/hidden and the window reflows; **When**
  the filter toggle is used, **Then** the filter row is shown/hidden.
- **Given** the top-left panel control or `Ctrl+\`, **When** used, **Then** the
  sidebar is shown/hidden.
- **Given** the "..." control in the breadcrumb bar, **When** clicked, **Then**
  a page-actions menu opens with searchable actions, Appearance & layout
  (Default/Serif/Mono, Small text, Full width) and the offline/lock toggles.
- **Given** a hovered block, **When** "+" is clicked, **Then** an empty block is
  inserted **below** and the block menu opens; **When** Alt+"+" is clicked,
  **Then** the block is inserted **above** instead.
- **Given** the six-dot handle between "+" and the block, **When** dragged onto
  another block and released, **Then** the source block is reordered to that
  position; **When** clicked, **Then** the categorized block menu opens.
- **Given** the large page (book) icon, **When** clicked, **Then** an Emoji /
  Icons / Upload picker opens; **When** a glyph is chosen (or Remove is used),
  **Then** the icon is stored on the page and rendered in the header, sidebar,
  breadcrumbs, tab strip and switcher preview.
- **Given** the top bar, **When** rendered, **Then** it is compact (36 px), has
  no person/settings control, and otherwise matches the reference tab strip.

## Test scenarios (written before implementation)

Red step recorded in `tests/shortcuts.rs`:

- `shortcuts_require_a_present_pointer_and_window_focus`
- `stale_modifiers_are_rejected_but_real_shortcuts_are_allowed`

Both were observed failing (`unresolved import`) before
`shortcut_context_ok` was added; all 80 tests are green afterwards.

## Plan / task list

1. Add the pure shortcut guard + tests (red → green).
2. Compact the title bar; remove the person/settings control.
3. Wrap the sidebar page list in a bounded `ScrollArea`.
4. Add `ui/switcher.rs`; make search and new-tab thin wrappers (DRY).
5. Add `ui/page_icon.rs` + `icons::draw_page_icon`; wire replication.
6. Add the page-actions menu and Appearance & layout effects.
7. Add `+`/Alt-`+`, six-dot drag reorder, categorized block menu.
8. Build, test, `--screenshot`, compare with the 29 reference pictures.
9. Update ADR-004, `AGENTS.md`, `CHANGELOG.md`, requirements; commit.
