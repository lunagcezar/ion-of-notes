# Spec 009 — Visual fidelity against the reference pictures

## Purpose

The person added 29 pictures of the original app. They show a dark desktop
workspace that the first version only approximated. This spec closes the gap
between the estimated layout and what the pictures actually show, and adds
the opened states the pictures document (Library, Share, Inbox, Settings).

Scope of this session: window chrome, sidebar, breadcrumb bar, page column,
typography, the floating controls, the three opened states, and the demo page
content. The client/server contract, persistence, and the block editor are not
changed.

## Evidence used

- `reference/picture-1.png` … `picture-29.png` (all opened and reviewed).
- `capture/README.md` (clean-room boundary; nothing inside the original was read).
- `docs/unknowns.md` (U-01 resolved by observation: dark theme).

## What the pictures establish

1. A custom top strip spans the whole window: sidebar toggle and quick-note
   icons, back/forward arrows, one active page tab, a new-tab button, and
   minimize / maximize / close controls on the right.
2. The sidebar is about a fifth of the window width. It starts with a
   "Search or ask" box carrying a `Ctrl+K` hint, then Library, Settings and a
   page row, a Favorites section, and a Private tree. Its bottom holds a
   "Customize sidebar" row, a card, and a user row.
3. A breadcrumb bar sits under the top strip: page icon + crumbs on the left,
   Share, link, star and more actions on the right.
4. The page column is centered in the content area. A large book icon sits
   above a very large bold title; below it is a grey callout, bold section
   headings, body paragraphs and a short list. Block handles are not visible
   until a row is hovered.
5. Blue is the only strong accent (buttons, toggles, links).
6. A small circular button floats at the bottom-right of the content.
7. Opened states: a full-width Library table, a Share popover, an Inbox panel,
   and a large two-column Settings window.

## Acceptance criteria

- **Given** the app starts with no saved data, **when** the first frame is
  drawn, **then** the window is 1322×861, dark, and shows the top strip,
  sidebar, breadcrumb bar and centered page column described above.
- **Given** the default page, **when** it is drawn, **then** the first block is
  a callout and the page contains bold section headings.
- **Given** a short or long page title, **when** the tab strip renders it,
  **then** short titles are unchanged and long titles are shortened with an
  ellipsis.
- **Given** a bold display font is available, **when** a title or heading is
  drawn, **then** it uses the strong font family at the requested size.
- **Given** the user clicks Library, Share, Inbox or Settings, **when** the
  frame is drawn, **then** the corresponding opened state from the pictures is
  shown and can be closed again.
- **Given** `--screenshot <file.png>`, **when** ~120 frames have been drawn,
  **then** a non-blank PNG of the whole window is written and the app exits.

## Test scenarios (written before implementation)

| # | Scenario | File |
| - | -------- | ---- |
| T-01 | The chrome color is distinct from the content color and differs per theme | `tests/fidelity.rs` |
| T-02 | `strong_font(42.0)` keeps the requested size | `tests/fidelity.rs` |
| T-03 | `tab_title` keeps short titles and truncates long ones with an ellipsis | `tests/fidelity.rs` |
| T-04 | The seeded home page opens with a callout and has bold section headings | `tests/fidelity.rs` |
| T-05 | Existing integration tests still pass (`cargo test`) | `tests/` |

## Plan

1. Add the failing tests and observe the red step (`cargo test`).
2. `palette.rs`: chrome color, font installation, `strong_font`, `tab_title`.
3. `icons.rs`: new shape primitives for the chrome, sidebar and page icon.
4. `app.rs`: opened-state flags, navigation history, and panel wiring.
5. `render.rs`: top strip, sidebar, breadcrumbs, page column, Library view,
   floating controls.
6. `dialogs.rs`: enlarged Settings, Share popover, Inbox panel.
7. `model.rs`: neutral demo content shaped like the pictured page.
8. `main.rs`: window size matching the pictures.
9. Green: `cargo build`, `cargo test`, `--screenshot` compared by eye.
10. Update `CHANGELOG.md`, `docs/requirements.md`, `docs/unknowns.md`,
    `AGENTS.md` if the foundations changed.

## Results (green)

- Red observed: `cargo test --test fidelity` failed to compile with ten errors
  (`chrome`, `strong_font`, `tab_title`, and the reshaped demo page were all
  missing).
- Green: all six acceptance criteria above are met. `cargo build` is clean and
  `cargo test` is green (57 tests). `--screenshot` writes a non-blank 1322×861
  PNG and exits.
- Each opened state was rendered with the preview hook and compared by eye
  with pictures 3, 12, 14 and 15–29; the two-column Settings layout was
  corrected after the first capture.
- Remaining estimates: the OS menu bar (pictures 5–11) and the full
  more-actions command list (picture 13) are documented in
  `docs/unknowns.md` (U-05, U-06) rather than silently dropped.

## Out of scope

- System menus (File / Edit / View / …) that belong to the operating system.
- Real accounts, invitations, network calls, or any branding from the original.
- Any content read from inside the original program (clean room is preserved).

> **Historical paths (2026-11-08):** the plan above names `render.rs`,
> `dialogs.rs`, `app.rs` and `model.rs` as they existed in this session.
> `spec-017` later split the UI into `ui/render/*`, `ui/dialogs/*`,
> `ui/app.rs` (state shell), `ui/apply.rs`, `ui/commands.rs`, `ui/view.rs` and
> `ui/input.rs`; `spec-018` split `model.rs`. The behavior described here is
> unchanged. See `docs/architecture.md` for the current map.
