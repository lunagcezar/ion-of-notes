# Spec index — ion-of-notes

Every feature starts here. A spec is the boundary of one working session:
nothing outside its scope is built in that session. This index is the map; the
individual files are the source of truth for their scope and acceptance
criteria.

## 1. Lifecycle

```mermaid
flowchart TD
    IDEA[Request, bug or review finding] --> WRITE[Write spec-NNN]
    WRITE --> TESTS[Write failing tests]
    TESTS --> RED[Observe red]
    RED --> IMPL[Implement]
    IMPL --> GREEN[Observe green]
    GREEN --> DOCS[Update requirements, data model, ADR, docs]
    DOCS --> SHOT[Compare --screenshot if UI changed]
    SHOT --> CHANGELOG[Add CHANGELOG entry]
    CHANGELOG --> COMMIT[Commit naming the spec]
    COMMIT --> DONE[Status: implemented]
    WRITE --> SPLIT{Too large for one session?}
    SPLIT -->|yes| NEXT[Split into spec-NNN+1]
    SPLIT -->|no| TESTS
```

Status vocabulary:

| Status | Meaning |
| ------ | ------- |
| **implemented** | Code, tests, docs and changelog are complete and committed |
| **in progress** | This session is actively working on it |
| **plan** | A scoped plan; some phases may still be open |
| **retired** | Kept as history; superseded by a later decision |
| **blocked** | Cannot proceed without a new decision or external input |

## 2. Index

