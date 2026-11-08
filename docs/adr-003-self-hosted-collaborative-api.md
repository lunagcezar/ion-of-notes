# ADR-003 — Decided API for the self-hosted collaborative server

**Status:** Accepted (API decision; loopback slice implemented, production
adapters staged across specs)
**Date:** 2026-11-05
**Last implementation review:** 2026-11-08 (spec-029)
**Public API alignment:** refined by `docs/adr-006-public-api-alignment.md`
**Supersedes / refines:** the "defer" verdicts in `docs/adr-001-stack-evaluation.md`
(accounts, database, WebSocket, CRDT, auth) — those are now **chosen as the
product direction**, still introduced behind the existing trait seams.

## Context

The person changed the product direction: the app is no longer a single-user,
local-only reconstruction. It must become a **client/server system** with a
**self-hosted server** that has:

- **accounts** (people who sign in),
- **synchronization of edited documents** across devices,
- **sharing** (invite people to a page/workspace with permissions),
- **real-time editing** like the reference product.

The person explicitly asked: *"we need to decide the API we need to use"*. The
reference pictures (29 at the time; 98 today) show the surfaces this API must
feed: a Share popover
(invite by email, "Full access", "People invited", "General access"), a People
settings page ("Members", "Guests", "Workspace owner"), an Export settings
page ("Workspace content", "Members ... as CSV"), and an "Invite members" card
in the sidebar. The described workspace export shows a page tree of markdown
files, "folder" pages that link to their children, and a `People` folder that
records membership/access.

Constraints carried over from `AGENTS.md`: clean room, Rust + eframe/egui,
small dependency set, **DRY / Clean Code / SOLID / modularization**, domain
independent of GUI and infrastructure, and the existing seams
`WorkspaceApi` (transport) and `WorkspaceStore` (persistence).

## Decision

**Two coordinated transports behind the `WorkspaceApi` seam:** a REST/JSON
request–response API for commands and queries, and a WebSocket channel for
real-time collaboration. One documented contract, one set of DTOs, one
`Client<A: WorkspaceApi>` — no duplicated protocol (DRY). The loopback
`std::net` listener remains the v1 adapter; a production self-hosted server
(Axum + SQLx/PostgreSQL + WebSocket + Yrs) is added **behind the same seams**
in later specs, not by rewriting the domain (Open/Closed, Dependency
Inversion).

### 1. Base shape

