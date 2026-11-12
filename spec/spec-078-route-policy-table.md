# spec-078 - Central route policy table

- Status: in progress (2026-11-12)
- Related: `spec-030`, `spec-037`, `spec-038`, `spec-073`, `spec-076`
- Review sources: Claude (route duplication, block-limit bookkeeping) and
  ChatGPT (central route metadata)

## Purpose and scope

The v2 API describes the same route in several places: the capability table,
the block-limit calculation, the owner-only checks and the handler dispatch.
Adding a route can leave one of those representations behind, and a route can
return `404` instead of `403` when the capability table is incomplete.

This spec introduces one declarative route-policy table and makes the
cross-cutting pipeline read it.

Scope:

- a `RoutePolicy` value with method set, exact/prefix match, capability,
  block-limit policy, owner-only flag and page-scoped flag;
- a `ROUTES` table and `route_policy(method, path)` lookup;
- `route_capability` delegates to the table;
- `handle_v2_inner` uses the policy for capability, owner-only and page-scoped
  checks;
- `blocks_created_by` uses the policy's block-limit kind;
- contract tests proving every known route has a policy and unknown routes do
  not inherit one.

Out of scope:

- replacing the handler dispatch with a function-pointer table (the handler
  match stays until the resource handlers are split);
- typed `ApiResult` handlers (documented in the refactoring roadmap);
- generating OpenAPI/docs from the table.

## Acceptance criteria

P1. **Given** a known v2 method/path, **when** `route_policy` is called, **then**
it returns one policy with the capability, block-limit kind and flags.

P2. **Given** an unknown v2 method/path, **when** the pipeline runs, **then**
the request returns `404` and does not inherit a capability or owner flag.

P3. **Given** `PUT /api/v2/workspace`, `POST /api/v2/members` or
`PUT /api/v2/shares`, **when** a non-owner calls it, **then** the policy
returns `owner_only` and the request is rejected with `403`.

P4. **Given** `PATCH /api/v2/pages/{id}`, **when** the pipeline calculates the
block delta, **then** it uses the policy's `PageReplace` limit kind.

P5. **Given** the contract test, **when** a new route is added to the table
without a capability, **then** the test fails.

## Test scenarios

- `tests/routes.rs::known_routes_have_one_policy`
- `tests/routes.rs::unknown_routes_have_no_policy`
- `tests/routes.rs::owner_only_routes_are_explicit`
- `tests/routes.rs::block_limit_kinds_are_explicit`
- `tests/routes.rs::page_scoped_routes_are_explicit`

## Design

- `src/server/routes/policy.rs` owns `RoutePolicy`, `MatchKind`,
  `BlockLimit`, `ROUTES` and `route_policy`.
- `capabilities.rs` becomes a thin re-export so the public
  `route_capability` API is unchanged.
- The table is ordered: exact routes first, then specific prefixes, then the
  general page prefix. `route_policy` returns the first match.
- `RoutePolicy` is `Copy` and `'static`; no allocation per request.
- The pipeline still dispatches through the existing handler match. The table
  is the single source for cross-cutting policy, not a second dispatcher.

## Plan / tasks

- [ ] Write spec-078 and failing route-policy tests (red)
- [ ] Add the policy table and lookup
- [ ] Delegate capability, block limit, owner-only and page-scoped checks
- [ ] Add the contract tests
- [ ] Document the table in `docs/architecture.md` and the refactoring roadmap
- [ ] Run the full gate and commit
