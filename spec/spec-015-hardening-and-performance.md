# spec-015 — Hardening, missing surfaces and performance

## Purpose and scope

This spec closes gaps found while reviewing the code against `AGENTS.md`,
`docs/requirements.md` and the earlier specs. It is deliberately contained:
no visual redesign, no new dependency, and no change to the public API
contract except one additive read route already present on the server
(`GET /api/pages/{id}/versions`).

Scope:

1. **Settings search actually filters** the navigation rail (spec-014 item 4
   was only interactive, not filtering).
2. **Version history is reachable from the page actions menu** and can restore
   a snapshot. The server route and `CollabState::versions_for` already exist;
   the client and UI did not use them.
3. **Search results are cached** instead of asking the server on every frame.
4. **Persistence is safer and failures are visible**: JSON saves go through a
   temporary file, and a failing `WorkspaceStore` now produces HTTP 500 instead
   of a silent success.

## Evidence

- `src/ui/dialogs.rs` sets `settings_query` but the rail ignores it.
- `src/ui/dialogs.rs` lists "Version history" as a no-op
  (`Command::ClosePageActions`), while `src/server.rs` already serves
  `/api/pages/{id}/versions`.
- `src/ui/app.rs` calls `self.client.search(...)` on every rendered frame when
  the query is non-empty.
- `src/server.rs` contains `let _ = self.persist();`, so a disk failure is
  reported as a successful mutation.
- `src/store.rs` writes directly to the target path, so an interrupted write
  can leave a truncated JSON document.

## Acceptance criteria

1. **Given** the settings window, **when** the user types a query, **then**
   only navigation entries whose title or description match are shown and a
   clear "no match" message appears when nothing matches.
2. **Given** a page with recorded versions, **when** the user chooses
   "Version history", **then** the newest-first versions are listed and the
   selected snapshot can be restored through the client.
3. **Given** a search query, **when** the frame redraws without a query or
   workspace change, **then** the server is not asked again (cache hit).
4. **Given** a store whose `persist` fails, **when** a mutation is handled,
   **then** the API returns `500` with an error envelope.
5. **Given** a successful save, **when** it completes, **then** the target file
   is valid JSON and no temporary file remains.

## Test scenarios (written before implementation)

- `tests/ui.rs`: `settings_matches` is case-insensitive, matches title or
  description, accepts an empty query, and rejects unrelated text.
- `tests/ui.rs`: `SearchCache::needs_refresh` is true on the first call, false
  for the same query/version, and true when either changes.
- `tests/client.rs`: `page_versions` returns newest-first snapshots after two
  updates.
- `tests/store.rs`: `save` leaves no temporary file and the file loads back.
- `tests/store_port.rs`: a failing store yields `500` from a mutation route.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `ui/settings_search.rs` pure matcher; wire the settings rail
- [x] `ui/search_cache.rs` pure cache; use it in `NotionApp`
- [x] `Client::page_versions`; version-history window + restore command
- [x] Atomic JSON save + surface persistence errors as `500`
- [x] Docs, changelog, build/test/screenshot, commit

> **Historical paths (2026-11-08):** the evidence above names
> `src/ui/dialogs.rs` and `src/ui/app.rs` as they existed before `spec-017`.
> The current modules are `ui/dialogs/*`, `ui/apply.rs` and `ui/app.rs` (state
> shell); `ui/settings_search.rs` and `ui/search_cache.rs` still own the pure
> helpers described here.
