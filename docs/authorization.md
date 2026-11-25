# Authorization model

This document describes the principal-aware authorization added in spec-076.
It explains the role capabilities, the page-access resolution order and the
enforcement points in the v1 and v2 adapters.

## 1. Roles and capabilities

| Role | Content read | Content update | Content insert | Comments read | Comments insert | User info |
| ---- | ------------ | -------------- | -------------- | ------------- | --------------- | --------- |
| Owner | Yes | Yes | Yes | Yes | Yes | With email |
| Member | Yes | Yes | Yes | Yes | Yes | With email |
| Guest | Yes | No | No | Yes | No | Without email |

Capabilities are the coarse route gate (`route_capability` in the v2
pipeline). They do **not** replace page-level access checks.

The demo owner connection (`NoAuthStore`) keeps every capability. Session
connections derive their capabilities from the member role in
`connection_for_token`.

## 2. Page access resolution

`WorkspaceServer::page_access` resolves access in this order:

1. **Owner** → `Access::Full`.
2. **Direct share** for this page where the subject matches the member id,
   email (case-insensitive) or name → the highest matching access.
3. **Page-general share** (`page_id == the page`, subject `*` or
   `Everyone at workspace`) → that access.
4. **Workspace-general share** (`page_id == "*"`, subject `*` or
   `Everyone at workspace`) → that access.
5. Otherwise → `Access::None`.

`Access::View` allows reads; `Access::Edit` and `Access::Full` allow writes;
`Access::None` denies both.

## 3. Enforcement points

### v2 pipeline

After version, token, rate-limit and capability checks:

- direct page routes (`GET/PATCH/DELETE /api/v2/pages/{id}`, its versions and
  properties) call `can_read_page` / `can_write_page`;
- nested resources resolve their owning page in `server::authorization` and
  apply the same check: blocks, comments, databases, data sources and views;
- workspace-level resources (a database without a page parent) are owner-only;
- `PUT /api/v2/workspace`, `POST /api/v2/members`, `PUT /api/v2/shares`,
  `GET /api/v2/export`, webhook administration and file-upload listing require
  the owner role;
- `GET /api/v2/workspace` returns `WorkspaceServer::project_workspace`: only
  readable pages and the databases, data sources, views, comments and
  referenced uploads attached to them;
- `GET /api/v2/pages`, `/api/v2/search` and `/api/v2/sync` filter their
  results; `/api/v2/shares` requires owner access when no page is given and
  read access otherwise;
- denied requests return `403 restricted_resource` with
  `additional_data.required_access` or `required_role`.

### CRDT and WebSocket

`POST /api/v2/crdt/sync` requires read access to the body `page_id` and
`POST /api/v2/crdt/update` requires write access before the document is read or
merged. The same checks run for WebSocket `crdt_sync` and `crdt_update`
messages. The connection backlog and live change/CRDT messages are dropped when
they belong to a page the connection cannot read. Workspace-level change markers
(`*`, `__workspace__`, resource markers) are owner-only.

Awareness identity is not trusted from the client: the server overwrites
`session_id`, `member_id` and `name` from the authenticated session and rejects
an update whose page is unreadable, whose block is not on that page, or whose
cursor/selection is outside the block text. Broadcast awareness is filtered per
entry, so one unreadable cursor neither drops the batch nor leaks a page the
member cannot read. A deleted page keeps its delete event visible through the
persisted share rules, even though the page itself is gone.

### Webhooks

Webhook subscription, delivery, retry and requeue routes are owner-only. List
responses use `WebhookSubscriptionDto`, which has no `verification_token`; the
secret appears only in the creation response.

### v1 adapter

The same page target, nested-resource, projection, list/search/sync and
owner-only checks run after the token check. The local `NoAuthStore` resolves
the demo owner connection, so local mode keeps working and sees the full
workspace.

## 4. Account and invitation lifecycle

The current `POST /api/members` path creates a collaboration member but does
not create an authentication account. This is intentional for now: the
invitation → account → first-login lifecycle needs a product decision about
temporary passwords, email delivery and account recovery. It is tracked in the
roadmap; do not treat a collaboration invite as a working login.

## 5. Known gaps

- **Client-side hiding**: the server filters every read, but the desktop UI
  still renders the full local tree in `NoAuthStore` mode. That connection is
  the owner by design.
- **Grant granularity**: shares are page-level; there is no per-block or
  per-property grant.
- **Pagination totals**: list routes filter before paginating, so a page's
  `has_more`/cursor reflects the filtered set rather than the workspace total.
- **Sharing UI**: the Share dialog still writes grants through the owner
  connection in local mode; remote member flows should use their own session.
- **Password reset / registration**: not implemented.

## 6. Adding a protected resource

1. Decide whether the route is owner-only, page-scoped or workspace-level.
2. For a page-scoped route, resolve the page id from the path and call
   `can_read_page` / `can_write_page`.
3. For a nested resource, use `WorkspaceServer::page_for_block` /
   `page_for_comment` / `page_for_database` / `page_for_data_source` /
   `page_for_view` and then `can_access_resource`; do not authorize only by the
   resource id.
4. For a global read, return a projection or filter the result set.
5. Add a positive and a negative test with a member and a guest session.
6. Return `403 restricted_resource` with a stable `required_access` or
   `required_role` value in `additional_data`.
