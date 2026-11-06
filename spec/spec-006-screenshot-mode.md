---
title: Screenshot option for automated visual checks
version: 1.0
date_created: 2026-11-05
tags: screenshot, cli
status: implemented
---

# Spec 006 — Screenshot Mode

## 1. Purpose & Scope

The build harness runs the app with `--screenshot <file.png>` after each
build. The app must start normally, draw about 120 frames, save the window's
picture as a PNG to the given path, and exit.

## 2. Acceptance Criteria

- **AC-001** — Given `--screenshot path`, when the app starts, then the
  regular window and UI are shown (no blank placeholder).
- **AC-002** — Given ~120 frames have been drawn, when the screenshot is
  requested, then the rendered pixels of the window are captured.
- **AC-003** — Given the captured `ColorImage`, when `write_png` runs, then a
  valid PNG file exists at the requested path with the same width and height.
- **AC-004** — Given the PNG is saved, when the next frame runs, then the app
  exits cleanly.
- **AC-005** — Given no `--screenshot` flag, when the app starts, then it
  stays open as a normal desktop app.
- **AC-006** — Given a 2x2 `ColorImage` with known colors, when converted to
  RGBA, then byte order and dimensions are correct.

## 3. Test Scenarios

- TS-001 `color_image_to_rgba_preserves_pixels_and_size`
- TS-002 `write_png_produces_readable_png_with_right_dimensions`
- TS-003 `cli_parses_screenshot_path_and_serve_flag`
- TS-004 manual: run `--screenshot app/shots/now.png`, open the PNG, confirm
  the workspace is visible and not blank.

## 4. Plan & Tasks

1. Write the tests — **red**.
2. Implement `screenshot::color_image_to_rgba` and `write_png` using the
   `image` PNG encoder.
3. Implement `Args::parse` and the frame counter / `request_screenshot` /
   close sequence in `main.rs` and `ui.rs`.
4. Run `--screenshot` and inspect the PNG; iterate until non-blank.
5. Update `CHANGELOG.md`.
