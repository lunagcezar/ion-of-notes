# spec-057 - UI quality and interaction fidelity

- Status: plan (2026-11-11)
- Related: `spec-009`, `spec-013`, `spec-017`, `docs/visual-reference-inventory.md`
- Reference images: `reference/customize-sidebar.png`,
  `customize-sidebar-2.png`, `customize-sidebar-3.png`,
  `new-workspace.png`, `workspace-switcher.png`, `block-type.png`, `share.png`

## Purpose and scope

This is a quality pass over the existing window. It turns the review into
observable fixes grouped by surface, without changing the domain model unless a
UI behavior genuinely needs a small addition.

The review has five independent areas. They are one spec because they share
the same visual contract and screenshot harness, but the plan is phased so each
phase can be one implementation session:

- **A. Layout and chrome** - sidebar padding, tab close, sharing layout, page
  icon centering and settings dialog padding/alignment.
- **B. Editor interaction** - select-all, block drag preview and slash menu.
- **C. Page icon picker** - real emoji catalog/search, a monochrome icon pack,
  no stray blue underline.
- **D. Sidebar and workspace** - customize-sidebar mode, new-workspace dialog,
  workspace switcher popover and correct favorite targets.
- **E. Settings behavior** - wire the placeholder settings and fix the Ollama
  test state.

Out of scope:

- No redesign of the application shell, no new web view and no copied assets.
- No broad backend feature work; where a UI item needs persistence or a new
  DTO, the smallest additive change is called out.
- Phase D may be split if multi-workspace persistence is larger than the UI
  dialog; the spec records that decision rather than hiding it.

## Evidence

The person reviewed the running window against `reference/`:

1. Main sidebar right padding is larger than the left.
2. Ctrl+A and mouse drag cannot select text beyond the current block.
3. Tabs have no "x" close control like the original.
4. The six-dot handle is used for selection as well as drag; dragging only
   shows a blue line instead of a live block preview.
5. The page-icon picker has an incomplete emoji set, a non-working emoji
   search, emojis in the Icons tab and a stray blue underline.
6. "Customize sidebar" opens a settings page instead of replacing the sidebar
   with shortcuts/sections (`customize-sidebar*.png`).
7. Most settings are placeholders.
8. "Create new workspace" opens Settings General instead of the
   `new-workspace.png` dialog.
9. The workspace switcher is not a dropdown and help/four-point controls sit
   inside its hover area (`workspace-switcher.png`).
10. Favoriting a dynamic page (Library) favorites the last static page.
11. Typing `/` immediately removes the slash instead of keeping it while the
    block-type menu filters (`block-type.png`).
12. Sharing panel margins, padding and positioning are wrong (`share.png`).
13. Page icon glyph is not vertically centered.
14. The Ollama test button stays at "Waiting for Ollama."
15. Settings content is glued to the rail (no content padding).
16. Settings rail has unequal left/right padding.
17. Settings search placeholder is not vertically aligned.
18. Settings close "x" sits on the dialog border.

Current code seams:

| Area | Code |
| ---- | ---- |
| Sidebar rows and padding | `src/ui/render/sidebar.rs` |
| Tabs and chrome | `src/ui/render/chrome.rs` |
| Blocks, selection and drag | `src/ui/render/content/blocks.rs`, `content/page.rs` |
| Slash menu | `src/ui/render/content/blocks.rs`, `src/ui/dialogs/menus.rs` |
| Page icon picker | `src/ui/page_icon.rs`, `src/ui/icons.rs`, `src/ui/palette.rs` |
| Settings dialog | `src/ui/dialogs/settings.rs`, `settings_pages/*` |
| Sharing | `src/ui/dialogs/sharing.rs` |
| Workspace switcher | `src/ui/dialogs/menus.rs`, `src/ui/render/sidebar.rs` |
| Favorites/commands | `src/ui/apply/navigation.rs`, `src/ui/commands.rs` |
| Ollama | `src/ui/apply/ai.rs`, `src/ollama.rs` |

## Acceptance criteria

### Phase A - layout and chrome

A1. **Given** the sidebar, **when** it renders, **then** the left and right
content padding are equal, and the search box, section labels, page rows and
bottom group share the same content width.

A2. **Given** two or more tabs, **when** a tab is hovered or active, **then** an
"x" control appears inside the tab; clicking it closes that tab without
selecting it. The last tab still closes the window as today.

A3. **Given** the Share popover, **when** it opens, **then** the title, tabs,
invite field, member rows, general access, footer and close control follow the
`share.png` spacing and alignment, with no clipped or glued content.

A4. **Given** a page icon in the page header, **when** it renders, **then** the
glyph is vertically centered in its icon well.

A5. **Given** the settings window, **when** it opens, **then** the content pane
has its own padding, the rail has equal left/right padding, the search
placeholder is vertically centered and the close "x" sits inside the dialog
padding rather than on the border.

### Phase B - editor interaction

B1. **Given** the page editor, **when** Ctrl+A is pressed, **then** all text in
the current page is selected; dragging across block boundaries extends the
selection, and Ctrl+C copies the selected text. Clicking elsewhere clears it.

B2. **Given** a block's six-dot handle, **when** it is dragged, **then** the
handle only drags (the "+" remains the insert/menu control) and a live ghost
of the block follows the pointer with a target gap/line; dropping reorders the
block.

