# spec-153 - Browser UI

- Status: implemented (2026-11-25)
- Related: `spec-038`, `spec-148`, `spec-152`
- Review source: deferred "Browser UI"

## Purpose and scope

The self-hosted server exposed the REST/WebSocket contract but no browser
surface. This spec serves a small, dependency-free single-page app from the
Axum adapter so a browser can sign in and edit pages without installing the
desktop window.

### Served routes

| Route | Content |
| ----- | ------- |
| `GET /app` | embedded HTML shell |
| `GET /app.js` | embedded vanilla JavaScript |
| `GET /app.css` | embedded stylesheet |

Assets are compiled in with `include_str!`; there is no bundler, CDN or external
origin. Every response carries:

- `Content-Type` for the asset;
- `Cache-Control: no-store`;
- `Content-Security-Policy: default-src 'self'; connect-src 'self'; ...;
  script-src 'self'; frame-ancestors 'none'`;
- `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`,
  `X-Frame-Options: DENY`.

### Features

- Sign in with a session token stored in `sessionStorage`.
- Register an invited email (spec-152) or change/reset passwords.
- List pages, create a page, open a page and edit block text.
- Search pages.

The browser uses the same `/api/v2` contract and version header as the desktop
client. It does not use cookies, so no CSRF surface is introduced. Real-time
WebSockets are not wired into the browser UI yet; a reload shows the latest
state.

## Acceptance criteria

P1. **Given** `GET /app`, **then** the HTML shell is returned with the CSP and no
external origin reference.

P2. **Given** `GET /app.js` and `/app.css`, **then** they are served with the
right content types and the same security headers.

P3. **Given** a browser, **then** login/register, page list/create/open, block
save and search work against the v2 API.

P4. **Given** no token, **then** the auth form is shown and only public routes
are called.

## Test scenarios

- `tests/web.rs` (`browser_app_is_served_with_a_restrictive_csp`).
- `tests/auth.rs` for the account flows used by the forms.
- Manual smoke: `--serve`, open `/app`, sign in, edit a block, reload.

## Plan / tasks

- [x] Embed the HTML/JS/CSS assets.
- [x] Add the three routes and security headers.
- [x] Implement login/register, pages, blocks, search and password forms.
- [x] Add the asset/header test.
