# spec-047 - Use-case seam, capability metadata and server sub-state

- Status: implemented (2026-11-10)
- Related: `docs/review-findings.md` section 9 (RF-M07, RF-M08),
  `spec-046`, `docs/adr-002-architecture-guidelines.md`

## Purpose and scope

This is the follow-up refactor to spec-046. It addresses the two remaining
application-layer findings from the modularity review:

1. **v2 depended on the v1 transport.** `handle_v2_inner` rewrote
   `/api/v2` to `/api`, called `handle_v1`, and mapped the v1 error envelope
   back to `ApiError`. A route existed in two dispatchers, and a change could
   drift between them.
2. **`WorkspaceServer` was a flat god object.** Persistence, runtime limits,
   webhooks, realtime and CRDT fields sat in one struct, so every handler could
   reach every dependency.

In scope:

- Extract page, workspace and sharing use cases into `server/usecases/*`,
  returning typed results and one `UseCaseError` instead of `ApiResponse`.
- Rewrite both `routes/v1.rs` and the v2 shared routes to adapt those use cases;
  remove the v1 fallback and `map_v1_error`.
- Make the capability table public as `route_capability` and test that every
  documented v2 route has one.
- Group `WorkspaceServer` state into `Persistence`, `Runtime` and
  `WebhookRuntime` sub-structs.

Out of scope:

- No behavior, protocol, persistence, DTO or visual change.
- No new crate and no use-case trait indirection; the server still has one
  bounded context.
- UI/client splits stay in spec-048.

## Evidence

- `docs/review-findings.md` RF-M07 and RF-M08.
- `docs/architecture.md` sections 3 and 11.
- `spec-046` split the files; this spec fixes the dependency and state shape
  those files exposed.
- The existing 252 integration tests plus `--screenshot` are the behavior
  contract.

## Acceptance criteria

1. **Given** a v1 route, **when** it serves a page/workspace/sharing request,
   **then** it calls the same `server::usecases` function as v2 and adapts the
   typed result to the v1 envelope.
2. **Given** a v2 non-list route, **when** it serves a page/workspace/sharing
   request, **then** it calls the use case directly; no request is rewritten to
   `/api` and no v1 error envelope is parsed.
3. **Given** a documented v2 route, **when** `route_capability` is queried,
   **then** it returns a capability; an unknown route returns `None`.
4. **Given** `WorkspaceServer`, **when** its fields are read, **then**
   persistence, runtime limits and webhooks are grouped and a handler can only
   reach the sub-state it names.
5. **Given** the full suite, **when** build, tests, clippy, fmt and
   `--screenshot` run, **then** all are green and the window is unchanged.
6. **Given** the docs, **when** the spec completes, **then** the architecture
   map, review findings, changelog and spec index reflect the seam.

## Test scenarios (written before implementation)

- Existing `tests/server.rs`, `tests/contract.rs`, `tests/limits.rs`,
  `tests/collab.rs`, `tests/integration.rs`, `tests/web.rs` and the remaining
  crates exercise the public v1/v2 behavior.
- New `tests/contract.rs` test: every documented v2 method/path pair resolves
  through `route_capability`; `/api/v2/unknown` resolves to `None`.
- Red step: add the new module declarations and the `route_capability`
  re-export before moving the code, run `cargo test --all-targets`, and observe
  the compile failure.
- Clippy and rustfmt stay clean; the default screenshot matches.

## Design

```text
src/server/usecases.rs            facade over the use-case modules
src/server/usecases/
  error.rs                        UseCaseError { Invalid, NotFound, Storage }
  pages.rs                        list/get/create/update/delete/versions/search
  workspace.rs                    get/replace
  sharing.rs                      members, shares, sync, export, realtime
src/server/routes/v1.rs           v1 adapter over use cases
src/server/handlers/shared.rs     v2 adapter over the same use cases
src/server/routes/v2.rs           pipeline; no v1 fallback
src/server/routes/capabilities.rs public route_capability
```

State shape:

```text
WorkspaceServer {
  workspace, collab,
  persistence: Persistence { workspace, collab, auth },
  runtime: Runtime { connections, request_log, plan, block counts,
                      overload, write timeout, rate limits },
  webhooks: WebhookRuntime { allow_insecure, queue, sender },
  realtime, crdt
}
```

The use cases keep the existing side effects in the same order: validate,
mutate, record a version, `write` (which persists and publishes), then emit the
webhook. Error mapping stays explicit in each adapter.

## Plan / tasks

- [x] Write this spec and record the red compile step.
- [x] Add `server/usecases/*` and move the v1 page/workspace/sharing logic.
- [x] Rewrite v1 and add v2 shared routes over the use cases; remove the v1
      fallback.
- [x] Expose `route_capability` and add the contract test.
- [x] Group `WorkspaceServer` state into sub-structs.
- [x] Update docs, changelog and spec index.
- [x] Run build, tests, clippy, fmt and the screenshot check.
- [x] Commit the whole `app/` tree naming this spec.
