# spec-136 - Inline comment thread rendering

- Status: implemented (2026-11-24)
- Related: `spec-035`, `spec-131`
- Review source: deferred inline-thread rendering item

## Purpose and scope

Anchored comments existed in the model/API but the page discussions panel only
rendered the legacy page-level `Comment` list. Anchored v2 comments were
invisible in the desktop.

This spec adds:

- `ui::comments::inline_threads(workspace, page_id)`:
  - selects v2 comments whose parent resolves to the page;
  - ignores comments without an `anchor`;
  - groups by `discussion_id`;
  - orders threads by the block tree order and anchor start;
  - quotes the anchored block range (or the first 80 characters when the range
    is empty) and resolves author/text/timestamp;
- an **Inline comments** section in the page panel showing each thread's quote
  and comments;
- a composer in that section: choose a block, write a comment, and create a v2
  comment anchored to the whole block (`0..block.text.chars().count()`);
- a delete button calling `delete_comment_v2` and removing the comment from the
  workspace and committed baseline;
- localized `comment_deleted` and updated inline-comment hint text in en,
  pt-BR and eo.

Out of scope:

- dragging/selecting an arbitrary text range in the composer (the whole block
  is anchored today);
- replies within a thread beyond the comments sharing a `discussion_id`;
- resolved/unresolved thread state.

## Acceptance criteria

P1. **Given** two comments with the same discussion and anchor, **when** the
panel renders, **then** they appear as one thread in creation order.

P2. **Given** a comment anchored to a block range, **then** the thread quotes
that range.

P3. **Given** a comment without an anchor or from another page, **then** it is
not in the inline section.

P4. **Given** a delete of an inline comment, **then** the server call succeeds
and the comment disappears from the workspace.

P5. **Given** a selected block and comment text, **when** the composer submits,
**then** a v2 comment is created with a `0..block text length` anchor and appears
in the inline section.

## Test scenarios

- `tests/ui.rs`
  (`inline_threads_group_anchored_comments_by_discussion`,
  `inline_comment_input_anchors_the_whole_block`,
  existing page/sidebar tests).
- `tests/comments.rs` (anchor validation and CRUD).
- `cargo test --features desktop --test ui`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Thread projection | `ui/comments.rs` |
| Panel rendering | `ui/render/content/page.rs` |
| Delete command | `ui/commands.rs`, `ui/apply/page_actions.rs` |
| Locale strings | `locales/en.yml`, `pt-BR.yml`, `eo.yml` |

## Plan / tasks

- [x] Group anchored comments by discussion and block order.
- [x] Render the quote and comment rows in the page panel.
- [x] Add the block picker + composer and create a v2 comment.
- [x] Add comment deletion through the v2 client.
- [x] Localize the new strings.
- [x] Add the projection and input tests.
