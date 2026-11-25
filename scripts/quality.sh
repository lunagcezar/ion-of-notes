#!/bin/sh
# PowerShell-free quality gate (spec-128).
#
# This wrapper runs the same cargo commands as scripts/quality.ps1. The
# documentation checks are part of `tests/docs.rs`, so no PowerShell is
# required.

set -eu

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --all-targets --no-default-features
cargo test --features desktop --test ui --test fidelity --test ui_crdt --test screenshot
cargo test --all-targets
