# Disclaimer

## Independent project

`ion-of-notes` is an independent workspace project. It is **not affiliated
with, endorsed by, sponsored by, or approved by Notion Labs, Inc.** “Notion” is
a trademark of Notion Labs, Inc. Any comparison to Notion or other workspace
products is descriptive only and does not imply compatibility, endorsement or
partnership.

## AI-assisted development

This repository was built with **substantial AI assistance** from a written
request and public documentation. AI-generated code and documentation can
contain mistakes, omissions, insecure patterns or outdated assumptions. The
test suite, `clippy` and documentation checks are necessary but not sufficient
for production use.

Review, audit and test the code before deploying it — especially authentication,
session handling, persistence, networking, webhook delivery and anything exposed
to untrusted input.

## Clean-room process

The project follows a strict public-behavior-only clean-room process. The 98
`reference/` pictures (the original 29 plus the later build) are the primary
visual source of truth; the 72 `screenshots/` pictures are local later-build
evidence. No original program, file, asset, traffic capture, source map or
decompiled code is read, run or copied.

This is an independent reimplementation; it does **not** claim wire
compatibility with any third-party service. See
[`docs/clean-room.md`](docs/clean-room.md) and
[`docs/reverse-engineering-log.md`](docs/reverse-engineering-log.md).

## No warranty

The software is provided under the MIT license **without warranty**. You are
responsible for your deployment, data, backups and security posture.

## No production security guarantee

This is a self-hosted project under active development. Do not expose the server
directly to the public internet without TLS termination, a reverse proxy,
backups and a security review. The development owner password must not be used
in production; set `ION_OF_NOTES_OWNER_PASSWORD` before the first SQLite run.