- Base URL: `https://<self-hosted-host>/api/v1` (TLS terminates at the
  operator's reverse proxy; the app itself needs no certificate code).
- Every REST response is the existing envelope
  `Envelope<T> { data: Option<T>, error: Option<String> }` with standard
  status codes (`200`, `201`, `400`, `401`, `403`, `404`).
- **Auth:** self-hosted accounts with own email/password → short-lived
  session token sent as `Authorization: Bearer <token>`. No third-party OAuth2
  in v1 (the operator owns identity). `401` for a missing/invalid token,
  `403` for a valid token without the required permission. Auth is a
  cross-cutting concern at the transport adapter, never in the domain.

### 2. Decided REST endpoints (the API to use)

| Method | Path | Purpose | Body → response |
| ------ | ---- | ------- | --------------- |
| `GET` | `/health` | Liveness | `200` |
| `POST` | `/api/v1/session` | Sign in (account) | `{email,password}` → `{token,member}` |
| `GET` | `/api/v1/workspace` | Full snapshot (cold start) | `Workspace` |
| `PUT` | `/api/v1/workspace` | Replace snapshot | `Workspace` |
| `GET` / `POST` | `/api/v1/pages` | List summaries / create page | `PageDraft` → `Page` |
| `GET` / `PATCH` / `DELETE` | `/api/v1/pages/{id}` | Read / update / delete (subtree) | `Page` / ids |
| `GET` | `/api/v1/search?q=` | Ranked search | `Vec<SearchResult>` |
| `GET` / `POST` | `/api/v1/members` | People directory / invite member | `MemberDraft` → `Member` |
| `GET` / `PUT` | `/api/v1/shares` | Read / set permission (per page or general) | `Share` → `Share` |
| `GET` | `/api/v1/sync?since=` | Versioned changes after a cursor | `SyncBatch` |
| `GET` | `/api/v1/export` | Workspace export tree | `Vec<ExportFile>` |
| `GET` | `/api/v1/realtime` | Discover the real-time channel contract | `RealtimeContract` |
| `WS` | `/ws/workspace/{id}?token=` | Real-time co-editing channel | `RealtimeEvent` frames |

The DTOs (`Member`, `Role`, `MemberDraft`, `Share`, `Access`, `Change`,
`ChangeOp`, `SyncBatch`, `RealtimeEvent`, `RealtimeKind`,
`RealtimeContract`, `ExportFile`) live in `src/collab.rs` (one cohesive
domain module, SRP). The v1 loopback server already exposes the unversioned
equivalents (`/api/members`, `/api/shares`, `/api/sync`, `/api/export`,
`/api/realtime`) so the contract is real and testable today; `/api/v1` is the
documented production shape.

### 3. Accounts, roles and sharing (maps to the People folder + Share popover)

- `Member { id, name, email, role }`; `Role ∈ {Owner, Member, Guest}`. A
  person's export file is `# Name` plus `Membership Type: <role>` — exactly
  the "Workspace owner" line in the described export.
- `Share { page_id, subject, access }`; `page_id == "*"` is a workspace-wide
  ("General access") rule; `Access ∈ {Full, Edit, View, None}`. Effective
  access resolves most-specific-first (member → page-general →
  workspace-general). The server must check `can_write` before any mutation.
- The `People` directory is exported as a folder markdown linking to person
  files, a `People_all.csv`, and a `filters:`/`sort:` view definition.

### 4. Synchronization of edited documents

- **Cold start:** `GET /api/v1/workspace` returns the full snapshot.
- **Incremental:** every mutation appends a `Change { seq, page_id, op,
  actor }` to a versioned log (`ChangeOp ∈ {Create, Update, Delete,
  Replace}`). Clients poll `GET /api/v1/sync?since=<cursor>` and receive
  `SyncBatch { cursor, changes }`; a client caught up to `cursor` gets an
  empty batch. This is already implemented by `CollabState::changes_since`
  and the single `WorkspaceServer::write` path (DRY).
- Offline edits are queued on the client and replayed against the cursor;
  conflicts are resolved by the real-time CRDT layer below, not by last-write-
  wins on whole documents.

### 5. Real-time editing

- Transport: **WebSocket** (`wss` behind the proxy), channel
  `/ws/workspace/{id}?token=`. Frame payload: `RealtimeEvent { kind,
  page_id, actor, payload }` with `RealtimeKind ∈ {Presence, Edit, Awareness,
  Sync}`.
- Conflict resolution: a **CRDT per block** (Yrs/automerge-style, the `Yrs`
  choice from ADR-001). The server merges concurrent edits, persists the
  merged snapshot through `WorkspaceStore`, and broadcasts the new
  `SyncBatch` cursor. Presence/awareness (who is on a page, cursor
  positions) is ephemeral and not persisted.
- `GET /api/v1/realtime` returns the `RealtimeContract` so clients do not
  hard-code the channel (single source of truth, DRY).

### 6. Export format (takes the described Notion export into account)

`GET /api/v1/export` returns `Vec<ExportFile>` (in-memory; no filesystem
dependency in the domain). The tree mirrors the example the person supplied:

- one markdown file per page, named `Title <id>.md`;
- a page with children is a **folder**: its own markdown lists links to the
  child markdown files (URL-encoded `%20` paths), and the children live in a
  same-named directory;
- the workspace root is itself a folder page linking to top-level pages;
- a `People` folder holds the membership/access files and a `_all.csv`;
- inline tables emit a companion `_all.csv`.

See `export_tree`, `page_markdown`, `people_markdown` and `people_csv` in
`src/collab/export.rs` (re-exported by `src/collab.rs`).

### 7. Server-side stack (deferred, behind the seams)

| Concern | v1 (today) | Self-hosted production (later spec) |
| ------- | ---------- | ----------------------------------- |
| HTTP | `std::net::TcpListener` loopback | Axum implementing `WorkspaceApi` |
| Store | `JsonStore` (`WorkspaceStore`) + `JsonCollabStore` (`CollabStore`) | `SqlxStore` / PostgreSQL for both ports |
| Real-time | contract exposed as data; no socket server | WebSocket + Yrs CRDT |
| Auth | seeded demo identity; loopback only | own sessions + password hashes, Bearer tokens |
| Deploy | `--serve 127.0.0.1:8787` | operator-run process behind TLS reverse proxy |

No production dependency is added by this ADR; each production adapter
replaces or adds an implementation of an existing port (Liskov Substitution,
Open/Closed).

## Implementation status (2026-11-08)

| Capability | Status |
| ---------- | ------ |
| Workspace/page CRUD, search, envelopes | Implemented (unversioned loopback routes) |
| Members, shares, sync log | Implemented; persisted in `collab.json` (`spec-019`) |
| Page versions and restore | Implemented (`GET /api/pages/{id}/versions`, 50/page, `spec-022`) |
| Export tree | Implemented in `src/collab/export.rs` (pure, in memory) |
| Realtime contract | Implemented as data at `GET /api/realtime` |
| Per-page writes + debounce | Implemented (`spec-020`) |
| `/api/v1` prefix | Decided; not served |
| Sessions/password hashing/Bearer auth | Decided; not implemented |
| WebSocket/CRDT server | Decided; not implemented |
| PostgreSQL/SQLx | Decided; not implemented |
| Pagination/sorting | Not implemented; full snapshots |

The exact implemented routes, DTOs and status codes are documented in
`docs/api-contract.md`.

## Consequences

- The "free & open for all / no accounts" framing is retired: the app is
  **open source** (MIT) as a license fact, and **accounts are required** for a
  self-hosted multi-user server. The UI no longer advertises a free edition.
- `WorkspaceServer` owns a `CollabState`; `spec-019` added the dedicated
  `CollabStore` port and `JsonCollabStore`, so members, shares, the sync log and
  versions are durable in `collab.json`. The loopback server remains
  unauthenticated and local-only; auth and the WebSocket/CRDT server are still
  staged behind the same seams.
- The domain (`model`, `collab`) still knows nothing about egui, sockets or
  files; adapters implement ports. `Client<A>` works unchanged with the
  in-process server and, later, a remote HTTP/WS transport.
- This ADR is the "decide the API" answer; `spec/spec-011` scopes the first
  implemented slice and records the Given/When/Then acceptance criteria.
- **Public API refinement (2026-11-08):** ADR-006 compares this contract with
  the public Notion API and schedules a `/api/v2` alignment (versioning,
  structured errors, capabilities, pagination, rich text, properties, data
  sources, views, comments, files, webhooks, limits). v1 remains stable; the
  decisions here are not replaced.
