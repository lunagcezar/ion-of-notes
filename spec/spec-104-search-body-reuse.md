# spec-104 - Reuse the search body text

- Status: implemented (2026-11-12)
- Related: `spec-015`, `spec-097`
- Review source: ChatGPT performance analysis (repeated search work)

## Purpose and scope

`Workspace::search` called `page.plain_text()` twice for a body match: once to
build the searchable text and again to build the excerpt. It also built the
table-row suffix twice. This spec computes the body text once per page and
reuses it for ranking and excerpting.

The UI already caches results by query and workspace version, so this removes
the remaining duplicate work inside one search pass.

Out of scope:

- a persisted incremental search index;
- background search;
- fuzzy ranking.

## Acceptance criteria

P1. **Given** a body match, **when** search runs, **then** the excerpt is built
from the same text that was searched.

P2. **Given** a page with table rows, **when** search runs, **then** row names
are still searchable.

P3. **Given** the existing search tests, **when** the body is reused, **then**
results, ranking and snippets are unchanged.

## Test scenarios

- `tests/model.rs` (search ranking and excerpts)
- `tests/ui.rs` (search cache and filters)
- `tests/server.rs` (`/api/search`)
- Full quality gate

## Design

- Build `body_text` once (plain text plus optional row names).
- Lowercase it once for matching.
- Pass the original `body_text` to `excerpt`.

## Plan / tasks

- [x] Reuse the body text and row suffix
- [x] Run the search tests and full gate
