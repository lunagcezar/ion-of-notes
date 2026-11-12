# spec-084 - Resource-oriented client modules

- Status: implemented (2026-11-12)
- Related: `spec-030`, `spec-044`, `spec-083`
- Review sources: Claude and ChatGPT (large `client.rs`, duplicated request
  processing)

## Purpose and scope

`src/client.rs` was a single ~759-line module covering v1/v2 workspace, pages,
search, members, shares, sync, blocks, databases, data sources, views,
comments, users, file uploads, webhooks, CRDT and realtime. The public
`Client` facade stays, but the implementations move into resource modules.

Scope:

- `src/client/mod.rs`: `Client`, constructors, request helpers, pagination,
  `ClientError`, `encode_body`, retry re-export;
- `src/client/pages.rs`: workspace, pages, search (v1 and v2);
- `src/client/sharing.rs`: members, shares, sync, export, realtime contract;
- `src/client/blocks.rs`;
- `src/client/databases.rs`;
- `src/client/views.rs`;
- `src/client/comments.rs`: comments, users, file uploads;
- `src/client/webhooks.rs`;
- `src/client/collab.rs`: CRDT and realtime transport methods;
- `src/client/retry.rs` unchanged.

Out of scope:

- changing the public method names or signatures;
- a separate protocol module for v1/v2;
- a second client facade.

## Acceptance criteria

P1. **Given** the public `Client` API, **when** the modules move, **then** all
method names, signatures and return types are unchanged.

P2. **Given** a request-only transport, **when** a request method is called,
**then** it compiles through `impl<A: RequestTransport> Client<A>`.

P3. **Given** a collaboration-capable transport, **when** a CRDT/realtime
method is called, **then** it compiles through `impl<A: WorkspaceApi>
Client<A>`.

P4. **Given** the existing client/remote/CRDT tests, **when** the split is
applied, **then** they pass unchanged.

## Test scenarios

- `tests/client.rs`
- `tests/remote.rs`
- `tests/crdt.rs` and `tests/ui_crdt.rs`
- `tests/app.rs`, `tests/contract.rs` and the full gate

## Design

- The module root keeps the shared request plumbing and the public `Client`.
- Each resource file starts with `use super::*;` and an `impl` block with the
  correct transport bound.
- Private helpers (`v2_request`, `call`, `call_v2`, `paged_path`,
  `list_page_v2`, `collect_all_v2`) remain in the root and are reachable from
  child modules.
- `comments_path` lives with the comments resource.

## Plan / tasks

- [x] Write spec-084
- [x] Split the methods into resource modules
- [x] Keep the public API and tests unchanged
- [x] Run the full gate and commit
