---
title: API client used by the desktop UI
version: 1.0
date_created: 2026-11-05
tags: client, api
status: implemented
---

# Spec 004 — Client Layer

## 1. Purpose & Scope

Provide the only data-access object the UI is allowed to use. `Client<A>` is
generic over the `WorkspaceApi` trait, so it works with the in-process server
now and a remote transport later. The client builds requests, checks status
codes, and decodes JSON envelopes.

## 2. Acceptance Criteria

- **AC-001** — Given a server with a demo workspace, when `client.workspace()`
  runs, then it returns the same workspace the server holds.
- **AC-002** — Given a draft, when `client.create_page()` runs, then a page
  with a non-empty id is returned and a later `get_page()` finds it.
- **AC-003** — Given an existing page, when `client.update_page()` runs, then
  the server holds the updated title.
- **AC-004** — Given a page id, when `client.delete_page()` runs, then
  `get_page()` returns `NotFound`.
- **AC-005** — Given a query, when `client.search()` runs, then ranked results
  are decoded correctly.
- **AC-006** — Given a stub API that always returns 500, when any client
  method runs, then `ClientError::Http(500)` is returned instead of a panic.
- **AC-007** — Given a malformed JSON body with status 200, when a client
  method runs, then `ClientError::Decode` is returned.
- **AC-008** — Given a workspace snapshot, when `client.replace_workspace()`
  runs, then the server state equals the snapshot.

## 3. Test Scenarios

- TS-001 `workspace_matches_server_state`
- TS-002 `create_then_get_page`
- TS-003 `update_page_changes_title`
- TS-004 `delete_page_makes_it_unavailable`
- TS-005 `search_results_decode`
- TS-006 `http_error_maps_to_client_error`
- TS-007 `bad_json_maps_to_decode_error`
- TS-008 `replace_workspace_roundtrip`

## 4. Plan & Tasks

1. Write the eight tests (including a `StubApi`) — **red**.
2. Implement `Client<A: WorkspaceApi>` with typed methods and `ClientError`.
3. Run `cargo test` — **green**; update `CHANGELOG.md`.
