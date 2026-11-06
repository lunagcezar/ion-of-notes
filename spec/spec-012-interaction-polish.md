# Spec 012 — Frameless window, tabs, shortcuts and the Ollama AI assistant

## Purpose and scope

A follow-up round driven by six notes from the person. It changes interaction
and a few visual details, without touching the workspace model, the server
contract or the seeded demo data.

1. Mail & Calendar is not implemented.
2. The AI assistant is integrated with Ollama (CLI in `PATH` or a service at a
   network port).
3. The native window frame is removed; the Rust-drawn frame is the only one.
4. The `×` in the preferences window is misaligned; fix it against the
   reference layout.
5. Implement the keyboard shortcuts found in the app menu and on the Notion
   help site.
6. The tab-strip `+` gets the behaviour shown in the two attachments.

## Evidence used

- `reference/picture-1.png` .. `picture-14.png`: dark 1322x861 window, custom
  tab strip (one tab, `+`, minimize/maximize/close), sidebar, page column.
- `reference/picture-5.png` .. `picture-11.png`: the app menu (File, Edit, View,
  History, Window, Help) with the shortcut labels (Ctrl+T, Ctrl+Shift+N,
  Ctrl+W, Ctrl+Z, Ctrl+L, Ctrl+\, F11, Ctrl+[, Ctrl+Tab, …).
- `reference/picture-15.png` .. `picture-29.png`: preferences window. The
  close `×` is a fixed top-right control in a rounded well, **above** the
  "Preferences" heading and to the left of the scrollbar; the rail has glyphs
  and sections Appearance / Input options / Language & time / Desktop app.
- `reference/picture-17.png`: "New Tab Search", "Command Search shortcut
  Shift+Ctrl+K" and "Notion AI shortcut Shift+Ctrl+J".
- `reference/picture-22.png`: the "Mail & Calendar" page — explicitly out of
  scope for this reconstruction.
- `attachments/picture-1.png` and `attachments/picture-2.png`: clicking `+`
  opens an "Open in new tab…" switcher: search field, filter row,
  "New chat" / "New page", Today / Past week page groups, a live preview pane
  and a footer (`Ctrl+↵ Open in new tab`, `Ctrl+L Copy link`,
  `Shift+Ctrl+K Command Search`).
- <https://www.notion.com/help/keyboard-shortcuts> (fetched): most-popular,
  desktop, markdown, block-edit and slash shortcuts.
- `craft.json` / `AGENTS.md`: Rust + eframe 0.37 / egui 0.37, SDD + TDD, no
  copied assets, tests in `tests/`.

## Acceptance criteria (Given / When / Then)

- **Frameless window.** Given the app starts, when `NativeOptions` is built,
  then `ViewportBuilder::with_decorations(false)` is set and the custom tab
  strip is a drag handle (`ViewportCommand::StartDrag`).
- **No mail/calendar.** Given Settings is open, when the rail is drawn, then no
  "Mail & Calendar" entry exists.
- **Preferences `×`.** Given Settings is open, when the content scrolls, then
  the close `×` stays fixed at the top-right corner and is not part of the
  heading row.
- **Add tab.** Given the tab-strip `+` is clicked (or Ctrl+T), when the
  switcher opens, then it shows the search field, filter row, New chat /
  New page, grouped pages, preview and footer hints; activating a row opens a
  new tab.
- **Shortcuts.** Given a key combination from `shortcuts.rs`, when it is
  pressed, then the matching `Command` runs and the key event is consumed so
  it does not leak into a text editor; `F1` shows the shortcut sheet.
- **AI assistant.** Given Ollama is available as a CLI in `PATH` or a service
  at `host:port`, when the user asks a question, then a background thread
  calls `ollama::generate` and the answer (or a clear error) is displayed.
- **Fidelity.** Given a normal run, when `--screenshot` is captured, then the
  default window still matches the 29 reference pictures (dark theme, custom
  chrome, sidebar, centered page column).

## Test scenarios (written before implementation)

1. `tests/shortcuts.rs`: exact `lookup` for File/Edit/View/History/Window,
   help-site and popover bindings; exactness of Ctrl+Shift+K vs Ctrl+K;
   clipboard keys are not globally consumed; `SHORTCUT_TABLE` is complete.
2. `tests/ollama.rs`: endpoint parsing/normalisation, request body, response
   parsing, tags parsing, chunked + plain HTTP bodies, status labels, empty
   prompt rejection.
3. `cargo build`, `cargo test`, `cargo run -- --screenshot` and a visual
   comparison of the fresh PNG against `reference/`.

## Plan / task list

- [x] `src/shortcuts.rs` — pure `Combo` → `Action` table + documentation table.
- [x] `src/ollama.rs` — CLI/service detection, `/api/generate`, `/api/tags`,
      pure HTTP/JSON helpers, env config, auto-selected model.
- [x] `src/main.rs` — `.with_decorations(false)`.
- [x] `src/ui/app.rs` — tabs, switcher state, keyboard translation, AI slot,
      undo/redo, zoom, fullscreen, settings page, drag handle.
- [x] `src/ui/render.rs` — multi-tab strip, `+` → switcher, markdown prefixes.
- [x] `src/ui/dialogs.rs` — switcher, AI chat in quick search, settings `×`
      fix, icons in the rail, AI settings page, shortcut sheet.
- [x] `src/lib.rs`, `docs/requirements.md`, `AGENTS.md`, `CHANGELOG.md`.
- [x] Red/Green: new tests fail without the modules, pass with them; full
      `cargo test` green; fresh screenshot compared with the references.

## Notes

- Clipboard keys (Ctrl+X/C/V/A) are intentionally left to `egui::TextEdit`;
  consuming them would break text editing, so they are documented but not
  bound globally.
- `NewWindow` (Ctrl+Shift+N) opens the same new-tab switcher: the app keeps a
  single native window with several internal tabs.
- `NOT_NOTION_SHOW` gained harmless preview values (`newtab`, `ai`,
  `shortcuts`) for capturing the new states; normal runs and the rebuild check
  never set it.
