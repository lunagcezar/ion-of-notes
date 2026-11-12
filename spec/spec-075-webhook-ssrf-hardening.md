# spec-075 - Webhook SSRF hardening

- Status: in progress (2026-11-12)
- Related: `spec-041`, `spec-054`
- Review sources: the 2026-11-12 external reviews (Kimi, ChatGPT, Claude)

## Purpose and scope

Webhook destinations are currently validated by scheme only: any `https://`
URL is accepted. That allows a server-side request to a private, loopback or
link-local address (including cloud metadata endpoints) and does not defend
against DNS rebinding.

This spec adds a two-layer outbound policy:

- a pure URL policy that rejects obvious private/loopback/link-local hosts;
- a delivery-time DNS resolution check in the production sender;
- an explicit development override for loopback tests.

Scope:

- `webhooks::is_allowed_url` rejects private HTTPS targets and keeps the
  loopback HTTP exception behind the existing development flag;
- `WebhookSender` gains an optional `allow_private_targets` hook;
- `UreqWebhookSender` resolves the destination and rejects private IPs unless
  the development override is enabled;
- `WorkspaceServer::allow_insecure_webhooks` forwards the override to the
  sender;
- tests for private URL rejection and the sender's DNS guard.

Out of scope:

- an outbound allowlist configured by the operator (documented as a future
  option);
- egress firewall / network-policy automation.

## Acceptance criteria

P1. **Given** a webhook URL with a loopback, private, link-local or unique-local
host, **when** it is created, **then** it is rejected unless the development
override is enabled.

P2. **Given** a normal public HTTPS URL, **when** it is created, **then** it is
accepted.

P3. **Given** the production sender, **when** the destination resolves to a
private address, **then** delivery fails before the HTTP request.

P4. **Given** the development override, **when** a loopback HTTP webhook is
delivered, **then** the existing local test behavior is preserved.

## Test scenarios

- `tests/webhooks.rs::private_https_targets_are_rejected`
- `tests/webhooks.rs::public_https_targets_are_allowed`
- `tests/webhooks.rs::production_sender_rejects_a_private_resolution`
- `tests/webhooks.rs::development_sender_allows_loopback`

## Design

- Parse the URL host without adding a new dependency: strip the scheme,
  isolate the authority, strip userinfo, handle `[IPv6]`, and reject known
  private ranges.
- `is_allowed_url` is pure and used at subscription creation time.
- `UreqWebhookSender` resolves `(host, port)` with `ToSocketAddrs` before
  calling `agent.post`; any private/loopback/link-local/unspecified/multicast
  address rejects the delivery. This is defense in depth, not a complete
  DNS-rebinding solution; the docs note the remaining TOCTOU window.
- `WebhookSender::allow_private_targets` defaults to a no-op so injected test
  senders keep working.
- `WorkspaceServer::allow_insecure_webhooks` calls the hook on the sender.

## Plan / tasks

- [ ] Write spec-075 and failing webhook tests (red)
- [ ] Add the private-host URL policy
- [ ] Add the sender DNS guard and development override
- [ ] Add `docs/security-hardening.md` webhook section
- [ ] Run the full gate and commit
