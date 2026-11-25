# spec-138 - Resource moves and upload ownership

- Status: implemented (2026-11-25)
- Related: `spec-115`, `spec-131`, `spec-035`, `docs/authorization.md`
- Review source: 2026-11-25 Kimi/ChatGPT second audit, findings 2.8-2.10

## Purpose and scope

A write check on the *current* owner of a nested resource is not enough when an
update can change that owner. This spec closes the two move paths and gives file
uploads a creator, an optional page scope and durable content.

### Moves

- `PATCH /api/v2/views/{id}` with `data_source_id` resolves the destination data
  source (404 when unknown), derives its owning page and requires write access
  there before mutating.
- `PATCH /api/v2/databases/{id}` with `parent_page_id` requires the destination
  page to exist and requires write access there before mutating.

### Uploads

- `FileUpload` gains `created_by` and optional `page_id`.
- `FileUploadInput` accepts `page_id`; creating an upload for a page requires
  write access to that page.
- `send` and `complete` require the creator (or the owner) and, when the upload
  is page-scoped, write access to that page.
- `send` validates the declared `content_length`, writes the body to
  `<uploads-dir>/<upload-id>` atomically and records the real length.
- `complete` requires an uploaded body whose stored length matches the record.
- `WorkspaceServer::use_uploads_dir` overrides the directory (tests and
  deployments); the default is `<data-dir>/uploads`.

Out of scope: quotas, MIME sniffing, virus scanning and a download route.

## Acceptance criteria

P1. **Given** edit access to page A and no access to page B, **when** a member
moves a database from A to B, **then** the request is 403 and the parent is
unchanged.

P2. **Given** the same member and a view whose data source lives on B, **then**
moving the view to that data source is 403.

P3. **Given** a member who creates an upload, **then** another member cannot
`send` or `complete` it.

P4. **Given** a declared length that differs from the body, **then** `send` is
400 and no content is stored.

P5. **Given** a successful `send`, **then** the body exists on disk with the
recorded length and `complete` succeeds; without the body `complete` is 400.

P6. **Given** an upload created for a page, **then** the creator must still have
write access to that page to mutate it.

## Test scenarios

- `tests/moves.rs` (`database_cannot_move_to_a_page_without_write_access`,
  `view_cannot_move_to_a_data_source_on_an_unreadable_page`,
  `upload_can_only_be_sent_by_its_creator`).
- `tests/comments.rs` upload lifecycle and client round-trip.
- Existing view/database authorization tests.

## Design

| Piece | Location |
| ----- | -------- |
| Destination check (views) | `server/handlers/views.rs` |
| Destination check (databases) | `server/handlers/databases.rs` |
| Upload model/input | `model/file.rs` |
| Upload authorization and storage | `server/handlers/user_file.rs`, `server.rs` |

## Plan / tasks

- [x] Authorize the destination page for view and database moves.
- [x] Add upload creator/page fields and enforce them.
- [x] Persist upload content and validate length on send/complete.
- [x] Add regression tests.
