# Open Questions (reconstruction ledger)

The clean-room boundary forbids looking inside the original program. In the
first session `reference/` was empty and `capture/` held only its README, so the
visual design and feature scope were estimates. The person later added **29
reference pictures** in `reference/` (dark-theme screens of the original),
and pointed to the product features and pricing pages. The "free & open for
all" note that motivated `spec-007` was later **de-emphasized** (the app is
open source, not marketed as a free edition), and the person changed the
direction to a **self-hosted client/server architecture with accounts,
document sync, sharing and real-time editing**, asking for a **decided API**.
This session opened and reviewed all 29 pictures again, decided the API
(ADR-003), and rebuilt the window to keep matching them.

| # | Question | Severity | Probes that would settle it | Status / what I did |
| - | -------- | -------- | --------------------------- | ------------------- |
| U-01 | What does the original app look like on screen — layout, colors, typography, sidebar structure, opened page states? | high | Study the 29 pictures in `reference/` and the 72 additional pictures in `screenshots/`. | **Resolved (observed).** Pictures 1–14 show a 1322×861 dark window: a custom top tab strip, a ~266 px sidebar, a breadcrumb bar with Share/link/star/more, and a centered page column with a large book icon, a very large bold title, a grey callout, bold headings and paragraphs. Pictures 3, 12, 13, 14 and 15–29 show the Inbox, Share, more-actions, Library and Settings opened states. `spec-009` rebuilds the top strip, sidebar, breadcrumbs, page column, typography, floating controls, Library, Share, Inbox and Settings to match. The window now opens at 1322×861 and defaults to dark. The 72 `screenshots/` pictures show a later build with the new-tab switcher, Add members, richer page actions, Library tabs, onboarding, Settings General/Sidebar/People/Language/Offline, AI/agent surfaces and slash/icon pickers; `docs/visual-reference-inventory.md` records them as UI-observed evidence without copying text or branding. |
| U-02 | Which core features must the app include? | medium | Person's message + `/product/features` + `/pricing`. | **Refined.** `spec-007` ("free for all, no accounts") is retired. The app is open source (a license fact, not a marketing feature) and the direction is now accounts + a self-hosted server + sync + sharing + real-time editing. See `spec-011` and ADR-003. |
| U-03 | Should the server be a separate networked process or an embedded local service? | medium | Ask the person; confirm whether a second executable is wanted. | **Resolved (decided).** A **self-hosted server** is the target (ADR-003): REST/JSON + WebSocket CRDT, accounts, sync, sharing. v1 keeps the same contract in-process behind `WorkspaceApi` and the `--serve` loopback adapter; a production Axum/PostgreSQL/WebSocket server is added later behind the same seams, not by rewriting the domain. |
| U-04 | Does the person own / have permission for original branding or sample content? | low | Ask once if copying brand assets is desired. | **Closed.** Neutral placeholders (`not-notion`, original shape artwork, short English sample copy); the pictured Portuguese document text was not copied. Shipped under the MIT license. |
| U-05 | The File / Edit / View / History / Window / Help menus in pictures 5–11: are these an in-app feature to reproduce? | low | Ask the person; compare with a capture of the menus open. | **Estimate (documented, not asked).** They read as operating-system application menus rather than workspace content. The rebuilt app uses the custom tab strip from picture 1 instead; a full OS-style menu bar is out of scope for this version. Recorded here rather than silently dropped. |
| U-06 | The "more actions" popover in picture 13 lists many commands (duplicate, move, version history, …). | low | Ask the person whether the full command list is required. | **Partially resolved.** The "…" control opens a working page-actions menu with Copy link, Duplicate, Move to Trash, appearance/layout, offline/lock and **Version history** (spec-015). "Move to", "Archive" and a few analytics entries remain estimates and are not claimed to match the original. |
| U-07 | Which API should the self-hosted client/server use? | high | The person's request to "decide the API" + the Share/People/Export pictures + the supplied workspace export. | **Resolved (decided).** ADR-003 decides REST/JSON (`/api/v1`, envelopes, accounts/sessions) + a WebSocket CRDT real-time channel, a versioned sync log, sharing/permissions DTOs, and the Notion-style export tree. `spec-011` implements a tested in-process slice of that contract today. |
| U-08 | What exactly does the original server-side contract look like beyond the UI and ADR-003? | high | Public docs, UI behavior, exports, and a derived mock contract. | **In progress.** The first behavioral inventory is in `docs/api-observations.md`; every entry is marked UI-observed, inferred, decided or unknown. No original server response has been observed. |
| U-09 | Do we have explicit permission to capture traffic from a third-party instance? | high | Ask the person; confirm account ownership and terms. | **Resolved (strict scope).** The person chose public behavior only; no HAR, proxy, Wireshark, source maps or decompilation. B3 is intentionally blocked unless a new scope decision is recorded (ADR-005 B1/B8). |

## Status of this session

- All 29 pictures in `reference/` were opened and reviewed again. They are
  dark-theme screenshots of a document page, Inbox, quick search, application
  menus, Share and more-actions popovers, the Library table, and many Settings
  panels.
- U-01 is resolved by observation and implemented in `spec-009`; U-02 refined
  (the "free for all" note retired); U-03 and U-07 resolved by the ADR-003 API
  decision; U-04 closed; U-05 and U-06 are documented estimates.
- The Craft Anything helper tools (`core_person_ask`, `core_unknown_add`,
  `core_unknown_edit`, `core_unknowns_show`, `TodoWrite`, `rebuild_check`,
  `capture`) are **not exposed as tools in this builder session**. Their roles
  are covered locally: the checklist is kept in the agent's messages, the
  unknowns ledger is this file, and the equivalent checks are `cargo build`,
  `cargo test`, and running `--screenshot` with the fresh PNG compared by eye
  against the reference pictures (including the opened states, via the
  `NOT_NOTION_SHOW` preview hook).
- Verification of this session: `cargo build` clean, `cargo test` green
  (65 tests across the integration crates), `--screenshot` produces a
  non-blank 1322×861 PNG, and the Library / Share / Inbox / Settings / Tasks /
  Search states were each rendered and checked against the pictures.
