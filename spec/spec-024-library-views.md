# spec-024 — Library views

## Purpose and scope

The Library screen had placeholder behavior: `Shared` showed every page,
`Private` meant "root pages", `More` was a tab that showed everything, and the
`Created by` / `Last edited` columns were hard-coded. This spec makes the tabs
real views over the page list and shows actual metadata.

Scope:

- functional Recents / Favorites / Shared / Private filters;
- a `More` menu with placeholder views (AI Meeting Notes, Agents, Templates,
  Trash);
- real Created by, Source and Last edited time values;
- refresh shares/members when the Library opens and for the
  `ION_OF_NOTES_SHOW=library` preview.

Out of scope:

- server-side pagination or sorting;
- real AI Meeting Notes / Agents / Templates data;
- per-user ownership (the owner label comes from the member directory);
- a dedicated Trash resource.

## Evidence

- `screenshots/picture-53.png` … `picture-63.png`: Library tabs, table columns,
  empty states and card views.
- `src/ui/render/library.rs`: placeholder filtering and hard-coded cells.
- `Client::list_shares(None)` / `Client::list_members` already expose the data.
- `Share.page_id == "*"` is the workspace-general rule; the Library views use
  page-specific shares only.

## Acceptance criteria

1. **Given** the Favorites tab, **when** it renders, **then** only favorite
   pages are listed.
2. **Given** the Shared tab, **when** it renders, **then** only pages with a
   page-specific share are listed; the general `"*"` rule does not mark every
   page as shared.
3. **Given** the Private tab, **when** it renders, **then** pages without a
   page-specific share are listed.
4. **Given** the Recents tab, **when** it renders, **then** all pages are
   listed in workspace order.
5. **Given** the More menu, **when** it opens, **then** AI Meeting Notes,
   Agents, Templates and Trash are offered; selecting one shows its empty
   state.
6. **Given** a page row, **when** it renders, **then** Created by is the owner
   from the member directory, Source is the workspace name, and Last edited
   time is the page's `updated_at`.
7. **Given** the Library opens, **when** it loads, **then** shares and members
   are refreshed through the client.

## Test scenarios (written before implementation)

- `tests/ui.rs`: `pages_for_view` for Favorites, Shared, Private, Recents and
  an unknown placeholder view.
- Existing client tests cover `list_shares` / `list_members`.
- The Library screenshot is checked with `ION_OF_NOTES_SHOW=library`.

## Design

- `ui/library_filter.rs`: pure `pages_for_view(pages, tab, shared)`.
- `NotionApp`/`ViewState` hold `library_shared: Arc<HashSet<String>>` and
  `library_owner: String`.
- `refresh_library` loads all shares, keeps page-specific ids, and finds the
  owner name.
- `render_library` uses the pure filter, a `ui.menu_button("More", …)` menu and
  real row metadata.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `pages_for_view` + tests
- [x] Library state and `refresh_library`
- [x] Functional tabs, More menu and real columns
- [x] Docs, changelog, build/test/clippy/fmt/screenshot, commit
