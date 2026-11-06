# spec-007 — Free and open for all

> **Status: RETIRED (de-emphasized).** The person later said the
> "free & open for all" emphasis is **not** wanted in the application: the app
> will be **open sourced** (a license fact, not a marketing feature), and the
> product is now a **self-hosted client/server with accounts**, which directly
> contradicts this spec's "no accounts" rule. See `docs/adr-003-*` and
> `spec/spec-011-collaborative-server.md`. The `edition` module, the sidebar
> "Free & open for all" card, and the Settings "Plans & access" section were
> removed. This file is kept only as history of the earlier framing.

## Purpose and scope

The person has clarified the nature of the product:

> "We aren't making a for profit application, it will be open and free for all."

The feature and pricing references (`/product/features`, `/pricing`) describe a
workspace app whose capabilities are split between free and paid plans. Because
this reconstruction is **non-profit, open, and free for everyone**, it has a
single, free edition: **every feature is included for every user at no cost**,
including capabilities that commercial products reserve for paid plans (AI,
version history, guests, custom domains, site analytics, and so on). There are
no paid tiers, upgrades, trials, billing, accounts, or paywalls.

This spec covers:

- A pure `edition` module that states the license and the free/open policy and
  lists the full feature set (all free).
- A visible "Free & open for all" card in the sidebar.
- A "Plans & access" area inside Settings that lists every feature with its
  (always `Free`) tier.
- An open-source `LICENSE` file and matching documentation.
- Switching the default theme to dark, matching the 29 reference screenshots in
  `reference/`, which all show a dark workspace.

Out of scope: real accounts, billing, network services, and any pricing UI.

## Acceptance criteria (Given / When / Then)

1. **Given** any feature name, **when** its access tier is resolved, **then**
   the result is `Free` (there is no other tier).
2. **Given** the edition policy flags, **when** read, **then** `HAS_PAID_TIER`,
   `HAS_UPGRADES`, `HAS_BILLING`, and `HAS_ACCOUNTS` are all false.
3. **Given** the full feature list, **when** inspected, **then** it is non-empty,
   every name is unique, and it contains capabilities normally sold as add-ons.
4. **Given** the app window, **when** rendered, **then** a "Free & open for all"
   card is visible in the sidebar and the theme matches the dark reference
   screenshots.
5. **Given** the Settings dialog, **when** opened, **then** a "Plans & access"
   section shows the plan, the license, and every feature tagged `Free`.

## Test scenarios (written before implementation)

- `every_feature_is_free_for_everyone` — `tier_for` returns `Free` for each
  listed feature and for an arbitrary name; the list has a meaningful size.
- `there_is_no_paid_tier_or_monetization` — all four policy flags are false and
  the plan label is `Free for everyone`.
- `the_app_is_open_source` — the license is `MIT` and the edition label is
  `Free & open for all`.
- `feature_names_are_unique_and_groups_are_nonempty` — no duplicate names,
  every group has a name and at least one feature.

## Plan

1. Write the four tests first and observe them fail (red).
2. Add the `edition` module with the constants and feature groups (green).
3. Re-export the module from `lib.rs`; add `license = "MIT"` to `Cargo.toml`.
4. Wire the sidebar card, the Settings "Plans & access" section, and switch the
   default theme to dark.
5. Add `LICENSE`, update `README.md`, `AGENTS.md`, `docs/requirements.md`,
   `docs/unknowns.md`, and `CHANGELOG.md`.
6. `cargo build`, `cargo test`, and `--screenshot`; compare the fresh PNG with
   the reference pictures.

## Notes

- Feature names are short, neutral, and written in our own words; no text is
  copied from the original product.
- The dark default resolves open question U-01 (visual design) now that 29
  reference pictures are available.
