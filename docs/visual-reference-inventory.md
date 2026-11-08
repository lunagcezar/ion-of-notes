# Visual reference inventory — `reference/` and `screenshots/`

## Purpose

There are **two visual evidence sets**, and both matter:

| Folder | Count | What it is |
| ------ | ----: | ---------- |
| workspace `reference/` | **98** | The primary visual source. `picture-1` … `picture-29` are the original dark-build set; `picture-30` … `picture-98` are the later build. |
| `app/screenshots/` | **72** | Local, git-ignored pictures of the later build. They overlap the later `reference/` set and add a few states. |

They are **visual and behavioral evidence**, not source material. The later
set includes surfaces that the original 29 do not. The count grew over time:
older specs and changelog entries say "29" because that was the set at the
time; current documentation refers to the full 98 + 72.

Clean-room rule for these folders:

- observe layout, affordances, states and workflows;
- do **not** copy the Portuguese document text, brand names, logos, avatars,
  photos or long passages;
- do not treat the screenshots as proof of the server contract. They are
  **UI-observed** evidence only.

## The `reference/` set (98 pictures)

| Range | Main subject |
| ----- | ------------ |
| `picture-1` | Default dark page: sidebar, breadcrumb bar, centered page column, callout and section headings. |
| `picture-2` | Page with sidebar tree and section content. |
| `picture-3` | Inbox panel with export-ready notifications. |
| `picture-4` | Quick search overlay with grouped results and footer hints. |
| `picture-5`–`picture-11` | Application menu (File / Edit / View / History / Window / Help) and submenus. |
| `picture-12` | Share popover: invite field, owner, invited people, general access, Copy link. |
| `picture-13` | Page actions menu: Copy link, Copy page contents, Duplicate, Move to, Version history, Updates & analytics, Appearance & layout, Use with AI, Suggest edits, Notify me, Available offline, Lock page, Archive, Move to Trash, More, Last edited, word count. |
| `picture-14` | Library table (Recents, Favorites, Shared, Private, AI Meeting Notes, More) with owner/source/last-edited columns. |
| `picture-15`–`picture-29` | Settings: Preferences (appearance, input, language & time, desktop app), General, People, Sidebar, Import and related panels. |
| `picture-30`–`picture-98` | The later build: new-tab switcher, Add members, richer page actions, Library tabs, Settings General/Sidebar/People/Language/Offline, onboarding, template library, AI/agents, icon and slash pickers. |

The original 29 are the visual source of truth for the base window. The later
69 extend the evidence for the surfaces implemented in specs 023–028. Neither
set is proof of the server contract; see the clean-room rules below.

## What the `screenshots/` set covers (72 pictures)

| Range | Main subject |
| ----- | ------------ |
| `picture-1` | “Open in new tab…” switcher with search, filters, Today / Past week groups, preview pane, footer hints and install banner. |
| `picture-2` | Page with sidebar tree, callout and section headings. |
| `picture-3` | Page-icon / emoji picker over the page. |
| `picture-4` | Sidebar tree with a page context menu. |
| `picture-5` | Page with sidebar and page actions. |
| `picture-6`–`picture-9` | Page, sidebar and menu states. |
| `picture-10` | **Add members** dialog: email chips, Guest role, optional message, Send invite / Cancel. |
| `picture-11`–`picture-12` | Share popover and page actions. |
| `picture-13` | **Page actions** menu with Copy link, Copy page contents, Duplicate, Move to, Version history, Updates & analytics, Appearance & layout, Use with AI, Suggest edits, Notify me, Available offline, Lock page, Archive, Move to Trash, More, Last edited and word count. |
| `picture-14` | Page actions plus the **Use with AI** submenu (AI skill, agent instructions, meeting instruction). |
| `picture-15`–`picture-19` | Page actions / menu states and page layout. |
| `picture-20`–`picture-24` | “01 – Sobre o projeto” page with callout, bold headings and sidebar. |
| `picture-25`–`picture-36` | Slash-command menu and block-type menus. |
| `picture-37`–`picture-38` | Emoji / page-icon picker. |
| `picture-39`–`picture-40` | Search overlay with grouped results. |
| `picture-41`–`picture-43` | Settings / search and page states. |
| `picture-44`–`picture-50` | Page actions and “Biblioteca de modelos” (template library) surfaces. |
| `picture-51` | **Settings → Offline**: automatic downloads, offline pages table, “Viewing” filter, search. |
| `picture-52` | Page with a copied-text toast. |
| `picture-53`–`picture-63` | **Library** views: Recents, Favorites, Shared, Private, AI Meeting Notes, Agents, “more” tabs, table columns and empty states. |
| `picture-64` | **Settings → Language & time**: language, number format, text direction, spellchecker, week start, date format, time zone, desktop app. |
| `picture-65`–`picture-68` | Page, settings and sidebar states. |
| `picture-69` | Onboarding: **“What is this space for?”** with For work / For personal life / For school. |
| `picture-70` | **Settings → General**: workspace name, icon, custom landing page, sidebar, allowed email domains. |
| `picture-71` | **Settings → Sidebar**: other apps in sidebar, allowed domains, export workspace content/members, analytics toggle. |
| `picture-72` | **Settings → People**: directory, recent activity, hover cards, danger zone, workspace ID. |

