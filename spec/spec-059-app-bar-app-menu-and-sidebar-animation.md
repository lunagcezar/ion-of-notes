# spec-059 - App bar, app menu and sidebar animation

- Status: implemented (2026-11-11)
- Related: `spec-013`, `spec-017`, `spec-057`
- Reference: `reference/app-bar.png`, `reference/app-menu.png`,
  `reference/customize-sidebar.png`

## Purpose and scope

The top chrome does not match the reference. The app menu and sidebar toggle
sit in the same row as back/forward and the tabs, there is a hardcoded gap, the
notification and compose controls are missing, and the sidebar opens and closes
instantly. The app menu is one flat list instead of the reference's sections
with submenus.

This spec restructures the title bar into two regions:

```text
+--------------------------+----------------------------------------------+
| sidebar-width container  |  outside the sidebar container               |
| [menu][toggle][spacer]   |  [back][forward] [tabs...] [+]               |
| [notifications][compose] |                                              |
+--------------------------+----------------------------------------------+
| window controls pinned to the far right                                  |
+--------------------------------------------------------------------------+
```

It also animates the sidebar width and rebuilds the app menu as one popover
with major sections and hover submenus.

Out of scope:

- New notification storage; the badge uses the existing inbox count.
- A full chat/meeting/database feature. The compose menu can list them, but
  only actions that already exist are enabled.
- Native menus or OS-level menu bars.

## Evidence

`reference/app-bar.png` shows, from left to right:

1. A rounded app-menu button.
2. A sidebar show/hide button.
3. A wide empty area.
4. An inbox icon with a red badge.
5. A compose/new-page icon.
6. A separator.
7. Back and forward buttons.
8. The active tab with its page icon and title.
9. The `+` tab button.
10. Window minimize/maximize/close at the far right.

`reference/app-menu.png` shows one menu surface whose left column lists File,
Edit, View, History, Window and Help. Hovering File opens a second column with
New Tab, New Window, Reopen Last Closed Tab, Close Tab, Print, Restart, Exit
Without Saving Tabs and Exit, with shortcut hints.

Current code seams:

| Area | Code |
| ---- | ---- |
| Title bar and tabs | `src/ui/render/chrome.rs` |
| App menu | `src/ui/dialogs/menus.rs` (`render_app_menu`, `popup_menu`) |
| Sidebar panel | `src/ui/app.rs` (`Panel::left("sidebar")`) |
| App state | `src/ui/app.rs`, `src/ui/view.rs`, `src/ui/commands.rs` |
| Icons | `src/ui/icons.rs` |
| Sidebar render | `src/ui/render/sidebar.rs` |

## Acceptance criteria

### App bar layout

A1. **Given** the sidebar is open, **when** the title bar renders, **then** the
left container is exactly the sidebar width (266 logical points), and its
contents are ordered: app menu, sidebar toggle, flexible space, notification
control, compose control.

A2. **Given** the sidebar is closed, **when** the title bar renders, **then**
the left container shrinks to the compact width that still holds the app menu
and toggle, and the back/forward buttons and tabs move left without overlap.

A3. **Given** the title bar, **when** it renders, **then** back/forward, the tab
strip, the `+` button and the window controls are outside the sidebar container
and there is no hardcoded 186-point spacer.

A4. **Given** the notification control, **when** there are unread inbox items,
**then** a small red badge shows the count; with no items, the control has no
badge.

A5. **Given** the compose control, **when** it is clicked, **then** a small menu
opens with New page, New chat, New meeting note and New database. Actions that
are not implemented are shown as informational rows, not dead buttons.

A6. **Given** an empty area of the title bar, **when** the pointer drags it,
**then** the window moves; dragging over a button, tab or menu control does not
start a window drag.

### Sidebar animation

A7. **Given** the sidebar is open, **when** the toggle is clicked, **then** the
sidebar width animates to zero over about 150 ms with an ease-out curve, the
left app-bar container follows the same value, and the content area reflows
without a flash.

A8. **Given** the sidebar is closed, **when** the toggle is clicked, **then**
the sidebar animates back to 266 points and its scroll position and selection
are preserved.

A9. **Given** the animation is in progress, **when** the toggle is clicked
again, **then** the animation reverses from the current width instead of
jumping to the endpoint.

### App menu

A10. **Given** the app menu button, **when** it is clicked, **then** one menu
surface opens with the major sections File, Edit, View, History, Window and
Help.

A11. **Given** a major section, **when** it is hovered or clicked, **then** its
submenu opens in a second column to the right, aligned with that section, and
the section is highlighted.

A12. **Given** a submenu item, **when** it is clicked, **then** the command runs
and the whole menu closes. Shortcut hints are right-aligned.

A13. **Given** the menu is open, **when** Escape is pressed or the pointer
clicks outside, **then** the menu closes. The submenu does not overlap or shift
the section column.

## Test scenarios (written before implementation)

- **Pure layout:** `app_bar_layout(open, sidebar_width)` returns ordered rects;
  the left container equals the sidebar width when open and the compact width
  when closed; back/forward start at the container's right edge.
- **Animation:** an `AnimatedWidth` helper reverses from the current value and
  reaches both endpoints within the duration; repeated toggles do not jump.
- **Menu data:** every section has at least one item; every item has a label and
  command; no two sections share a title.
- **Screenshots:** `appbar-open`, `appbar-closed`, `appbar-compact`,
  `app-menu-file`, `app-menu-view`, `compose-menu`.
- **Regression:** existing tab close/select tests, window-control commands and
  the full gate.

## Design

- Split `render_title_bar` into `render_app_bar` (left container + back/forward
  + tabs + window controls) and a pure `app_bar_layout` helper. The helper is
  testable without egui.
- Introduce `SIDEBAR_WIDTH`, `APP_BAR_COMPACT_WIDTH` and `APP_BAR_HEIGHT`
  constants in one place so the panel and the bar cannot drift.
- The sidebar panel stays mounted and its width is
  `ctx.animate_bool_with_time(id, open, 0.15)` mapped to
  `[compact, SIDEBAR_WIDTH]`; content is clipped while closing. The same value
  drives the left app-bar container.
- Add `app_menu_section: Option<usize>` to app state. `render_app_menu` paints
  the section column and the active submenu from a `MenuSection` table; hover
  or click updates the state.
- The notification control reads the existing inbox count (or zero). The
  compose control reuses the `Command` vocabulary and disables unimplemented
  rows explicitly.
- Keep the window drag interaction on a rect that excludes every interactive
  child rect.

## Plan / tasks

- [x] Extract `app_bar_layout` and the width constants with tests.
- [x] Rebuild `render_title_bar` into the sidebar container + outside region.
- [x] Add notification and compose controls.
- [x] Animate the sidebar and keep content state across the transition.
- [x] Rebuild `render_app_menu` with sections and submenus.
- [x] Add preview states and screenshots.
- [x] Run the full gate and update the changelog.

## Decisions and open questions

- Decision: the left container is exactly the sidebar width, so the tab strip
  begins where the content pane begins.
- Decision: the notification badge uses the inbox count; no new notification
  model in this spec.
- Open: whether the compose menu should create a chat/meeting note now or only
  after those features exist. The default is to show them as informational.
- Open: animation duration is 150 ms; the person may prefer 120-180 ms. This is
  a visual constant, not a contract.