B3. **Given** an empty block, **when** `/` is typed, **then** the slash remains
in the text and the block-type menu opens below it. Typing filters the menu;
selecting a type replaces the slash text. Escape or "Close menu" leaves the
typed `/` and continues normal writing.

### Phase C - page icon picker

C1. **Given** the Emoji tab, **when** the picker opens, **then** it shows a
complete, real emoji catalog (not replacement characters) and the filter
matches emoji names/keywords.

C2. **Given** the Icons tab, **when** it opens, **then** it shows only the
monochrome icons already bundled in `ui/icons.rs` (no external icon pack), and
the filter matches the icon names.

C3. **Given** the picker, **when** a tab is active, **then** there is no stray
blue underline; selection uses the normal hover/selected background.

### Phase D - sidebar and workspace

D1. **Given** "Customize sidebar", **when** it is invoked, **then** the sidebar
enters an edit mode with a Done control, shortcut and section rows, Add
shortcut/Add section menus, and the reference bottom invite/account group; it
does not open a settings page.

D2. **Given** "Create new workspace", **when** it is invoked, **then** the
`new-workspace.png` dialog opens with name/icon and a create action. If
multi-workspace persistence is required, the spec records that additive model
change before implementation.

D3. **Given** the bottom-left account control, **when** it is opened, **then**
the workspace switcher is a dropdown anchored above the control, with
settings, invite, accounts, workspace list, new workspace and log out. Help and
the four-point control remain outside the switcher hover area.

D4. **Given** the Library or another dynamic view, **when** the favorite
control is used, **then** it does not favorite the last visited static page;
the target is the visible page or the control is disabled in a non-page view.

Implementation note (2026-11-11): the current store keeps a single workspace,
so "Create new workspace" opens the purpose dialog from
`reference/new-workspace.png` and applies the chosen starter template. True
multi-workspace persistence is deferred to a follow-up spec rather than hidden
inside the dialog.

### Phase E - settings behavior and Ollama

E1. **Given** each settings row, **when** it is changed, **then** the value is
persisted and reflected after reopening Settings; rows that remain
intentionally informational are marked as such instead of looking interactive.

E2. **Given** the AI assistant test, **when** Ollama is unavailable or
returns an error, **then** the UI leaves "Waiting for Ollama" and shows the
actual status/error; a successful response renders the answer.

## Test scenarios (written before implementation)

- **Screenshot states:** default window, `share`, `settings`,
  `settings-general`, `settings-sidebar`, `settings-people`, `settings-offline`,
  `settings-server`, `iconpicker`, plus a new `customize-sidebar` and
  `new-workspace` preview state.
- **Manual interaction:** Ctrl+A + Ctrl+C, cross-block mouse drag, tab close,
  six-dot drag ghost, `/` filtering/escape, favorite in Library, workspace
  switcher open/close, Ollama test with the service down and up.
- **New pure tests:** emoji/icon catalog search, slash query parsing/filtering,
  favorite target selection, settings toggle persistence.
- **Regression:** existing `tests/ui.rs`, `tests/fidelity.rs`,
  `tests/screenshot.rs`, `tests/remote.rs` and the full two-pass gate.
- **Full gate:** `pwsh -File scripts/quality.ps1` and a fresh screenshot
  comparison against the named reference images.

## Design

- **A:** introduce shared sidebar/content padding constants; keep the existing
  row painter. Add a tab-close hit target inside the tab rect. Rebuild the
  share popover with explicit section spacing and a padded footer. Center the
  page icon with `Rect::from_center_size`. Give the settings content pane an
  inner margin and move the close control inside the header padding.
- **B:** add a page-level selection model (`anchor`, `focus`, selected block
  set) and render the selection highlight; Ctrl+A selects all current-page
  blocks. Add a drag ghost position and render a floating block snapshot plus a
  target insertion marker; the six-dot handle starts drag only. Keep the slash
  text in the block, use the text after `/` as the query and anchor the menu
  below the caret; Escape/Close preserves the text.
- **C:** replace the corrupted constants with a real emoji table
  (`glyph`, `name`, `keywords`) and a monochrome icon table backed by the
  existing `ui/icons.rs` shape artwork; filter by name/keywords. Remove the
  manual underline and use the normal selected background.
- **D:** add a `sidebar_customize` UI mode and the smallest settings model for
  shortcuts/sections. Build the workspace switcher as an anchored popover, not
  a free-floating window, and keep help/four-point outside its bounds. Add the
  new-workspace dialog; if the current single-workspace store cannot support
  multiple workspaces, split the persistence part into a follow-up spec and
  keep the dialog disabled with an explanation.
- **E:** audit every settings row, wire the ones with a real setting, and mark
  informational rows as such. Fix the Ollama slot/poll path so `ai_busy` always
  clears and errors are visible.

## Plan / tasks

- [x] Phase A: sidebar padding, tab close, share layout, page-icon centering,
      settings content/rail/search/close padding.
- [x] Phase B: selection model + Ctrl+A/mouse/Ctrl+C, drag ghost and six-dot
      behavior, slash menu.
- [x] Phase C: real emoji catalog/search, monochrome icon pack, remove blue
      underline.
- [x] Phase D: customize-sidebar mode, new-workspace dialog, workspace switcher,
      favorite target.
- [ ] Phase E: settings persistence/placeholders, Ollama test state.
- [ ] Add the new preview states and screenshot checks.
- [ ] Run the full gate and update the changelog/spec index.
- [ ] Commit each phase as its own spec if a phase is too large for one session.
