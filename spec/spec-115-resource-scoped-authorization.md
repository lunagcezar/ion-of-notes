# spec-115 - Resource-scoped authorization

- Status: implemented (2026-11-24)
- Related: `spec-030`, `spec-035`, `spec-036`, `spec-076`, `spec-111`
- Review source: Kimi and ChatGPT security reviews (2026-11-24)

## Purpose and scope

The v2 route table already applied capabilities and direct page checks, but a
capability is not a page permission. Nested resources (blocks, comments,
databases, data sources, views, file uploads) and global reads (workspace,
pages, search, sync, export) could still reach pages the principal did not
share. The WebSocket backlog and CRDT routes had the same gap.

This spec adds one resource-authorization layer and uses it everywhere:

- `server::authorization` resolves the owning page of a nested resource and
  applies `can_read_page` / `can_write_page`; workspace-level resources are
  owner-only.
- `GET /api/v2/workspace` (and v1 `/api/workspace`) return a projection with
  only readable pages and their databases, data sources, views, comments and
  referenced uploads.
- `GET /api/v2/pages`, `/api/v2/search`, `/api/v2/sync` and the v1 equivalents
  filter their results per principal.
- Blocks, comments, databases, data sources, views and CRDT REST/WebSocket
  operations resolve the parent page before reading or writing.
- The WebSocket backlog and live change/CRDT/awareness messages are filtered
  by page access.
- Webhook administration and export are owner-only; webhook list responses
  never include `verification_token`.
- File-upload listing is owner-only; upload creation stays available to
  members with the insert capability.

Out of scope:

- a UI that hides unreachable pages (the server is authoritative);
- per-property or per-block share grants (the model shares pages);
- cursor-level filtering that also changes pagination totals (results are
  filtered before cursor pagination).

## Acceptance criteria

P1. **Given** a member with a share on one page, **when** the member reads a
block, comment, database, data source or view of another page, **then** the
server returns `403 restricted_resource` and no resource data.

P2. **Given** the same member, **when** the member lists pages, searches,
reads the workspace, syncs or lists comments, **then** only resources of shared
pages are returned.

P3. **Given** a guest with view access, **when** the guest patches or deletes a
nested resource, **then** the write is denied.

P4. **Given** a member who knows a CRDT page id, **when** the member requests a
sync or applies an update through REST or WebSocket, **then** read/write page
access is enforced before the CRDT document is read or merged.

P5. **Given** a WebSocket subscriber, **when** another page changes or a CRDT
update is published, **then** the subscriber receives only messages for pages
it can read.

P6. **Given** a member session, **when** it calls webhook, export or
file-upload listing routes, **then** the owner-only policy applies; webhook
list bodies omit the verification secret.

P7. **Given** an owner, **when** it reads the workspace, **then** the complete
workspace is returned.

## Test scenarios

- `tests/authorization.rs`:
  `nested_block_routes_resolve_the_owning_page`,
  `nested_comment_routes_resolve_the_owning_page`,
  `list_search_and_workspace_reads_are_projected`,
  `database_view_and_export_routes_require_page_access`,
  `crdt_and_webhook_routes_are_scoped_to_the_connection`.
- `tests/web.rs` (`webhook_roundtrip_works_through_axum` asserts the list body
  has no `verification_token`; `snapshot_reads_are_projections_for_a_member`
  covers the fast snapshot path).
- Existing `tests/blocks.rs`, `tests/comments.rs`, `tests/views.rs`,
  `tests/properties.rs`, `tests/realtime.rs`, `tests/webhooks.rs`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Page resolution, access and projection | `src/server/authorization.rs` |
| v2 pipeline nested page ids | `src/server/routes/pipeline.rs` |
| Owner-only policy rows | `src/server/routes/policy.rs` |
| Workspace projection | `WorkspaceServer::project_workspace` |
| Block checks | `src/server/handlers/blocks.rs` |
| Comment checks | `src/server/handlers/comments.rs` |
| Database/data-source checks | `src/server/handlers/databases.rs` |
| View checks | `src/server/handlers/views.rs` |
| List/search/sync filters | `src/server/handlers/lists.rs` |
| Workspace/export/page create | `src/server/handlers/shared.rs` |
| CRDT REST | `src/server/handlers/crdt.rs` |
| CRDT/WebSocket/backlog | `src/server/web/websocket.rs` |
| v1 filters | `src/server/routes/v1.rs` |
| Webhook DTOs | `src/webhooks.rs`, `src/server/services/webhooks.rs` |

The projection is deliberately computed server-side on every read. It is not a
cache with an invalidation rule that could serve a stale permission.

## Plan / tasks

- [x] Add page resolution and resource access helpers.
- [x] Add principal-aware workspace, search and sync projections.
- [x] Apply checks to blocks, comments, databases, data sources and views.
- [x] Apply page-scoped CRDT checks to REST and WebSocket.
- [x] Filter the WebSocket backlog and live messages.
- [x] Make webhooks, export and upload listing owner-only; redact webhook
  secrets.
- [x] Extend the v1 filters.
- [x] Add regression tests.
