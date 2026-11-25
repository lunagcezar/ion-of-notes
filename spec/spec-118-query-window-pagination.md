# spec-118 - Query-window pagination for data sources and views

- Status: implemented (2026-11-24)
- Related: `spec-031`, `spec-033`, `spec-034`, `spec-116`
- Review source: ChatGPT performance review (2026-11-24)

## Purpose and scope

`POST /api/v2/views/{id}/query` called `query_view_rows`, which cloned every
matching row, sorted the whole vector, and only then paginated. The data-source
query route did the same with `rows.clone()`. A first-page request therefore
paid for the whole table.

This spec splits selection from materialization:

- `Workspace::query_view_row_indices` filters and sorts row *indices* without
  cloning a row.
- `Workspace::view_rows` exposes the underlying row slice.
- `PageWindow` in `server/pagination.rs` computes the cursor range, `has_more`
  and `next_cursor` without cloning.
- The view query clones only `indices[start..end]` into the response.
- Board/calendar grouping still needs every filtered row; it consumes the same
  index set (`group_view_rows_indices`) and clones only when it builds groups.
- The data-source query clones only the requested slice.
- `paginate` is now a thin wrapper over `page_window`.

Out of scope:

- a persistent database index (the rows are an in-memory vector);
- cursor keyset pagination on property values;
- changing the cursor wire format.

## Acceptance criteria

P1. **Given** a view with more rows than the page size, **when** it is queried,
**then** only the requested window is cloned into `pages`.

P2. **Given** a filter and sort, **when** the window is queried, **then** the
indices select the same rows as the legacy materialized query.

P3. **Given** a board or calendar view, **when** it is queried, **then** groups
contain every filtered row exactly once.

P4. **Given** an invalid or foreign cursor, **when** it is queried, **then**
`page_window` returns `invalid_cursor` and no rows are cloned.

P5. **Given** a data-source query, **when** a page is requested, **then** only
that row slice is materialized.

## Test scenarios

- `tests/model.rs`
  (`view_row_indices_materialize_the_same_rows_as_the_legacy_query`).
- `tests/pagination.rs`
  (`page_window_selects_the_same_slice_without_cloning_the_list`).
- `tests/views.rs` (`v2_view_query_paginates`, board/calendar grouping).
- `tests/properties.rs` (`v2_data_source_query_paginates_rows`).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Cursor window | `server/pagination.rs` (`PageWindow`, `page_window`) |
| Index selection | `model/workspace.rs` (`query_view_row_indices`) |
| Row slice | `model/workspace.rs` (`view_rows`) |
| Grouping by index | `model/workspace.rs` (`group_view_rows_indices`) |
| View handler | `server/handlers/views.rs` |
| Data-source handler | `server/handlers/databases.rs` |

## Plan / tasks

- [x] Add `PageWindow` and `page_window`; keep `paginate` compatible.
- [x] Add index-based view selection and row-slice access.
- [x] Materialize only the requested view window.
- [x] Reuse the index set for board/calendar groups.
- [x] Materialize only the requested data-source window.
- [x] Add model and pagination regression tests.
