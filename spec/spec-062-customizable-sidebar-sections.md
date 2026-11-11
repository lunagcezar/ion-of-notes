# spec-062 - Customizable sidebar sections and shortcuts

- Status: implemented (2026-11-11)
- Related: `spec-013`, `spec-017`, `spec-057`, `spec-058`
- Reference: `reference/customize-sidebar.png`,
  `reference/customize-sidebar-2.png`, `reference/customize-sidebar-3.png`
- Contract docs: `docs/data-model.md`, `docs/requirements.md`

## Purpose and scope

`spec-057` added an in-place customize-sidebar mode with drag handles, a Done
button and simple Add shortcut/Add section menus. It does not yet let the person
manage the sidebar: rows have no settings or exclude actions, drag-and-drop does
not persist, and the Add menus are short placeholders instead of the reference
menus. The reference also shows sidebar sections that behave like tabs, each
with its own set of shortcuts.

This spec turns the sidebar into a persisted, editable layout:

- sections (tabs) with a title and a shortcut set;
- shortcuts to pages, views, settings and creation actions;
- per-row settings, exclude and drag-and-drop;
- the full Add shortcut and Add section menus with search and submenus.

Out of scope:

- A general drag-and-drop framework outside the sidebar.
- Chats, meetings, agents and databases as new backend features. Menu entries
  for them are present and can be added, but they resolve to the closest
  existing view until those features exist.
- Reordering the main navigation rows (Library/Settings/Inbox) outside the
  layout model.

## Evidence

`reference/customize-sidebar.png` shows the edit mode: rows with six-dot
handles, Home with a `+`, Library, Settings, the current page, Add shortcut,
Favorites, Add section, a blue Done button and the invite/account group.

`reference/customize-sidebar-2.png` shows the Add section menu: a search field,
Pages (Recents, Private, Shared, Database view with a submenu), Chats (Pinned,
Chats, Agents), Meetings (Upcoming events, AI Meeting Notes) and Advanced
(Developer).

`reference/customize-sidebar-3.png` shows the Add shortcut menu: a search field,
Create (New page, New chat, New meeting note, New database, New agent) and Go to
(Page submenu, Chat thread submenu, Settings, Meeting Notes, Agents, Skills, My
Tasks, Calendar, Marketplace).

Current code seams:

| Area | Code |
| ---- | ---- |
| Sidebar render and customize rows | `src/ui/render/sidebar.rs` |
| Sidebar menus | `src/ui/dialogs/menus.rs` (`render_sidebar_menu`) |
| Commands and app state | `src/ui/commands.rs`, `src/ui/app.rs`, `src/ui/view.rs` |
| Settings model | `src/model/settings.rs` |
| Persistence | `src/store.rs`, `src/store/sqlite.rs` |
| Search helpers | `src/ui/settings_search.rs` |

## Data model

```text
SidebarLayout
  sections: Vec<SidebarSection>

SidebarSection
  id: String
  title: String
  source: SectionSource     // Shortcuts | Pages | Recents | Private | Shared | Database
  shortcuts: Vec<SidebarShortcut>
  hidden: bool
  collapsed: bool

SidebarShortcut
  id: String
  label: String
  kind: ShortcutKind        // Page | Library | Settings | Inbox | Search |
                            // NewPage | Chat | Meeting | Database | Agent | Custom
  target: String            // page id, view id or empty
  hidden: bool
```

Defaults migrate the current sidebar into two sections:

- `Favorites` with one shortcut per favorite page.
- `Private` with the page tree as its source.

The layout lives in `WorkspaceSettings`, so it is per workspace, persisted by
the normal workspace diff and available in JSON and SQLite.

## Acceptance criteria

### Edit mode and row actions

C1. **Given** "Customize sidebar", **when** it is invoked, **then** the sidebar
enters edit mode with handle rows, the Done button and the reference bottom
group; it does not open Settings.

C2. **Given** a row in edit mode, **when** it is hovered, **then** it shows a
drag handle, a settings action and an exclude action without shifting the row
label.

C3. **Given** a row's settings action, **when** it is clicked, **then** a small
menu offers Rename (shortcuts and sections), Hide/Exclude and Remove (shortcuts
only).

C4. **Given** the exclude action, **when** it is used, **then** the row or
section disappears from the normal sidebar but stays in the layout as hidden;
"Show excluded" in edit mode reveals it again.

C5. **Given** a renamed section or shortcut, **when** the rename is confirmed,
**then** the new label is shown immediately and persists after restart.

### Add shortcut menu

C6. **Given** Add shortcut, **when** the menu opens, **then** it has a search
field, a Create group (New page, New chat, New meeting note, New database, New
agent) and a Go to group (Page, Chat thread, Settings, Meeting Notes, Agents,
Skills, My Tasks, Calendar, Marketplace).

