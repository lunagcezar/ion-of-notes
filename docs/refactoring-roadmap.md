# Refactoring roadmap

This document records the general refactoring pass and the work that is
deliberately deferred. It complements `docs/architecture.md` and
`docs/review-response.md`.

## 1. Implemented in this pass

| Refactor | What changed | Spec |
| -------- | ------------ | ---- |
| Central route policy | One `RoutePolicy` table owns the capability, block-limit kind, owner-only flag and page-scoped flag. `route_capability`, the block-limit calculation and the v2 authorization checks read it. | `spec-078` |
| Shared CRDT text helpers | `src/crdt_text.rs` owns the block-tree walk, text collection and projection used by the server and desktop CRDT sessions. `walk_blocks` / `walk_blocks_mut` live in the model. | `spec-079` |
| Sidebar decomposition | `render_sidebar` is a small shell; customize mode, the default list and the bottom group are separate functions. | `spec-080` |

## 2. Deferred: server request layer

These are the highest-value remaining refactors. They are mechanical but touch
many handlers, so they should be done one resource family at a time with the
existing tests as the safety net.

| Item | Why it is deferred | Suggested first slice |
| ---- | ------------------ | --------------------- |
| Typed handler errors | 140 `v2_error` calls and repeated `parse_body` matches need a new `ApiResult`/`HandlerError` type and a change to every handler signature. | Convert `content.rs` comments, then databases/views. |
| Pipeline stages | `handle_v2_inner` runs version, auth, rate limit, limits, conflict, capability and dispatch in one function. | Extract one `fn stage(&mut self, ctx) -> Result<(), ApiError>` per stage, starting with rate limiting and auth. |
| Service split | `WorkspaceServer` owns state, auth, webhooks, limits, realtime and CRDT. | Extract `AuthService` and `WebhookService` first; keep `WorkspaceServer` as a facade. |
| Domain events | Handlers call `emit_webhook`, `write` and `persist_collab` by hand. | Add an event list to the write path; persist/change-feed/webhook subscribers. |
| `WorkspaceApi` segregation | The trait mixes request handling with CRDT, realtime and awareness. | Introduce `RequestTransport` and `CollabTransport`; keep a composite alias for the desktop client. |
| Store ports | `WorkspaceStore` and `CollabStore` have the same shape and separate null adapters. | Only collapse them if a second backend needs the generic form; avoid an abstraction for its own sake. |
| Second HTTP adapter | `server/http.rs` is a hand-written HTTP/1.1 parser kept for tests and `serve_once`. | Check every caller; move it under a test-only module or keep it with an explicit compatibility note. |

## 3. Deferred: client and UI

| Item | Why it is deferred | Suggested first slice |
| ---- | ------------------ | --------------------- |
| Resource-oriented client | `client.rs` is a large facade over v1/v2, pages, blocks, databases, views, comments, files and webhooks. | Extract `client/pages.rs` and `client/comments.rs` with `impl<A: WorkspaceApi> Client<A>` blocks. |
| Shared response decoding | `call` and `call_v2` duplicate success decoding. | Extract `decode_success<T>` and keep protocol-specific error handling separate. |
| Composable app state | `NotionApp` owns navigation, editor, search, sharing, settings, AI, collaboration, server login and window state. | Group state by lifecycle (`SearchState`, `SharingState`, `CollaborationState`, …) without changing the command flow. |
| Narrow view models | `ViewState` is passed to every renderer. | Split it into feature views and pass only the relevant view. |
| Command grouping | `Command` has many variants; `apply_*` matches the whole enum. | Group into `SettingsCmd`, `SidebarCmd`, `BlockCmd`, matching the existing `apply/*` families. |
| Icon catalog as data | `icon_catalog.rs` is large data. | Move it to a data file loaded with `include_str!` if compile time or navigation becomes a problem. |

## 4. Deferred: domain and infrastructure

| Item | Why it is deferred | Suggested first slice |
| ---- | ------------------ | --------------------- |
| DTO/domain separation | `model` exports domain entities and API DTOs together. | Add v2 mapping functions for the resource whose contract is most likely to evolve. |
| Transport split | `remote.rs` combines session persistence, HTTP, WebSocket, CRDT and reconnection; `server/web.rs` combines router, metrics and WebSocket. | Extract `remote/session.rs` and `server/web/websocket.rs`. |
| Normalized SQLite | Workspace and collaboration are JSON snapshots in one transaction. | Measure first; add per-page rows only when the snapshot cost is proven. |
| PostgreSQL/Redis | SQLite is intentionally single-node. | Keep the ports; add adapters only when multi-node is a real requirement. |

## 5. Rules for the next refactor

1. **Keep the external behavior unchanged.** Run `cargo test --all-targets`
   and the desktop tests after every slice.
2. **One resource family per change.** Do not mix comments, databases and
   views in one commit.
3. **Do not add a trait until there is a second implementation or a test
   seam.** The review explicitly warns against a repository per noun.
4. **Prefer moving code over rewriting it.** The route table and CRDT helpers
   were moved, not redesigned.
5. **Update `docs/architecture.md` and this roadmap** when a deferred item is
   completed.

## 6. Measurement

Track the indicators the review suggested:

- business logic left in HTTP handlers;
- duplicate protocol/response code;
- public mutable server state;
- runtime-unavailable trait methods;
- features that require edits across unrelated modules;
- test time and compile time after each slice.

Do not use file count or line count as the only measure.
