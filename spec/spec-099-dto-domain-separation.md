# spec-099 - Separate block and comment DTOs from the domain

- Status: implemented (2026-11-12)
- Related: `spec-032`, `spec-035`, `spec-085`
- Review sources: Claude and ChatGPT (DTOs mixed into domain modules)

## Purpose and scope

`model/block.rs` and `model/comment.rs` declared both the domain entity and the
v2 wire shapes, so a contract change touched a domain file. This spec moves
those DTOs into a dedicated `model/dto/` module:

- `model/dto/block.rs`: `ParentDto`, `BlockDto`, `BlockInput`,
  `BlockChildrenRequest`, plus `Block::to_dto` and `Block::from_input`;
- `model/dto/comment.rs`: `CommentDto`, `CommentInput`, `AttachmentInput`,
  `CommentDisplayNameDto`;
- `model/dto.rs` is the facade and re-exports them;
- `model.rs` re-exports the same public names, so `crate::model::BlockDto` and
  the crate-level re-exports keep working.

Out of scope:

- moving page, database, view, file or user DTOs;
- changing any wire shape, field or conversion rule;
- changing the v1 API.

## Acceptance criteria

P1. **Given** a v2 block create/read/update, **when** it runs, **then** the JSON
shape and conversion are unchanged.

P2. **Given** a v2 comment create/read, **when** it runs, **then** the JSON shape
and attachment/display-name rules are unchanged.

P3. **Given** the public crate surface, **when** a consumer imports
`ion_of_notes::BlockDto` or `ion_of_notes::model::BlockDto`, **then** it still
resolves.

P4. **Given** the domain modules, **when** they are read, **then** they contain
no v2 DTO declarations.

## Test scenarios

- `tests/blocks.rs`
- `tests/comments.rs`
- `tests/properties.rs`
- `tests/contract.rs`
- Full quality gate

## Design

- DTOs depend on the domain; the domain does not depend on the DTO module.
- Inherent impls (`to_dto`, `from_input`) may live beside the DTO.
- The `model` facade keeps the historical path stable for compatibility.

## Plan / tasks

- [x] Add `model/dto/` with block and comment DTOs
- [x] Move the conversions
- [x] Update model imports and re-exports
- [x] Run the full gate and commit
