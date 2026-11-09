# spec-023 — Add members / sharing dialog

## Purpose and scope

The Share popover was mostly static: the invite field reset every frame, the
Invite button only closed the dialog, and members/general access were
hard-coded. This spec makes the sharing surface functional against the
existing collaboration API.

Scope:

- load the member directory and general access when Share opens;
- support comma-separated emails, a Guest/Member role choice, and an optional
  message;
- send invites through `Client::invite_member`, refresh the list, and show
  success/error feedback;
- cycle general access through Full / Can edit / Can view / No access and
  persist it with `Client::set_share`;
- copy the page link from the dialog;
- preview the populated Share state with `ION_OF_NOTES_SHOW=share`.

Out of scope:

- sending actual email or invitation lifecycle tracking;
- per-member access editing beyond the role label;
- transporting the optional note: the current `MemberDraft` has no message
  field, so the message is captured in UI state but not sent yet;
- teamspace/workspace-level permission inheritance beyond the existing
  general-access rule.

## Evidence

- `screenshots/picture-10.png`: Add members dialog with multiple email chips,
  Guest role, optional message, Send invite / Cancel.
- `screenshots/picture-13.png`: Share popover with owner, general access and
  advanced/copy-link rows.
- `src/ui/dialogs/sharing.rs`: current static Share popover.
- `src/client.rs`: `list_members`, `invite_member`, `list_shares`, `set_share`.
- `src/collab.rs`: `Member`, `MemberDraft`, `Role`, `Share`, `Access`.

## Acceptance criteria

1. **Given** the Share popover opens, **when** the member directory loads,
   **then** the owner and current members are listed with their roles.
2. **Given** one or more comma-separated valid emails, a role and an optional
   message, **when** Send invite is clicked, **then** one `invite_member` call
   is made per email, the input is cleared, and the member list refreshes.
3. **Given** invalid or empty input, **when** Send invite is clicked, **then**
   no request is sent and an inline error is shown.
4. **Given** an invite request fails, **when** the response returns, **then**
   the error is shown and the input is not silently discarded.
5. **Given** the general-access row, **when** it is clicked, **then** the level
   cycles Full → Can edit → Can view → No access → Full and is persisted with
   `set_share`.
6. **Given** Copy link, **when** it is clicked, **then** the current page link
   is copied and the dialog closes.
7. **Given** `ION_OF_NOTES_SHOW=share`, **when** the screenshot runs, **then**
   the populated Share state is rendered.

## Test scenarios (written before implementation)

- `tests/ui.rs`: invite parser, email validation and name derivation.
- `tests/collab.rs`: general-access resolution (page rule wins over workspace
  rule; empty means no access).
- Existing `tests/collab.rs` client round-trip covers `invite_member` and
  `set_share`.
- The Share screenshot is checked manually.

## Design

- `ui/invite.rs`: pure `parse_invite_emails`, `is_valid_email`,
  `member_name_from_email`.
- `collab::general_access(shares, page_id)` resolves page-general then
  workspace-general then `Access::None`.
- `Command` gains `SetInviteInput`, `SetInviteRole`, `SetInviteMessage`,
  `SendInvites` and `SetGeneralAccess`.
- `NotionApp`/`ViewState` hold the member list, invite fields, role, message,
  status/error and current general access.
- `OpenShare` calls `refresh_share`; `SendInvites` and `SetGeneralAccess`
  update through the client and refresh.
- `render_share` renders the input, role chips, message, member rows, general
  access and Advanced/Copy link.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `ui/invite.rs` + `general_access`
- [x] Commands, state and `refresh_share`
- [x] Functional Share popover
- [x] Docs, changelog, build/test/clippy/fmt/screenshot, commit