C7. **Given** the search field, **when** text is typed, **then** the visible
items are filtered by label and the menu stays open.

C8. **Given** Page or Chat thread, **when** it is hovered, **then** a submenu
lists existing pages or threads; choosing one adds a shortcut to the active
section.

C9. **Given** a Create action, **when** it is chosen, **then** the corresponding
page/view is created (when implemented) and added as a shortcut; unimplemented
actions are marked informational instead of dead.

### Add section menu

C10. **Given** Add section, **when** the menu opens, **then** it has a search
field and the categories Pages, Chats, Meetings and Advanced from the
reference.

C11. **Given** Database view, **when** it is hovered, **then** a submenu lists
data sources; choosing one creates a database section.

C12. **Given** a section choice, **when** it is selected, **then** a new section
(tab) is created with that source and its own shortcut set, inserted after the
active section.

C13. **Given** "Blank section" (or an empty custom source), **when** it is
created, **then** the sidebar shows a new named section with no shortcuts and
Add shortcut can populate it.

### Drag and drop

C14. **Given** a shortcut row, **when** it is dragged within its section,
**then** a drop indicator shows the insertion point and releasing reorders it.

C15. **Given** a shortcut row, **when** it is dragged onto another section,
**then** it moves to that section at the indicated position.

C16. **Given** a section header, **when** it is dragged, **then** the section
order changes; the active section and its shortcuts move with it.

C17. **Given** any reorder, **when** the app restarts, **then** the order is
unchanged.

### Normal sidebar

C18. **Given** a customized layout, **when** edit mode is closed, **then** the
sidebar renders the sections in order, hides excluded rows, and keeps the
existing navigation/search/bottom group behavior.

C19. **Given** a shortcut to a page, **when** it is clicked, **then** the page
opens in the current tab and the active section is remembered.

## Test scenarios (written before implementation)

- **Layout model:** default migration creates Favorites + Private; add shortcut,
  add section, rename, exclude/restore, remove and reorder round-trip through
  JSON and SQLite.
- **Pure menus:** shortcut and section filters match labels; Database submenu
  lists data sources; submenu selection returns the right command.
- **Drag model:** move within a section, across sections and sections
  themselves; invalid drops are ignored; the result is persisted.
- **UI:** preview states `customize-sidebar`, `customize-shortcut`,
  `customize-section`, `customize-row-settings`, `customize-drag`,
  `customize-excluded`.
- **Regression:** existing sidebar settings toggles, favorites, tree and
  search; full gate.

## Design

- Add `SidebarLayout`, `SidebarSection`, `SidebarShortcut`, `SectionSource` and
  `ShortcutKind` to `model/settings.rs` with serde defaults and a
  `default_layout()` migration helper.
- Put layout operations in a pure `ui/sidebar_layout.rs` module:
  `add_shortcut`, `add_section`, `rename`, `set_hidden`, `remove`, `move_item`,
  `move_section`. The render layer only reads the result.
- Commands: `OpenSidebarMenu(SidebarMenu)` grows to carry a query and target
  (`Shortcut`, `Section`, `RowSettings(id)`, `ShowExcluded`), and
  `MoveSidebarItem`/`MoveSidebarSection` apply drops.
- The Add shortcut/section menus are one reusable searchable popover with
  grouped items and optional submenus. Keep the data tables pure so the filter
  is testable.
- Drag state lives in app state (`SidebarDrag { section, index, target }`);
  release always applies or cancels, following the same lesson as `spec-060`.
- Edit mode renders the same rows as the normal sidebar plus hover actions; the
  normal render skips `hidden` rows and sections.

## Plan / tasks

- [x] Model: layout types, defaults, migration and operations.
- [x] Store: JSON/SQLite round-trip and diff behavior.
- [x] Commands and app state for menus, rename, exclude, drag.
- [x] Sidebar render: sections, tabs, hover actions, Done.
- [x] Add shortcut and Add section menus with search/submenus.
- [x] Preview states, screenshots and full gate.
- [x] Update data model, requirements, glossary and changelog.

## Decisions and open questions

- Decision: sections are the sidebar "tabs"; each section owns a shortcut set.
- Decision: exclude is reversible and never deletes; remove is explicit.
- Open: whether sections can be collapsed independently. The model has a
  `collapsed` flag; the first implementation may render all sections expanded.
- Open: whether the default Favorites section should stay a shortcut list or
  become a source-based view. The default is a shortcut list.
- Open: how unimplemented Create actions behave. The default is an
  informational row with a "coming later" hint, not a disabled-looking button.
