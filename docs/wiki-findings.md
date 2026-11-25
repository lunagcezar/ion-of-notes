# DeepWiki findings - ion-of-notes

This document records what the public DeepWiki pages for this repository add to
the project documentation, what was adopted, and what was stale or wrong.

- **Wiki:** <https://deepwiki.com/lunagcezar/ion-of-notes>
- **Base commit named by the request:** `75a4587ddaeff0fd2cb4cd0d9c22410c4d03dbc5`
  (spec-149, tagged-release checksums).
- **Retrieved:** 2026-11-25 through the DeepWiki MCP server
  (`read_wiki_structure` + `read_wiki_contents`).
- **Nature of the source:** AI-generated documentation from the repository at
  that commit. It is useful as an independent reading of the architecture, but
  it is not authoritative over code, tests or `spec/`. Its index can include a
  few later changes and omit others; every claim below was re-checked against
  the current tree.

## 1. Wiki structure and where each finding lives

| Wiki page | Repository owner of the same topic | Finding used here |
| --------- | ---------------------------------- | ----------------- |
| Overview | `README.md`, `docs/architecture.md` | The client/server split and modular-monolith framing are accurate. |
| Getting Started and CLI | `README.md`, `src/cli.rs` | The feature-flag/binary matrix (desktop vs server-only) is worth stating explicitly. |
| Architecture and Layering | `docs/architecture.md`, `AGENTS.md` | Add a ports/adapters table and the composition-root responsibilities. |
| Spec-Driven Development | `AGENTS.md`, `spec/README.md`, `docs/testing.md` | SDD/TDD and changelog traceability match the repository rules. |
| Domain Model; Entities; DTOs | `docs/data-model.md`, `src/model/*` | The entity grouping matches; no correction needed. |
| Search and Indexing | `src/model/search.rs`, `src/model/index.rs` | The incremental search cache and verified block→page index are correct. |
| Request Pipeline and Routing | `src/server/routes/*`, `docs/api-contract.md` | The pipeline stage order (version → auth → limits → capability → authorization) is correct. |
| API v2 Handlers and Use Cases | `src/server/handlers/*`, `src/server/usecases/*` | The per-family handler/use-case split is accurate. |
| HTTP/WebSocket Web Adapter | `src/server/web/*` | Add the browser UI and its headers to the adapter description. |
| Authentication and Authorization | `docs/security-hardening.md`, `docs/authorization.md` | Confirms the dummy-hash enumeration defense, session cache and page-access order. |
| Realtime, CRDT and Collaboration | `docs/architecture.md`, `src/server/realtime.rs`, `src/crdt_text.rs` | Confirms Yrs per-page documents, change feed, presence and single-use tickets. |
| Webhooks and Domain Events | `docs/persistence-integrity.md`, `docs/security-hardening.md` | Confirms the durable outbox, HMAC, backoff/dead-letter and SSRF pinning. |
| Persistence and Stores | `docs/persistence-integrity.md`, `src/store/*` | Correct the "whole-document JSON" claim: pages and blocks are normalized in SQLite. |
| Typed Client API | `src/client/*`, `docs/api-contract.md` | The resource modules, error mapping, retry and cursor pagination are accurate. |
| Remote Session and Transport | `src/remote/*`, `docs/testing.md` | The session cache, background worker and bounded awareness channel are accurate. |
| App State and Command Dispatch | `src/ui/app.rs`, `src/ui/apply/*` | Confirms patch-based undo and command-family routing. |
| Rendering: Chrome, Sidebar and Content | `src/ui/render/*` | Confirms sidebar flattening/clip culling and the Fenwick height index. |
| Dialogs, Settings and Sharing | `src/ui/dialogs/*` | Confirms the shared switcher, settings pages and icon picker. |
| Editing, Blocks and Comments | `src/ui/apply/blocks.rs`, `src/ui/comments.rs` | Confirms height estimation, shortcuts, inline threads and CRDT editing. |
| Testing Strategy | `docs/testing.md` | Confirms the pyramid, two feature passes, determinism and gates. |
| Security Hardening | `docs/security-hardening.md` | Confirms the store matrix, proxy trust, egress rules and request bounds. |
| Performance Engineering | `docs/performance-roadmap.md` | Confirms caching/indexing strategy; note that v2 reads now use a published snapshot too. |
| Deployment and CI/CD | `docs/deployment.md` | Confirms the image, Compose, TLS and release layout. |
| Glossary | `docs/glossary.md` | Add the terms introduced by the wiki (ReadView, snapshot, ticket, outbox). |

## 2. Adopted in the repository documentation

- `docs/architecture.md`: ports/adapters table, composition-root responsibilities,
  read snapshot path and browser UI module.
- `docs/refactoring-roadmap.md`: normalized SQLite moved from "deferred" to
  "implemented" (per-block rows and `persist_block`, spec-133); remaining
  deferred item is the auth/webhook service split.
- `docs/performance-roadmap.md`: SQLite normalizes pages and blocks while
  metadata/collaboration remain JSON documents; HTTP writes publish an
  `Arc` snapshot (spec-151); added the tracing-span list.
- `docs/glossary.md`: DeepWiki, ReadView, WorkspaceSnapshot, single-use ticket,
  outbox, awareness, Fenwick height index.
- `AGENTS.md`: external-wiki reviews now have a documented home and a maintenance
  rule.

## 3. Findings reconciled as stale

| Wiki statement | Current reality |
| -------------- | --------------- |
| "Normalized SQLite schema is deferred; workspaces are whole-document JSON snapshots." | SQLite stores page rows and one `workspace_blocks` row per block; single-block writes use `persist_block` (spec-133). Only metadata and collaboration documents remain JSON. |
| "Reads clone the collaboration state under the v1 lock; v2 reads still serialize." | v1 and the heavy v2 GETs project from a versioned snapshot; HTTP writes publish it (spec-150, spec-151). Writes remain serialized. |
| "The WebSocket token may be passed in the query string." | `?token=` is rejected; the socket accepts a Bearer header or a single-use `?ticket=` (spec-148). |
| "No registration, password reset or browser UI." | Account registration/change/owner reset and `/app` browser UI exist (spec-152, spec-153). |
| "Patch-based undo and sidebar virtualization are deferred." | Both are implemented (spec-146, spec-147). |

## 4. Open limitations the wiki agrees with

- Writes still run through one workspace mutex; reads are decoupled, writes are
  not.
- The WebSocket hub and change feed are per process; multi-instance realtime
  needs a shared bus or sticky routing.
- Email verification, self-service reset tokens and email delivery are absent;
  registration is invite-first and reset is owner-driven.
- The browser UI does not wire WebSockets yet; a reload shows the latest state.
- There is no external security audit.
- `server/http.rs` remains a bounded parser used by tests and `serve_once`; the
  serving path is Axum.

## 5. Maintenance

1. Re-read the wiki with the DeepWiki MCP server:
   `read_wiki_structure` then `read_wiki_contents` for
   `lunagcezar/ion-of-notes`.
2. Record the wiki commit/date at the top of this file.
3. Re-check every claim against the current tree; do not copy wiki text as a
   requirement.
4. Move adopted findings into the owning document (architecture, security,
   testing, performance) and keep this page as the reconciliation log.
5. Add a spec and a `CHANGELOG.md` entry when the review changes documentation
   contracts.
