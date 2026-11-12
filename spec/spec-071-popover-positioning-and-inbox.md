# spec-071 - Popover positioning and full-height inbox

- Status: implemented (2026-11-12)
- Related: `spec-059`, `spec-063`
- Reference: `reference/app-bar.png`, `reference/app-menu.png`,
  `reference/customize-sidebar.png`

## Purpose and scope

The workspace switcher, four-point menu and app menu are still not anchored to
the controls they belong to, and the inbox popover does not use the full
available height or start at the sidebar edge.

## Acceptance criteria

P1. **Given** the workspace switcher, **when** it opens, **then** its left edge
aligns with the account row content and its bottom edge is just above the row.

P2. **Given** the four-point menu, **when** it opens, **then** its right edge
aligns with the four-point control and it sits just above the row.

P3. **Given** the app menu, **when** it opens, **then** its left edge aligns
with the app-menu button and its top edge sits just below the tab strip.

P4. **Given** the inbox/notifications popover, **when** it opens, **then** it
starts at the sidebar's right border, starts below the tab strip, extends to
the bottom of the window, and overlays the breadcrumb row.

P5. **Given** the sidebar is closed, **when** the inbox opens, **then** it
starts at the left edge of the content area.

## Design

- Use bottom-left/right `Area`/`Window` anchors tied to the sidebar/button
  constants instead of arbitrary screen offsets.
- App menu uses `fixed_pos(screen.left() + 6, screen.top() + 38)`.
- Inbox uses `fixed_pos(sidebar_width, TITLE_BAR_HEIGHT)` and
  `fixed_size(420, screen.height - TITLE_BAR_HEIGHT)`; its content scrolls.
