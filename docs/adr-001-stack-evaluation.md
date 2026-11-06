# ADR-001 — Evaluation of the suggested library list

**Status:** Accepted (evaluation only; no dependency changes)
**Date:** 2026-11-05
**Context:** The person shared a list of libraries suggested by ChatGPT and
asked whether we agree with the choices. The app is a clean-room, non-profit,
local-first desktop reconstruction: one user, no accounts, no cloud services,
all state in a JSON file, optional loopback HTTP listener (`spec-007`,
`docs/requirements.md` NFR-04).

> **Update (2026-11-05):** The person later changed the direction to a
> **self-hosted client/server with accounts, sync, sharing and real-time
> editing**. `docs/adr-003-self-hosted-collaborative-api.md` now **decides** the
> API and turns the "Defer" verdicts below (Axum, PostgreSQL, SQLx, WebSocket,
> Yrs) into the chosen product direction — still introduced behind the same
> trait seams, with no dependency added in that session. The "Decline" verdict
> for OAuth2 / own sessions is superseded: own-session **accounts are now
> required** (ADR-003 §1). The "free & open for all / no accounts" framing that
> motivated the original verdicts is retired.

## Decision

We agree with **Rust** and **Serde** (already in use). We **defer** the
server/database/collaboration choices behind existing or new trait seams, and
we **decline** the UI replacements and authentication for this scope. None of
the libraries are added to `Cargo.toml` in this session: adding them now would
break the offline, dependency-light, account-free, single-user design and
would require replacing the renderer that already matches the 29 pictures in
`reference/`.

| Suggested choice                    | Verdict | Rationale |
| ----------------------------------- | ------- | --------- |
| Rust (main language)                | Agree   | Already the language; stable 2024 edition, `cargo build`/`cargo test`. |
| Slint (native UI)                   | Decline | Swapping egui/eframe for Slint is a full rewrite of the window, not a modularization. It would discard the dark layout already matched to the reference pictures. A native-UI alternative can be evaluated later behind the `Client` seam. |
| Tauri (desktop alternative)         | Decline | Webview/Node toolchain adds a browser engine and JS runtime, contradicts NFR-04 (no runtime network/services) and the clean-room "no web view" constraint in `AGENTS.md`. |
| Axum (API)                          | Defer   | Reasonable for a real HTTP service. Today the `WorkspaceApi` trait already decouples the API contract from its transport; `serve_once`/`serve_forever` is the loopback adapter. An Axum adapter can implement the same contract later without touching the domain or the client. |
| PostgreSQL                          | Defer   | A relational server is useful for multi-user, remote workspaces. This app is single-user and offline with JSON persistence (FR-30, NFR-04); a DB server would add an external runtime dependency and an install step. |
| SQLx                                | Defer   | Goes with PostgreSQL. The new `WorkspaceStore` port (`spec-008`) is exactly the seam where a `SqlxStore` adapter can be added later without editing `WorkspaceServer`. |
| WebSocket (real time)               | Defer   | Real-time transport is only needed once collaboration exists; it is out of scope in `docs/requirements.md` §4. The `WorkspaceApi` seam keeps a future socket transport possible. |
| CRDT / Yrs                          | Defer   | Correct technology for conflict-free co-editing, but there is one local user, so there are no concurrent edits to merge. Adding it now would add a heavy dependency with no user-visible feature. |
| Serde (serialization)               | Agree   | Already used for the JSON envelope, the data model, and persistence. |
| OAuth2 / own sessions (auth)        | Decline | `spec-007` and FR-35 forbid accounts, sign-in, billing, and paywalls: the app is free and open for all with no user identity. Authentication would directly contradict an accepted requirement. |

## Consequences

- `Cargo.toml` stays small; `cargo build` needs no external services.
- The architecture is intentionally prepared for later adoption: `WorkspaceApi`
  (transport port), `WorkspaceStore` (persistence port), `Client<A>` (generic
  data access), and `ui -> client -> server -> model/store` dependency
  direction. This is the Open/Closed and Dependency Inversion principles in
  practice: new adapters are added, core modules are not rewritten.
- **Revisited:** the product did become multi-user — `docs/adr-003-*` decides
  the accounts, database, sync, real-time and authentication API, and
  `spec/spec-011` implements the first tested slice behind these seams.
