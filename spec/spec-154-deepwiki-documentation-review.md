# spec-154 - Documentation enrichment from DeepWiki

- Status: implemented (2026-11-25)
- Related: `docs/wiki-findings.md`, `docs/architecture.md`,
  `docs/refactoring-roadmap.md`, `docs/performance-roadmap.md`
- Review source: DeepWiki pages for `lunagcezar/ion-of-notes`

## Purpose and scope

An independent AI-generated wiki described the repository at commit
`75a4587ddaeff0fd2cb4cd0d9c22410c4d03dbc5`. This spec records what it added,
what was adopted, and what was stale. It does not add runtime behavior.

### Findings kept

- The client/server split, layers, ports, composition root and pipeline stages
  are accurate; the ports/adapters table and composition-root responsibilities
  were missing from `docs/architecture.md` and are now there.
- The wiki confirmed the dummy-hash login defense, session cache, single-use
  tickets, SSRF pinning, durable webhook outbox and per-block SQLite rows.
- The performance page confirmed the Fenwick height index, sidebar clip culling,
  query windows and the tracing span names; those are now listed explicitly.

### Findings reconciled

- "Normalized SQLite is deferred" was stale: pages and blocks are normalized and
  single-block writes exist (spec-133).
- "Reads project under the lock; v2 still serializes" was stale after
  spec-150/151.
- "The WebSocket accepts `?token=`" was stale after spec-148.
- "No registration, password reset or browser UI" was stale after spec-152/153.

### Source handling

The wiki is not copied as a requirement. `docs/wiki-findings.md` keeps the page
map, the adopted items, the stale claims and the open limitations, with the base
commit and retrieval date. The DeepWiki MCP server is the retrieval path.

## Acceptance criteria

P1. **Given** the wiki pages, **then** `docs/wiki-findings.md` names the source
URL, base commit, retrieval date and page-to-document map.

P2. **Given** a wiki claim that is now false, **then** the reconciliation table
says what changed and which spec superseded it.

P3. **Given** an adopted finding, **then** it lives in the owning document
(architecture, refactoring, performance, glossary), not only in the review log.

P4. **Given** the documentation gate, **then** links, tables and Mermaid fences
remain valid.

## Test scenarios

- `pwsh -File scripts/check-docs.ps1`.
- `cargo test --no-default-features --test docs`.
- Manual comparison of `docs/wiki-findings.md` with the wiki structure.

## Plan / tasks

- [x] Retrieve the wiki structure and contents through the DeepWiki MCP server.
- [x] Add the findings/reconciliation page.
- [x] Enrich architecture, refactoring, performance and glossary.
- [x] Document the maintenance rule in `AGENTS.md`.