| Spec | Title | Status | Notes |
| ---- | ----- | ------ | ----- |
| [001](spec-001-project-foundation.md) | Project foundation, documentation, build skeleton | implemented | Rules, docs, Cargo skeleton |
| [002](spec-002-domain-model.md) | Workspace domain model, page tree, search | implemented | Tests now live in `tests/model.rs` (spec-008) |
| [003](spec-003-workspace-server.md) | Workspace server, API, persistence, loopback | implemented | Core routes and HTTP adapter |
| [004](spec-004-client-layer.md) | API client used by the desktop UI | implemented | `Client<A: WorkspaceApi>` |
| [005](spec-005-desktop-ui.md) | Desktop window, blocks, table, search, theme | implemented | First UI |
| [006](spec-006-screenshot-mode.md) | Screenshot option for visual checks | implemented | `--screenshot` |
| [007](spec-007-free-and-open-for-all.md) | Free and open for all | **retired** | Superseded by ADR-003 (accounts, no marketing card) |
| [008](spec-008-modular-architecture.md) | Modular architecture, separated tests | implemented | Tests moved to `tests/`, `WorkspaceStore` port |
| [009](spec-009-visual-fidelity.md) | Visual fidelity against the reference pictures | implemented | 29 at the time; the set is now 98 plus 72 screenshots |
| [010](spec-010-architecture-guidelines.md) | Architecture guidelines and composition root | implemented | `src/app.rs`, ADR-002 |
| [011](spec-011-collaborative-server.md) | Self-hosted client/server, decided API | implemented | First slice; `CollabStore` closed in spec-019 |
| [012](spec-012-interaction-polish.md) | Frameless window, tabs, shortcuts, Ollama | implemented | `shortcuts.rs`, `ollama.rs` |
| [013](spec-013-interaction-polish-and-modularity.md) | Interaction polish, bugs, modularity | implemented | ADR-004, `switcher`, `page_icon` |
| [014](spec-014-bug-fixes-and-improvements.md) | Bug fixes and improvements | implemented | Batch of editor/settings/UI fixes |
| [015](spec-015-hardening-and-performance.md) | Hardening, missing surfaces, performance | implemented | Settings filter, versions, cache, atomic save |
| [016](spec-016-architecture-follow-up-and-observation.md) | Architecture follow-up and clean-room observation | **plan** | A1–A10 done; B1/B2/B4 done; B3 blocked; B5 pending |
| [017](spec-017-ui-modularization.md) | UI modularization refactor | implemented | `ui/commands`, `view`, `input`, `apply`, `render/*`, `dialogs/*` |
| [018](spec-018-domain-and-infrastructure-modularization.md) | Domain and infrastructure modularization | implemented | `model/*`, `collab/export.rs`, `server/http.rs` |
| [019](spec-019-collaboration-persistence.md) | Collaboration persistence | implemented | `CollabStore`, `collab.json` |
| [020](spec-020-per-page-writes-and-debounce.md) | Per-page writes and debounce | implemented | `PageDiff`, `CommitPlan`, 250 ms |
| [021](spec-021-quality-and-concurrency-baseline.md) | Quality and concurrency baseline | implemented | `ClientError::Encode`, concurrent `--serve`, CI, fmt |
| [022](spec-022-workspace-index-and-version-retention.md) | Workspace index and version retention | implemented | `WorkspaceIndex`, 50 versions/page |
| [023](spec-023-add-members-sharing-dialog.md) | Add members / sharing dialog | implemented | Functional Share popover |
| [024](spec-024-library-views.md) | Library views | implemented | Recents/Favorites/Shared/Private, More menu |
| [025](spec-025-settings-pages.md) | Settings pages | implemented (Sidebar page retired 2026-11-12) | General/People/Offline, persisted settings; sidebar layout lives in customize mode |
| [026](spec-026-page-actions-and-comments.md) | Page actions and comments | implemented | TOC, discussions, archive, copy contents |
| [027](spec-027-onboarding-and-templates.md) | Onboarding and templates | implemented | Purpose modal, starter catalog |
| [028](spec-028-i18n-coverage.md) | Internationalization coverage | implemented | en / pt-BR / eo key tables |
| [029](spec-029-documentation-and-contributor-guide.md) | Documentation and contributor guide | implemented | Documentation hub and review |
| [030](spec-030-api-v2-foundation.md) | API v2 foundation: versioning, errors, capabilities | implemented | `/api/v2`, `ApiError`, capabilities, request IDs |
| [031](spec-031-pagination.md) | Cursor pagination and list responses | implemented | `ListResponse`, opaque cursors, client helpers |
| [032](spec-032-rich-text-and-blocks.md) | Rich text and the block model | implemented | RichText, metadata, children, block endpoints |
| [033](spec-033-page-properties-and-data-sources.md) | Page properties and data sources | implemented | Typed properties, databases, data sources, migration |
| [034](spec-034-views-and-database-layouts.md) | Views and database layouts | implemented | View CRUD, filters/sorts, board/calendar grouping |
| [035](spec-035-comments-users-and-files.md) | Comments, users and files | implemented | Comment CRUD, users, file uploads |
| [036](spec-036-webhooks-and-event-delivery.md) | Webhooks and event delivery | implemented | Subscriptions, signatures, retries, all documented events |
| [037](spec-037-limits-and-operations.md) | Limits, rate limiting and operations | implemented | Rate/size/block limits, retry policy, operational errors |
| [038](spec-038-axum-web-server-and-contract-suite.md) | Axum web server and contract suite | implemented | Axum/Tokio adapter, B5 contract suite, graceful shutdown |
| [039](spec-039-sessions-and-sqlite-storage.md) | Sessions, Argon2id and SQLite storage | implemented | SQLite ports, login/logout/session, JSON migration |
| [040](spec-040-realtime-websocket-change-feed.md) | Real-time WebSocket change feed and presence | implemented | WS auth, backlog resume, change broadcast, presence |
| [041](spec-041-tls-webhooks-and-shared-limits.md) | TLS webhooks and shared rate limits | implemented | ureq/rustls sender, SQLite rate-limit store |
| [042](spec-042-yrs-crdt-merge.md) | Yrs CRDT text merge | implemented | Per-page Yrs text, WebSocket sync/update, projection |
| [043](spec-043-client-yrs-and-awareness.md) | Client-side Yrs and awareness | implemented | UI session, in-process CRDT, carets/selections |
| [044](spec-044-remote-websocket-client.md) | Remote WebSocket client transport | implemented | ureq REST + background WS, `--server`/`--token` |
| [045](spec-045-server-login-settings-ui.md) | Server login and settings UI | implemented | Server URL/email/password, connect/disconnect |
| [046](spec-046-module-boundary-refactor.md) | Module boundaries for server, model and command metadata | implemented | Facades, `server/routes`, `server/handlers`, `model/*` |
| [047](spec-047-use-case-seam-and-server-state.md) | Use-case seam, capability metadata and server sub-state | implemented | Shared `server/usecases`, `route_capability`, grouped state |
| [048](spec-048-build-performance-and-desktop-feature-seam.md) | Build performance and the desktop feature seam | implemented | Default-on `desktop` feature, core/desktop test passes, lean debug info |
| [049](spec-049-ui-client-modularization.md) | UI and client modularization | implemented | Command families, settings pages, content split, client retry |
| [050](spec-050-server-container-and-compose.md) | Server-only binary and Compose deployment | implemented | `ion-of-notes-server`, Dockerfile, `compose.yml` |
| [051](spec-051-container-hardening-and-deployment-operations.md) | Container hardening and deployment operations | implemented | Loopback bind, read-only root, Caddy example, deployment guide |
| [052](spec-052-deployment-hardening-ii.md) | Deployment hardening II | implemented | `/ready`, `/metrics`, Docker secret, multi-arch bake, CI container smoke |
| [053](spec-053-supply-chain-and-tooling.md) | Supply-chain and tooling baseline | implemented | Pinned toolchain/actions/images, Renovate, `cargo audit` |
| [054](spec-054-webhook-reliability.md) | Webhook reliability | implemented | Persistent outbox, worker, dead-letter/requeue, metrics |
| [055](spec-055-remote-mode-ux.md) | Remote-mode UX | implemented | Remembered session, background connect, startup restore |
| [056](spec-056-production-compose-and-tls.md) | Production Compose and TLS manifest | implemented | Caddy TLS stack, local TLS test, optional backup profile |
| [057](spec-057-ui-quality-and-interaction-fidelity.md) | UI quality and interaction fidelity | implemented | Phased layout, editor, icon, sidebar/workspace and settings fixes |
| [058](spec-058-multi-workspace-persistence.md) | Multi-workspace persistence | implemented (desktop catalog) | Desktop catalog, JSON migration, switcher, create/rename/delete/switch |
| [059](spec-059-app-bar-app-menu-and-sidebar-animation.md) | App bar, app menu and sidebar animation | implemented | Sidebar-width container, notification/compose, sectioned menu, animated sidebar |
| [060](spec-060-editor-picker-and-ai-responsiveness.md) | Editor, picker and AI responsiveness | implemented | Release-based drop, virtualized emoji grid, centered icons, async AI |
| [061](spec-061-settings-polish-and-number-time-formats.md) | Settings polish and number/time formats | implemented | Close/rail/scrollbar padding, working number and time formats |
| [062](spec-062-customizable-sidebar-sections.md) | Customizable sidebar sections and shortcuts | implemented | Row settings/exclude/drag, Add shortcut/section menus, sidebar tabs |
| [063](spec-063-sidebar-menus-breadcrumbs-and-gutter.md) | Sidebar menus, breadcrumbs and gutter alignment | implemented | Menu anchoring, breadcrumb padding, gutter centering, idle chevrons |
| [064](spec-064-block-editor-drop-targets-and-enter-split.md) | Block editor drop targets and Enter split | implemented | Gap-based drops, Enter creates a block, localized placeholder |
| [065](spec-065-page-icon-letter-fallback.md) | Page icon letter fallback | implemented | Empty icon uses the uppercase title letter everywhere |
| [066](spec-066-search-and-new-tab-filters.md) | Search and new-tab filters | implemented | Title only, Created by, In, Has comments, show/hide; remove Ask AI |
| [067](spec-067-runtime-smoke-tests.md) | Runtime smoke tests | implemented | Real listener, headless binary, Docker Compose smoke |
| [068](spec-068-header-icons-and-emoji-rendering.md) | Header, icons and emoji rendering | implemented | Breadcrumb centering, empty-icon fallback, emoji alignment/perf |
| [069](spec-069-dialog-sizing-scrolling-and-fixed-close.md) | Dialog sizing, scrolling and fixed close | implemented | Auto-shrink, rail/content scroll, border scrollbar, foreground close |
| [070](spec-070-sidebar-tabs-drag-and-remove.md) | Sidebar tabs, drag and remove | implemented | Sidebar tabs, section/shortcut drag, remove actions |
| [071](spec-071-popover-positioning-and-inbox.md) | Popover positioning and inbox | implemented | Anchored menus and full-height inbox panel |
| [072](spec-072-centralized-ui-metrics.md) | Centralized UI metrics | implemented | Constants-only layout module, even spacing scale, shared sidebar tokens |
| [073](spec-073-authentication-boundary.md) | Authentication boundary and security hardening | implemented | v1 auth, no production owner token, login limits, request caps, timeouts |
| [074](spec-074-atomic-persistence.md) | Atomic workspace and collaboration persistence | implemented | SnapshotStore port, SQLite transaction, reload after failure |
| [075](spec-075-webhook-ssrf-hardening.md) | Webhook SSRF hardening | implemented | Private-host policy and delivery-time DNS guard |
| [076](spec-076-principal-authorization.md) | Principal-aware authorization | implemented | Role capabilities, page shares, owner-only routes |
| [077](spec-077-review-response.md) | Review response documentation and roadmap | implemented | Security/authorization/persistence/performance docs and finding map |
| [078](spec-078-route-policy-table.md) | Central route policy table | implemented | One table for capability, block limit, owner-only and page scope |
| [079](spec-079-shared-crdt-helpers.md) | Shared CRDT text helpers | implemented | `crdt_text` module and model block-tree walkers |
| [080](spec-080-sidebar-decomposition.md) | Sidebar rendering decomposition | implemented | Sidebar shell plus customize/default/bottom functions |
| [081](spec-081-refactoring-roadmap.md) | Refactoring roadmap and implementation notes | implemented | Implemented/deferred refactors and next slices |
| [082](spec-082-request-pipeline-stages.md) | Request pipeline stages | implemented | Version/auth/limits/capability/authorization stages |
| [083](spec-083-workspace-api-segregation.md) | WorkspaceApi interface segregation | implemented | RequestTransport + CollabTransport |
| [084](spec-084-client-resource-modules.md) | Resource-oriented client modules | implemented | Client split by resource |
| [085](spec-085-transport-adapter-split.md) | Transport adapter split | implemented | `server/web/` and `remote/` modules |
| [086](spec-086-refactoring-round-ii.md) | Refactoring round II | implemented | Status and deferred roadmap |
| [087](spec-087-typed-handler-errors-and-comments.md) | Typed handler errors and comments use case | implemented | `HandlerResult`, comments use case, feature-specific handler |
| [088](spec-088-search-state-and-view.md) | SearchState and SearchView grouping | implemented | Search runtime/view fields grouped |
| [089](spec-089-typed-handlers-remaining.md) | Typed handlers for remaining families | implemented | User/file, blocks, databases, views, webhooks, auth |
| [090](spec-090-complete-typed-handlers.md) | Complete typed handler conversion | implemented | CRDT, lists and shared adapters |
| [091](spec-091-server-service-modules.md) | Server service modules | implemented | Auth/webhook logic under `server/services/` |
| [092](spec-092-sharing-state-and-view.md) | SharingState and SharingView grouping | implemented | Sharing/invite runtime and view fields grouped |
| [093](spec-093-collaboration-state.md) | CollaborationState grouping | implemented | CRDT/realtime/awareness runtime fields grouped |
| [094](spec-094-ai-and-server-state.md) | AiState and ServerState grouping | implemented | AI and server-login runtime/view fields grouped |
| [095](spec-095-settings-state-and-view.md) | SettingsState and SettingsView grouping | implemented | Settings dialog/preference runtime and view fields grouped |
| [096](spec-096-narrow-settings-views.md) | Narrow settings view models | implemented | Settings pages receive only their slice |
| [097](spec-097-performance-harness.md) | Deterministic performance harness | implemented | Opt-in 1k/10k/50k block timings |
| [098](spec-098-sidebar-commands.md) | Group sidebar commands | implemented | `SidebarCmd` behind `Command::Sidebar` |