## UI-observed surfaces and server implications

These are observations from the UI, not confirmed server behavior.

### Workspace and pages

- The sidebar has Library, Settings, a workspace row, Favorites, Private tree
  and an account row.
- Pages have icons, titles, callouts, headings, paragraphs, toggles, menus and
  an inline database.
- Page actions include duplicate, move, version history, offline, lock,
  archive, trash, page discussions and inline comments.
- A page can be marked available offline; Settings → Offline lists offline
  pages with a last-modified time.

**Server implication:** page metadata, block content, page settings
(offline/lock/archive/trash), comments/discussions and version snapshots are
all server-side state. The exact routes remain unknown; our decided contract
covers the core page CRUD and version history.

### Sharing and members

- “Invite members” and “Share” open member/invite surfaces.
- The Add members dialog accepts multiple emails, shows a Guest role, an
  optional message, and Send invite / Cancel.
- People settings include a directory, recent activity and hover cards.

**Server implication:** members, invitations, roles and per-page access are
server resources. Our decided contract has `Member`, `Role`, `Share` and
`Access`; invitation lifecycle and email delivery remain unknown.

### Search and navigation

- Search is reachable from the sidebar, `Ctrl+K`, and the “Open in new tab…”
  switcher.
- Results are grouped by Today / Past week and show a breadcrumb detail.
- The switcher has filters, a preview pane and keyboard hints.

**Server implication:** search may be server-side or client-side; our decided
contract uses `GET /api/search?q=`. Ranking and indexing remain unknown.

### Library

- Library tabs include Recents, Favorites, Shared, Private, AI Meeting Notes,
  Agents and additional “more” tabs.
- The table shows Page name, Created by, Source and Last edited time; some
  views show cards instead of a table.
- Empty states differ per tab.

**Server implication:** library views are derived from page metadata, owner,
source and timestamps. Our current Library is a simpler table; the additional
views are UI scope, not a server contract.

### Settings

- Account: profile, Preferences, Notifications, Mail & Calendar.
- Workspace: General, Requests, People, Sidebar, Analytics, Import.
- Features: Notion AI, Meeting notes, Connections, Notion MCP, Workers,
  Verified pages, Public pages, Emoji, Offline.
- General: workspace name, icon, custom landing page, sidebar behavior,
  allowed email domains.
- Sidebar: other apps, allowed domains, export workspace content/members,
  analytics.
- People: directory, recent activity, hover cards, danger zone, workspace ID.
- Language & time: language, number format, text direction, spellchecker,
  week start, date format, time zone, desktop app.
- Offline: automatic downloads and an offline-pages table.

**Server implication:** workspace settings, exports, members and analytics are
server-side configuration/resources. Our decided contract covers workspace,
members, shares, export and sync; many settings surfaces are presentation-only
today.

### AI and agents

- Later screenshots show Notion AI, AI Meeting Notes, Agents, agent provider
  filters and “Use with AI” actions.
- These are beyond the original 29-picture scope and beyond the current
  Ollama-based assistant.

**Server implication:** AI/agent features would add model/provider,
conversation and meeting resources. They are out of scope for the current
server contract and should not be inferred into it.

### Onboarding

- “What is this space for?” offers For work, For personal life and For school.
- This is a first-run experience, not part of the page tree.

**Server implication:** onboarding state may be local or server-side; unknown.
It is not part of the current decided contract.

## Implemented since the first inventory

These surfaces were gaps when this inventory was first written and are now
implemented:

| Surface | Spec |
| ------- | ---- |
| Multi-email Share dialog, roles, general access, copy link | `spec-023` |
| Library tabs (Recents / Favorites / Shared / Private), More menu, real owner/source/last-edited columns | `spec-024` |
| Settings General / Sidebar / People / Offline, persisted workspace settings, exports | `spec-025` |
| Table of contents, page discussions with add/delete comments, inline-comments preference, archive/restore, copy page contents | `spec-026` |
| Onboarding "What is this space for?" and the starter template library | `spec-027` |
| Core UI copy in English, Portuguese (Brazil) and Esperanto | `spec-028` |

## Remaining gaps

These are observations for future specs, not commitments:

- Settings rail differs: the screenshots show Mail & Calendar, Notion MCP,
  Workers, Verified pages, Public pages and Emoji; our app deliberately omits
  Mail & Calendar and uses a smaller AI/assistant surface.
- Library AI Meeting Notes and Agents views are placeholders; the screenshots
  show card views and richer columns.
- Page actions still have presentation-only entries (Move to, Updates &
  analytics, Suggest edits, Notify me, More). Table of contents, discussions,
  inline-comments preference and archive are implemented.
- Inline comments persist a preference but not block anchors.
- The invite message is captured in the UI but not sent (`MemberDraft` has no
  message field).
- The selected language is not persisted across restarts.
- Workspace deletion is disabled in this build.
- The screenshots show a newer product build; they should not silently expand
  the original 29-picture scope. Each addition needs its own spec.

## Evidence use in the clean-room log

This document is the visual companion to `docs/api-observations.md`. Every
entry here is **UI-observed**. It does not confirm routes, payloads or status
codes, and it does not authorize traffic capture. The strict B1 decision
(public behavior only) remains in force.
