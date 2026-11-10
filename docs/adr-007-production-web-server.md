# ADR-007 — Production web server stack

- **Status:** Accepted (2026-11-09)
- **Deciders:** project owner
- **Related:** ADR-003 (self-hosted API), ADR-006 (public API alignment),
  spec-038 (Axum adapter), spec-039 (sessions and durable storage)

## Context

The optional `--serve` mode used a hand-written HTTP/1.1 loopback listener. It
was enough for tests and a local demo, but it had no graceful shutdown,
structured logging, Tower middleware, WebSocket path or TLS story. The
application core already exposes a transport-neutral `WorkspaceApi` seam and a
`WorkspaceServer` with all routing, versioning, auth, capabilities, limits and
webhooks.

The server must remain self-hosted, local-first and deployable as one binary.
The desktop client must keep working without a running server. A framework must
not leak into `model`, `client`, `collab` or the core `server` logic.

## Decision

Use **Axum + Tokio + Tower/Tower-HTTP** as the production HTTP adapter.

- `src/server/web.rs` owns the Axum router and `serve_axum`.
- The adapter converts `http::Request` into `ApiRequest`, calls
  `WorkspaceApi::handle` in `spawn_blocking` behind a mutex, and converts
  `ApiResponse` back to HTTP.
- The hand-written `server/http.rs` listener remains for parser/formatter tests
  and as a minimal fallback; `--serve` uses Axum.
- `tower-http` provides tracing and a transport body limit; the v2 size checks
  remain the source of truth for the documented `400 validation_error`.
- `X-Request-Id` and `Retry-After` are copied from the existing response model.
- The synchronous core stays synchronous for this slice. Typed async
  application services and repository traits are a later refactor, not a
  prerequisite.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| Actix Web | Mature and fast, but a different ecosystem and actor-style coupling; Axum fits Tower and the existing seam better. |
| Rocket | Ergonomic, but less momentum for the WebSocket/worker roadmap. |
| Warp | Filter composition becomes awkward for a large API. |
| Hyper directly | Reimplements routing, extractors, middleware, WebSocket handshake and error mapping for little gain. |
| Loco | Batteries-included, but opinionated structure can fight the ports/adapters design. |
| Go (Chi/Gin) | Simple deployment, but duplicates the domain and loses shared Rust types with the client. |
| Node/TypeScript | Fast iteration, but adds a runtime and type drift from the Rust client. |
| Supabase/Appwrite/PocketBase | Own the schema and API; conflicts with the custom v2 contract and clean-room goals. |
| Framework-free Rust | More control, much more boilerplate; Axum is already pure Rust. |

## Consequences

Positive:

- One Rust toolchain, one binary and shared types with the desktop client.
- Tower middleware, tracing, body limits and graceful shutdown are available.
- The `WorkspaceApi` seam keeps the core testable without a socket.
- A clear path to WebSocket, SQLx, sessions and webhook workers.

Costs:

- Axum/Tokio/Hyper add compile time and dependency surface.
- The synchronous core needs `spawn_blocking` and a global mutex until typed
  async services replace it.
- The server remains single-node until the SQL and shared rate-limit adapters
  land.

## Follow-up

- spec-039: **implemented** — sessions, Argon2id, SQLite behind the store
  ports and JSON migration.
- spec-040: WebSocket/CRDT transport.
- spec-041: **implemented** — TLS webhook delivery and shared rate limiting.
- Split hot paths into typed async application services when the global mutex
  becomes the bottleneck.