## 3. Work status

| Item | Where | Status |
| ---- | ----- | ------ |
| B5 — broader mock server + contract suite | spec-016 | **Implemented:** Axum adapter + `tests/web.rs` (spec-038) |
| A9 — settings row filtering inside the open section | spec-016 | **Implemented:** per-page row table and filtered rendering |
| B4 — derive the contract from public API docs | spec-016 / ADR-006 | **Reviewed 2026-11-08**; specs 030–038 implemented |
| B3 - own-account HAR | spec-016 | **Blocked** by the strict B1 scope; needs a new decision |
| v2 foundation | spec-030 | **Implemented:** versioning, errors, capabilities |
| v2 pagination | spec-031 | **Implemented:** cursor lists |
| v2 rich text/blocks | spec-032 | **Implemented:** rich text, metadata, children |
| v2 properties/data sources | spec-033 | **Implemented:** typed schema, values, migration |
| v2 views | spec-034 | **Implemented:** CRUD, filters/sorts, grouping |
| v2 comments/users/files | spec-035 | **Implemented:** comments, users, uploads |
| v2 webhooks | spec-036 | **Implemented:** subscriptions, signatures, retries, all documented events |
| v2 limits | spec-037 | **Implemented:** rate/size/block limits, retry policy |
| Persist the selected language | spec-028 follow-up | Add to `WorkspaceSettings` or local config |
| Send or remove the invite message | spec-023 follow-up | Add `message` to `MemberDraft` or drop the UI field |
| Block-anchored inline comments | spec-026 follow-up | Model + tests |
| Production auth / WebSocket / PostgreSQL | ADR-003 | **Sessions + SQLite (039), WebSocket (040), TLS webhooks/shared limits (041), Yrs CRDT (042), client awareness (043), remote WS client (044) and login UI (045) done;** next: PostgreSQL, password reset |

## 4. Spec template

```markdown
# spec-NNN — Short title

## Purpose and scope
What this session changes, and what it deliberately does not.

## Evidence
Pictures, public docs, code findings, requirements.

## Acceptance criteria
Given / When / Then.

## Test scenarios (written before implementation)
Which test crates and test names.

## Design
Where the code goes and why.

## Plan / tasks
- [ ] ...
```

## 5. Rules

1. One spec is one working session. Split rather than sprawl.
2. Acceptance criteria are observable, not implementation steps.
3. Test scenarios are listed before implementation.
4. A spec that changes behavior updates `docs/requirements.md`; a new entity
   updates `docs/data-model.md`; a new route updates `docs/api-contract.md`.
5. A completed spec is committed in the same session with a message naming it.
6. A retired spec keeps a status note at the top and is not silently rewritten.
7. Historical plans keep their checkboxes honest: `[x]` only when the item is
   actually done, with a note when a later spec completed it.
