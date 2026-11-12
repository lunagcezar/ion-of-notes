# spec-077 - Review response documentation and roadmap

- Status: implemented (2026-11-12)
- Related: `spec-073`, `spec-074`, `spec-075`, `spec-076`
- Review sources: the 2026-11-12 external reviews (Kimi, ChatGPT, Claude)

## Purpose and scope

The external reviews produced a large set of security, persistence,
authorization, performance, testing, product and operations findings. The
implementation cycle fixed the release-blocking and high-priority items and
deferred the larger product/scale work.

This spec records the response in dedicated documents so the status of every
finding is discoverable without reading the review transcripts.

Scope:

- `docs/security-hardening.md`;
- `docs/authorization.md`;
- `docs/persistence-integrity.md`;
- `docs/performance-roadmap.md`;
- `docs/review-response.md`;
- updates to `AGENTS.md`, `README.md` and `docs/index.md`.

Out of scope:

- implementing the deferred product/scale items (they are documented with a
  measurement or design prerequisite);
- a full rewrite of historical specs.

## Acceptance criteria

P1. **Given** a reviewer finding, **when** a contributor looks it up, **then**
`docs/review-response.md` says whether it is implemented, partial or deferred
and links the relevant spec or document.

P2. **Given** the security boundary, **when** an operator reads the docs,
**then** `docs/security-hardening.md` explains the local/production auth modes,
owner password rules, login limits, request bounds, session storage and webhook
egress policy.

P3. **Given** an authorization change, **when** a contributor reads the docs,
**then** `docs/authorization.md` explains roles, capabilities, page access
resolution and the known nested-resource gap.

P4. **Given** a persistence change, **when** an operator reads the docs, **then**
`docs/persistence-integrity.md` explains the atomic snapshot, failure reload,
JSON limitation and a backup/restore drill.

P5. **Given** a performance proposal, **when** a contributor reads the docs,
**then** `docs/performance-roadmap.md` lists the deferred items and the
measurements required before changing the model.

## Test scenarios

- Documentation link, Mermaid and table checks pass.
- The full quality gate passes (no code behavior change in this spec beyond
  the preceding specs).
- A manual reading check: every implemented row in `docs/review-response.md`
  points to a spec and a test file.

## Design

- One document per concern: security, authorization, persistence, performance.
- `docs/review-response.md` is the index: finding → response → spec → evidence.
- `AGENTS.md` gets a short review-response pointer and the security rules that
  every future change must preserve.
- `README.md` links the new documents in the documentation table.
- `docs/index.md` adds the documents to the catalog and reading order.

## Plan / tasks

- [x] Write the five response documents
- [x] Update `docs/index.md`, `AGENTS.md` and `README.md`
- [x] Run the documentation checks and full gate
- [x] Commit
