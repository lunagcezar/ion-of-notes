# spec-076 - Principal-aware authorization

- Status: implemented (2026-11-12)
- Related: `spec-030`, `spec-035`, `spec-073`
- Review sources: the 2026-11-12 external reviews (Kimi, ChatGPT, Claude)

## Purpose and scope

Authentication alone is not authorization. The reviews found that sessions
received `Capability::all()` regardless of role, and page reads/writes did not
check the principal against the page shares. A guest or member could therefore
reach pages the workspace had not shared with them.

This spec adds a first, explicit authorization layer:

- role-derived capabilities for owner, member and guest sessions;
- page-level read/write checks against direct and general shares;
- owner-only workspace replacement, member invitation and share mutation;
- the same checks on v1 and v2 direct page routes.

Scope:

- `capabilities_for_role` in the server;
- `WorkspaceServer::page_access`, `can_read_page`, `can_write_page`,
  `role_for` and `is_owner`;
- direct share matching by member id, email or name, with page-general and
  workspace-general fallback;
- v1/v2 page route checks and owner-only management routes;
- tests with owner, member and guest sessions.

Out of scope (documented in `docs/authorization.md` and the roadmap):

- nested resources (blocks, comments, databases, files) resolving their parent
  page before authorization;
- filtering list/search responses per principal;
- account invitation → authentication account creation;
- password reset and account-management UI.

## Acceptance criteria

P1. **Given** a member session, **when** a page is shared with that member,
**then** the member can read and write it.

P2. **Given** a guest session, **when** a page is shared as `Can view`, **then**
the guest can read it but cannot write it.

P3. **Given** a member or guest session, **when** a page has no share for that
principal and no general share, **then** reading or writing it returns `403`.

P4. **Given** a non-owner session, **when** it tries to replace the workspace,
invite a member or set a share, **then** the request returns `403`.

P5. **Given** an owner session, **when** it accesses any page or management
route, **then** the request is allowed.

P6. **Given** a guest session, **when** it attempts to create a page, **then**
the request is denied because the guest lacks `InsertContent`.

## Test scenarios

- `tests/authorization.rs::members_and_guests_are_limited_by_page_shares`
- `tests/authorization.rs::only_the_owner_can_replace_the_workspace`
- `tests/authorization.rs::v1_page_routes_use_the_same_share_checks`

## Design

- `Capability` remains the coarse route gate. Session capabilities are derived
  from `Role`: owner all; member content read/update/insert plus comments and
  user info; guest content/comments read plus user info without email.
- `page_access` resolves direct shares first, then `general_access` (page
  general, then workspace general). An owner always has `Access::Full`.
- v1 and v2 page targets are parsed in one small helper per adapter. Both use
  the same `can_read_page` / `can_write_page` methods.
- Owner-only routes are an explicit allowlist in each adapter.
- Nested resources are a documented follow-up: they need to resolve their
  parent page before the same check.

## Plan / tasks

- [x] Write spec-076 and failing authorization tests (red)
- [x] Derive session capabilities from the member role
- [x] Add page access resolution and v1/v2 page checks
- [x] Add owner-only management checks
- [x] Add `docs/authorization.md`
- [x] Run the full gate and commit
