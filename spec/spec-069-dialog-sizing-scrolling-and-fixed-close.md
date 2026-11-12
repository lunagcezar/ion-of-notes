# spec-069 - Dialog sizing, scrolling and fixed close

- Status: implemented (2026-11-12)
- Related: `spec-061`, `spec-067`
- Reference: `reference/picture-15.png` .. `picture-29.png`

## Purpose and scope

The settings window still has layout problems:

1. The content scrollbar is not on the right border.
2. The settings rail has no scrollbar; shrinking the window leaves the rail
   content without its background at the bottom.
3. The dialog does not shrink when the window is shorter than the dialog.
4. The close `×` is painted after the content instead of staying fixed above it.
5. The search/new-tab window has the same fixed-height problem.

## Acceptance criteria

D1. **Given** the settings window, **when** it opens, **then** its size is
`min(1200, screen_width - 80) x min(720, screen_height - 80)`.

D2. **Given** the settings rail, **when** it overflows or the window shrinks,
**then** the rail scrolls and its background still fills the full height.

D3. **Given** the settings content, **when** it overflows, **then** the
scrollbar sits on the right border with a small visible margin.

D4. **Given** the close control, **when** content scrolls, **then** the `×`
stays fixed above the content inside the dialog padding.

D5. **Given** the search/new-tab window, **when** the screen is shorter than
690 points, **then** the window height shrinks and the result/preview panes
still scroll.

## Design

- Compute dialog sizes from `ctx.screen_rect()`.
- Wrap the rail in a `ScrollArea`; paint the rail background from the window
  rect so it cannot end early.
- Draw the close control in a foreground `Area` anchored to the dialog's
  top-right padding.
- Make the switcher height `min(690, screen_height - 120)` and keep its body
  height derived from the actual window height.
