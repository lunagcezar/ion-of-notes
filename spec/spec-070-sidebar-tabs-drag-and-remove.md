# spec-070 - Sidebar tabs, drag/drop and remove

- Status: implemented (2026-11-12)
- Related: `spec-062`, `spec-067`
- Reference: `reference/customize-sidebar.png`,
  `customize-sidebar-2.png`, `customize-sidebar-3.png`

## Purpose and scope

The customize sidebar still lacks three requested behaviors:

1. Drag and drop does not work.
2. The remove/exclude icon is not functional on every row.
3. There are no tabs. A tab is a new group of sections and shortcuts, not just
   a section inside the current list.

## Acceptance criteria

T1. **Given** a section or shortcut row in customize mode, **when** it is
dragged, **then** a drop indicator shows the target and releasing reorders it.

T2. **Given** a row's remove icon, **when** it is clicked, **then** the row is
removed from the sidebar (sections and custom shortcuts) or hidden for fixed
navigation rows; the change persists.

T3. **Given** the sidebar, **when** it has more than one tab, **then** a tab
strip appears under the search field and switching a tab shows that tab's
sections and shortcuts.

T4. **Given** customize mode, **when** "Add tab" is used, **then** a new blank
tab is created with its own section/shortcut set.

T5. **Given** a tab, **when** it is renamed or removed, **then** the tab strip
and persisted layout update; the last tab cannot be removed.

## Data model

```text
SidebarLayout
  active_tab: String
  tabs: Vec<SidebarTab>

SidebarTab
  id: String
  title: String
  sections: Vec<SidebarSection>
```

The default is one tab containing Favorites and Private. Existing
`SidebarLayout.sections` migrates into the default tab.

## Design

- Move section operations onto `SidebarTab`; `SidebarLayout` owns active tab.
- Render a compact tab strip when `tabs.len() > 1`; customize mode adds a `+`.
- Drag state stores `(tab, section, shortcut)` and a target gap; release always
  applies or cancels.
- The remove icon maps to remove for custom rows and hide for fixed rows.
- Add commands for add/rename/remove tab and move item.
