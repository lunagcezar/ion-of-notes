# Refactoring roadmap

This document records the general refactoring passes and the work that is
deliberately deferred. It complements `docs/architecture.md` and
`docs/review-response.md`.

## 1. Implemented

| Refactor | What changed | Spec |
| -------- | ------------ | ---- |
| Central route policy | One `RoutePolicy` table owns the capability, block-limit kind, owner-only flag and page-scoped flag. `route_capability`, the block-limit calculation and the v2 authorization checks read it. | `spec-078` |
| Shared CRDT text helpers | `src/crdt_text.rs` owns the block-tree walk, text collection and projection used by the server and desktop CRDT sessions. `walk_blocks` / `walk_blocks_mut` live in the model. | `spec-079` |
| Sidebar decomposition | `render_sidebar` is a small shell; customize mode, the default list and the bottom group are separate functions. | `spec-080` |
| Pipeline stages | Version, login throttle, authentication, runtime limits, capability and authorization are separate functions in `server/routes/pipeline.rs`; `handle_v2_inner` is a short `?` sequence plus dispatch. | `spec-082` |
| Transport segregation | `RequestTransport` owns `handle`; `CollabTransport` owns CRDT/realtime/awareness; `WorkspaceApi` is the composite. `Client` request methods need only `RequestTransport`. | `spec-083` |
| Resource-oriented client | `src/client/` is split into pages, sharing, blocks, databases, views, comments, webhooks, collab and the shared request core. | `spec-084` |
| Transport adapter split | `server/web/` is split into router/state, HTTP translation, operations and WebSocket modules; `remote/` is split into session, HTTP, realtime and CRDT modules. | `spec-085` |
| Typed handler errors | `HandlerError`/`HandlerResult`/`body()` and the comments handler are converted. | `spec-087` |
| Search state/view | Search fields are grouped into `SearchState` and `SearchView`; references use `self.search.*` / `view.search.*`. | `spec-088` |

## 2. Deferred: server request layer

| Item | Why it is deferred | Suggested first slice |
| ---- | ------------------ | --------------------- |
| Typed handler errors | All v2 resource adapters now return `HandlerResult<Option<ApiResponse>>`; the dispatcher attaches the request ID once. | — |
| Service split | Auth and webhook logic now live in `server/services/`; the methods are still inherent on `WorkspaceServer`. | Extract owned `AuthService`/`WebhookService` structs and move state into them. |
| Domain events | Handlers still call `emit_webhook`, `write` and `persist_collab` by hand (about 24 sites). | Add an event list to the write path; persist/change-feed/webhook subscribers. |
| Store ports | `WorkspaceStore` and `CollabStore` still have the same shape and separate null adapters. | Do not collapse them until a second backend needs the generic form; an abstraction for its own sake is not worth it. |
| Second HTTP adapter | `server/http.rs` remains a hand-written HTTP/1.1 parser used by tests and `serve_once`. | Keep it as an explicit compatibility adapter or move it under test support after checking every caller. |

## 3. Deferred: UI and domain

| Item | Why it is deferred | Suggested first slice |
| ---- | ------------------ | --------------------- |
| Composable app state | `SearchState`, `SharingState`, `CollaborationState`, `AiState`, `ServerState` and `SettingsState` are grouped; the remaining flat fields are window/navigation state. | — |
| Narrow view models | `SearchView` and `SharingView` are grouped; the renderers still receive `ViewState` for shared fields. | Split the remaining feature views. |
| Command grouping | `Command` still has one large family of variants. | Add `SidebarCmd` and route it through `apply/settings.rs`. |
| DTO/domain separation | `model` still exports domain entities and API DTOs together. | Add v2 mapping for comments or blocks, the resources most likely to evolve. |
| Normalized SQLite | Workspace and collaboration are whole-document JSON snapshots. | Measure load/serialization at 1k/10k/50k blocks first; then add per-page rows or an operation log. |
| Benchmarks/load tests | The suite is functional, not performance. | Add a deterministic benchmark harness or Criterion benches for load, serialize, search and CRDT sync. |

## 4. Rules for the next refactor

1. **Keep external behavior unchanged.** Run the full gate and the desktop
   tests after every slice.
2. **One resource family per change.** Do not mix comments, databases and
   views in one commit.
3. **Do not add a trait until there is a second implementation or a test
   seam.** The review explicitly warns against a repository per noun.
4. **Prefer moving code over rewriting it.** The route table, pipeline, CRDT
   helpers and client modules were moved, not redesigned.
5. **Update `docs/architecture.md` and this roadmap** when a deferred item is
   completed.

## 5. Measurement

Track the indicators the review suggested:

- business logic left in HTTP handlers;
- duplicate protocol/response code;
- public mutable server state;
- runtime-unavailable trait methods;
- features that require edits across unrelated modules;
- test time and compile time after each slice.

Do not use file count or line count as the only measure.
