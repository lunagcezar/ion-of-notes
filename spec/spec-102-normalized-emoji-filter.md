# spec-102 - Normalize the emoji query once

- Status: implemented (2026-11-12)
- Related: `spec-057`, `spec-060`
- Review source: ChatGPT performance analysis (emoji filtering allocations)

## Purpose and scope

`catalog_matches` lowercased the query and then lowercased the name and keywords
for every catalog entry. The curated catalog is already lowercase, so the loop
allocated two `String`s per entry.

This spec adds `catalog_matches_normalized`, which assumes a trimmed lowercase
query and scans the stored metadata directly. `cached_indices` normalizes the
query once per scan and reuses the existing cached-index and `show_rows`
virtualization. The random-pick action now reuses the same cached indices
instead of running a second filtering path.

`catalog_matches` stays as the case-insensitive convenience wrapper used by the
public tests and any future caller.

Out of scope:

- fuzzy matching;
- a background filtering worker;
- changing the catalogue contents.

## Acceptance criteria

P1. **Given** a query with mixed case, **when** the picker filters, **then** the
same entries match as before.

P2. **Given** a catalogue scan, **when** it runs, **then** the query is
lowercased once and entry metadata is not lowercased.

P3. **Given** the random-pick action, **when** it runs, **then** it uses the
same filtered indices as the grid.

P4. **Given** the catalogue, **when** an entry is added, **then** a test fails
if its name or keywords are not lowercase.

## Test scenarios

- `tests/ui.rs` (`catalog_metadata_is_lowercase_for_normalized_search`,
  existing catalog matching)
- `tests/screenshot.rs`
- Full quality gate

## Design

- `catalog_matches_normalized(name, keywords, query)` is the hot path.
- `catalog_matches` delegates after normalizing once.
- The cached-index key still uses the raw query; a case-only difference is a
  separate cache entry, not a correctness issue.

## Plan / tasks

- [x] Add the normalized matcher
- [x] Normalize once in `cached_indices`
- [x] Reuse cached indices for random selection
- [x] Add the lowercase-metadata test
