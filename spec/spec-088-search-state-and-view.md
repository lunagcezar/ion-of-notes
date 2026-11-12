# spec-088 - SearchState and SearchView grouping

- Status: implemented (2026-11-12)
- Related: `spec-013`, `spec-017`, `spec-087`
- Review sources: Claude and ChatGPT (composable app state, narrow view models)

## Purpose and scope

Search state was spread across `NotionApp` as nine `search_*` fields, and the
same fields were repeated in `ViewState`. This spec groups them into
`SearchState` (runtime) and `SearchView` (presentation snapshot).

Scope:

- `SearchState` in `ui/app.rs`: open, query, results, title-only, creator,
  in-page, has-comments, cache and selected;
- `SearchView` in `ui/view.rs` with the presentation fields;
- `ViewState` holds `search: SearchView`;
- `view_state()` builds the search view in one place;
- search-window and app references use `view.search.*` / `self.search.*`.

Out of scope:

- grouping sharing, collaboration, settings or AI state;
- changing the search UI or filters;
- a generic state framework.

## Acceptance criteria

P1. **Given** the app state, **when** search is opened or filtered, **then**
the behavior is unchanged.

P2. **Given** a render frame, **when** the search window is built, **then** it
reads the grouped `SearchView` rather than nine flat `ViewState` fields.

P3. **Given** the search cache, **when** the query or workspace version
changes, **then** the refresh rule is unchanged.

P4. **Given** the existing search, UI and screenshot tests, **when** the fields
move, **then** they pass unchanged.

## Test scenarios

- `tests/ui.rs` (search cache, filters, switcher helpers)
- `tests/screenshot.rs`
- `--screenshot` for the search state
- Full quality gate

## Design

- `SearchState` owns the cache and selected index; `SearchView` owns only the
  immutable presentation fields.
- The search renderer still receives `ViewState` because the shared switcher
  also needs highlight/filter/member data; the search fields are grouped under
  `view.search`.
- No new abstraction or trait is introduced.

## Plan / tasks

- [x] Write spec-088
- [x] Group the runtime and view fields
- [x] Update references
- [x] Run the full gate and commit
