---
title: Workspace server, REST-shaped API, persistence, and loopback listener
version: 1.0
date_created: 2026-11-05
tags: server, api, persistence
status: implemented
---

# Spec 003 — Workspace Server

## 1. Purpose & Scope

Build the server side requested in the original brief. The server owns all
state, answers `ApiRequest`s through the `WorkspaceApi` trait, persists the
workspace as JSON, and can optionally be exposed over loopback HTTP.

## 2. Acceptance Criteria

- **AC-001** — Given a server, when `GET /health` is handled, then status 200
  and body `{"data":"ok","error":null}` are returned.
- **AC-002** — Given a server, when `GET /api/workspace` is handled, then the
  full workspace is returned as a JSON envelope.
- **AC-003** — Given a server, when `POST /api/pages` with a draft is handled,
  then status 201, the new page is returned, the id is a UUID, and `version`
  increases.
- **AC-004** — Given a draft whose parent does not exist, when it is created,
  then status 400 with an error message and no page is added.
- **AC-005** — Given a server, when `GET /api/pages/{unknown}` is handled,
  then status 404.
- **AC-006** — Given a server, when `PATCH /api/pages/{id}` with a page is
  handled, then the page is replaced and the change is persisted to disk.
- **AC-007** — Given a page with children, when `DELETE /api/pages/{id}` is
  handled, then the subtree is gone and the file on disk reflects it.
- **AC-008** — Given a workspace, when `GET /api/search?q=road` is handled,
  then a ranked result list is returned.
- **AC-009** — Given an unknown route or malformed JSON, when handled, then
  status 404 or 400 respectively, never a panic.
- **AC-010** — Given an HTTP request line and headers, when
  `parse_http_request` runs, then method, path, and body are extracted.
- **AC-011** — Given an `ApiResponse`, when `format_http_response` runs, then
  a valid HTTP/1.1 response with `Content-Length` and `Content-Type` is
  produced.
- **AC-012** — Given a missing data file, when `load_or_seed` runs, then a
  demo workspace is created and saved.

## 3. Test Scenarios

- TS-001 `health_route` — TS-002 `workspace_snapshot_route`
- TS-003 `create_page_returns_201_and_bumps_version`
- TS-004 `create_page_rejects_unknown_parent`
- TS-005 `get_unknown_page_returns_404`
- TS-006 `patch_persists_to_disk`
- TS-007 `delete_removes_subtree_and_persists`
- TS-008 `search_route_returns_ranked_results`
- TS-009 `unknown_route_and_bad_json_do_not_panic`
- TS-010 `parse_http_request_extracts_method_path_body`
- TS-011 `format_http_response_has_valid_headers`
- TS-012 `load_or_seed_creates_file`

## 4. Plan & Tasks

1. Write the twelve tests first — **red**.
2. Implement `ApiRequest`, `ApiResponse`, `Method`, `Envelope`, `WorkspaceApi`.
3. Implement `WorkspaceServer` with route dispatch and persistence hooks.
4. Implement `parse_http_request` / `format_http_response` / `serve_once`.
5. Implement `--serve` in `main.rs` using `std::net`.
6. Run `cargo test` — **green**; update `CHANGELOG.md`.
