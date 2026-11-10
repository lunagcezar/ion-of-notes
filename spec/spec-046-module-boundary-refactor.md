# spec-046 - Module boundaries for server, model and command metadata

- Status: implemented (2026-11-10)
- Related: `docs/adr-004-modularization-decision.md`,
  `docs/review-findings.md` section 9, `spec-017`, `spec-018`
- Note (2026-11-10): the UI/client split originally queued as `spec-048` is
  renumbered to `spec-049`; `spec-048` covers build performance and the
  desktop feature seam.

## Purpose and scope

This is a behavior-preserving modularization pass. It restores the rule from
ADR-004: split a module when it holds more than one coherent responsibility,
not when it crosses a line count.

The 2026-11-10 review found two root files had absorbed several later feature
specs:

| File | Lines | Responsibilities |
| ---- | ----: | ---------------- |
| `src/server.rs` | 3,458 | contract, v1/v2 dispatch, CRDT/auth/webhook/comment/view/database/block/list use cases |
| `src/model.rs` | 2,442 | rich text, blocks, properties, databases, views, files, comments, users, pages, workspace |

It also found duplicated command classification in `ui/apply.rs`: the same
`Command` variants are listed again in the `force_flush` and `blocked` matches,
so one new variant needs three coordinated edits.

In scope:

- `server.rs` -> facade + `server/routes/*` + `server/handlers/*` +
  `server/response.rs`.
- `model.rs` -> facade + `model/{rich_text,block,property,database,view,file,
  comment,user,page,workspace}.rs`.
- `Command::forces_flush` and `Command::blocked_when_locked` as the single
  source for those two classifications.
- Documentation and changelog updates that describe the new map.

Out of scope:

- No behavior, protocol, persistence, DTO, visual or public-name change.
- No new crate, no `domain/ application/ infrastructure/` scaffold.
- No use-case extraction between v1 and v2 (scheduled as spec-047).
- No settings-page, page-column or client/retry split (scheduled as spec-048).

## Evidence

- `docs/review-findings.md` section 9 (RF-M01, RF-M02, RF-M03).
- `docs/adr-002-architecture-guidelines.md`: private by default, `pub use`
  facade, modules are units of visibility.
- `docs/adr-004-modularization-decision.md`: split by responsibility.
- `docs/architecture.md` section 10 already names the planned `model/*` files.
- `spec-017` and `spec-018` established the compile-failure red step for pure
  refactors and rely on the existing integration suite as the behavior
  contract.

## Acceptance criteria

1. **Given** the server module, **when** the split is complete, **then**
   `src/server.rs` contains only the module facade, transport-neutral DTOs,
   `WorkspaceApi`, the `WorkspaceServer` state/constructors/persistence; the
   `WorkspaceApi` implementation lives in `server/routes.rs`; and every moved
   handler still answers the same request with the same status and body.
2. **Given** the model module, **when** the split is complete, **then**
   `src/model.rs` declares and re-exports the domain modules and contains only
   the time helpers, and every `ion_of_notes::model::*` name still resolves.
3. **Given** the `ui::Command` enum, **when** a variant is added, **then** the
   flush and lock decisions live in one `impl Command`, not in `apply_commands`.
4. **Given** the full suite, **when** `cargo build`, `cargo test --all-targets`,
   `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` run,
   **then** all are green.
5. **Given** the default window, **when** `--screenshot` runs, **then** the
   picture is unchanged.
6. **Given** the documentation map, **when** the split is complete, **then**
   `AGENTS.md`, `docs/architecture.md`, `docs/contributor-guide.md`,
   `docs/api-contract.md` and `docs/glossary.md` point at the new locations.

## Test scenarios (written before implementation)

- Existing 252 integration tests are the behavior contract; no test semantics
  change.
- Red step: add the new `mod` declarations and `pub use` lines before moving the
  code, run `cargo test --all-targets`, and observe the compile failure.
- After the move: `tests/server.rs`, `tests/contract.rs`, `tests/blocks.rs`,
  `tests/properties.rs`, `tests/views.rs`, `tests/comments.rs`,
  `tests/webhooks.rs`, `tests/limits.rs`, `tests/realtime.rs`, `tests/crdt.rs`,
  `tests/model.rs`, `tests/ui.rs` and the remaining crates exercise the same
  public entry points.
- Clippy and rustfmt stay clean on every changed file.
- The default screenshot matches the pre-refactor window.

## Design

```text
src/server.rs                 facade + Method/ApiRequest/ApiResponse/Envelope,
                              WorkspaceApi, WorkspaceServer state/config and
                              persistence
src/server/routes.rs          WorkspaceApi impl, module facade and dispatch
src/server/routes/v1.rs       legacy v1 dispatch
src/server/routes/v2.rs       v2 version/auth/limit/capability pipeline
src/server/routes/capabilities.rs  required capability per method + path
src/server/handlers/          one module per resource family
src/server/response.rs        Envelope/json/error helpers and v1 error mapping

src/model.rs                  facade: declarations, re-exports, time helpers
src/model/rich_text.rs        rich text, mentions, equations
src/model/block.rs            block kinds, blocks, block DTOs
src/model/property.rs         typed properties, values, option/page refs
src/model/database.rs         databases, data sources, table rows
src/model/view.rs             views, filters, sorts, grouping
src/model/file.rs             file objects, uploads and attachments
src/model/comment.rs          comments and comment DTOs
src/model/user.rs             people, users, bots and user references
src/model/page.rs             pages, summaries, drafts, errors
src/model/workspace.rs        workspace tree behavior and deserialization
```

Visibility stays private by default. Cross-module helpers use
`pub(super)`; public names are re-exported from the module root so `lib.rs`
and integration tests do not change.

## Plan / tasks

- [x] Write this spec and record the red compile step.
- [x] Add the server submodule declarations and move dispatch/response/handlers.
- [x] Add the model submodule declarations and move the domain groups.
- [x] Add `Command::forces_flush` / `Command::blocked_when_locked` and consume
      them from `apply_commands`.
- [x] Update `CHANGELOG.md`, `AGENTS.md`, `docs/architecture.md`,
      `docs/contributor-guide.md`, `docs/api-contract.md`,
      `docs/glossary.md` and `docs/review-findings.md`.
- [x] Run build, tests, clippy, fmt and the screenshot check.
- [x] Commit the whole `app/` tree with a message naming this spec.
