# spec-018 — Domain and infrastructure modularization

## Purpose and scope

This is the second **pure refactor** pass, after `spec-017` split the UI. It
splits the three non-UI files that still mixed responsibilities:

| Before | Lines | Mixed responsibilities |
| ------ | ----- | ---------------------- |
| `src/model.rs` | 532 | entities, workspace tree behavior, ranked search, demo seed data |
| `src/collab.rs` | 532 | collaboration DTOs/state, realtime contract, markdown/CSV export |
| `src/server.rs` | 518 | API routes/use cases, HTTP/1.1 transport, URL encoding |

The split is by coherent responsibility (ADR-004), not by line count. No
behavior, public API, protocol or persistence change is intended.

## Evidence

- `docs/adr-004-modularization-decision.md`: split by responsibility, not line
  count; avoid one-struct-per-file fragmentation.
- `AGENTS.md`: domain must not know infrastructure/GUI; adapters implement
  ports.
- `spec/spec-017-ui-modularization.md`: the first refactor pass.
- The existing 85 integration tests and `--screenshot` are the safety net.

## Result

### `model.rs` → 356 lines

- `model.rs`: `BlockKind`, `Block`, `TableRow`, `Page`, `PageSummary`,
  `SearchResult`, `PageDraft`, `WorkspaceError`, `Workspace` struct and tree
  behavior (`page`, `page_mut`, `add_page`, `remove_page`, `children`,
  `breadcrumbs`, `favorites`, `bump_version`).
- `model/seed.rs` (133 lines): the neutral demo workspace.
- `model/search.rs` (66 lines): ranked search and the excerpt helper.

### `collab.rs` → 381 lines

- `collab.rs`: roles, access, members, shares, changes, versions, sync batch,
  realtime DTOs/contract, `ExportFile` and `CollabState`.
- `collab/export.rs` (149 lines): markdown pages, People files, CSV and the
  export tree.

### `server.rs` → 364 lines

- `server.rs`: `Method`, `ApiRequest`, `ApiResponse`, `Envelope`,
  `WorkspaceApi`, `WorkspaceServer`, JSON/error helpers and the route match.
- `server/http.rs` (114 lines): HTTP/1.1 request parsing, response formatting,
  `serve_once` and `serve_forever`.
- `server/url.rs` (44 lines): percent-encoding/decoding shared by the client
  and the server.

## Files that intentionally stay as they are

These are single-purpose and do **not** need a split:

- `ollama.rs`: one AI-assistant adapter (CLI + HTTP transports).
- `client.rs`: one typed API client.
- `store.rs`: one persistence port + JSON adapter.
- `shortcuts.rs`: one pure shortcut table.
- `app.rs` (bootstrap): one composition root.
- `ui/switcher.rs`: one shared search/new-tab window.
- `ui/palette.rs`: one visual constants/theming module.
- `ui/icons.rs`: one original icon-artwork module.
- `ui/page_icon.rs`: one page-icon feature.
- `ui/apply.rs`: one command-application switch (see spec-017).

`ui/render/content.rs` and `ui/dialogs/settings_pages.rs` are the next possible
candidates if they grow, but they are still cohesive enough to keep.

## Acceptance criteria

1. **Given** the domain modules, **when** they are split, **then** `model.rs`
   has no demo data or search implementation; `model/seed.rs` and
   `model/search.rs` own those.
2. **Given** the collaboration module, **when** it is split, **then**
   `collab.rs` keeps the DTOs/state and `collab/export.rs` owns the export tree.
3. **Given** the server module, **when** it is split, **then** `server.rs`
   keeps the routes and `server/http.rs` owns the transport adapter.
4. **Given** the public imports in `lib.rs` and `tests/`, **when** the split is
   done, **then** they continue to work through the module facades.
5. **Given** the full suite, **when** `cargo build`, `cargo test --all-targets`
   and `cargo clippy --all-targets -- -D warnings` run, **then** all are green.
6. **Given** the default window, **when** `--screenshot` runs, **then** the
   picture is unchanged.

## Test scenarios (written before the refactor)

- Existing 85 integration tests are the behavior contract.
- The red step is a compile failure when the new module declarations exist
  before the code is moved.
- Clippy and rustfmt must stay clean on the changed files.
- The default screenshot must match the pre-refactor window.

## Plan / tasks

- [x] `model.rs` → `model/seed.rs` + `model/search.rs`.
- [x] `collab.rs` → `collab/export.rs`.
- [x] `server.rs` → `server/http.rs` + `server/url.rs`.
- [x] Keep `lib.rs` and test imports working through facades.
- [x] Update `AGENTS.md` and `README.md`.
- [x] `cargo build`, tests, clippy, rustfmt, screenshot.

## Notes

- No behavior, protocol, persistence or visual change is part of this spec.
- The public API is unchanged: `not_notion::model`, `not_notion::collab` and
  `not_notion::server` still expose the same names.
- `ui/render/content.rs` and `ui/dialogs/settings_pages.rs` remain the largest
  cohesive UI files; they are left alone until a real second responsibility
  appears.
