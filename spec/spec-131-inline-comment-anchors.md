# spec-131 - Inline comment block anchors

- Status: implemented (2026-11-24)
- Related: `spec-035`, `spec-075`, `spec-115`
- Review source: deferred block-anchor item in `docs/review-response.md`

## Purpose and scope

Comments could target a page or a whole block, but not a text range inside a
block. This spec adds an optional anchor:

```json
{ "block_id": "h2", "start": 0, "end": 4 }
```

- `CommentAnchor` is part of `CommentDto` and `CommentInput` with
  `#[serde(default)]`, so existing comments and JSON remain valid.
- `create_comment` and `update_comment` validate:
  - the anchor block exists;
  - it belongs to the comment's parent page;
  - when the parent is a block, the anchor block is that same block;
  - `start <= end <= block.text.chars().count()` (character offsets, matching
    awareness indices).
- The use case persists the anchor through `write` and the client passes the
  DTO unchanged.

Out of scope:

- rendering inline highlight threads in the desktop UI (the current comment
  panel is page-level);
- resolving anchors after text edits (the server validates at write time);
- multi-block anchors.

## Acceptance criteria

P1. **Given** a block comment with a valid in-range anchor, **when** it is
created, **then** the response contains the anchor.

P2. **Given** an anchor past the block text, **then** the request is rejected
with `validation_error`.

P3. **Given** an anchor block from another page, **then** the request is
rejected.

P4. **Given** a block-parent comment, **then** the anchor block must match the
parent block.

P5. **Given** an existing comment without an anchor, **then** it still
deserializes and updates.

## Test scenarios

- `tests/comments.rs` (`comment_anchor_is_validated_against_the_block_text`,
  existing comment CRUD tests).
- `tests/authorization.rs` (comment creation still page-scoped).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| DTO/input | `model/dto/comment.rs` |
| Validation | `server/usecases/comments.rs::validate_anchor` |
| Client | existing `create_comment_v2` / `update_comment_v2` |

## Plan / tasks

- [x] Add `CommentAnchor` with serde defaults.
- [x] Validate existence, page ownership, parent consistency and range.
- [x] Persist the anchor on create and update.
- [x] Add regression tests.
