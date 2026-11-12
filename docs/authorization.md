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

- direct page routes (`GET/PATCH/DELETE /api/v2/pages/{id}` and
  `GET /api/v2/pages/{id}/versions`) call `can_read_page` / `can_write_page`;
- `PUT /api/v2/workspace`, `POST /api/v2/members` and `PUT /api/v2/shares`
  require the owner role;
- denied requests return `403 restricted_resource` with
  `additional_data.required_access` or `required_role`.

### v1 adapter

The same page target and owner-only checks run after the token check. The
local `NoAuthStore` resolves the demo owner connection, so local mode keeps
working.

## 4. Account and invitation lifecycle

The current `POST /api/members` path creates a collaboration member but does
not create an authentication account. This is intentional for now: the
invitation → account → first-login lifecycle needs a product decision about
temporary passwords, email delivery and account recovery. It is tracked in the
roadmap; do not treat a collaboration invite as a working login.

## 5. Known gaps

- **Nested resources**: blocks, comments, databases, views and file uploads
  do not yet resolve their parent page before authorization. They still sit
  behind the route capability and the authenticated connection, but a member
  with a page share should not be able to reach a nested resource of an
  unshared page. This is the next authorization slice.
- **List and search filtering**: `GET /api/v2/pages`, `/api/v2/search` and the
  v1 equivalents return the workspace view for authorized connections. A
  principal-aware list/search filter is a follow-up.
- **Sharing UI**: the Share dialog still writes grants through the owner
  connection in local mode; remote member flows should use their own session.
- **Password reset / registration**: not implemented.

## 6. Adding a protected resource

1. Decide whether the route is owner-only or page-scoped.
2. For a page-scoped route, resolve the page id from the path and call
   `can_read_page` / `can_write_page`.
3. For a nested resource, resolve its parent page first; do not authorize only
   by the resource id.
4. Add a positive and a negative test with a member and a guest session.
5. Return `403 restricted_resource` with a stable `required_access` or
   `required_role` value in `additional_data`.
