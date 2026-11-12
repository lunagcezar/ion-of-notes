//! Deployment and tooling drift tests for specs 048, 050-053 and 056.
//!
//! These files are configuration, not Rust logic. The tests read the shipped
//! artifacts and assert the security-critical contracts so a refactor cannot
//! silently drop the non-root user, the read-only root, the healthcheck, the
//! TLS proxy or the pinned toolchain.

use std::fs;
use std::path::PathBuf;

fn read(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(path)).unwrap_or_else(|error| {
        panic!("cannot read {path}: {error}");
    })
}

#[test]
fn server_image_is_multi_stage_non_root_and_health_checked() {
    let dockerfile = read("Dockerfile");
    assert!(dockerfile.contains("FROM rust:"));
    assert!(dockerfile.contains("FROM debian:"));
    assert!(dockerfile.contains("--no-default-features --bin ion-of-notes-server"));
    assert!(dockerfile.contains("USER ion"));
    assert!(dockerfile.contains("HEALTHCHECK"));
    assert!(dockerfile.contains("/health"));
    assert!(dockerfile.contains("--database"));
}

#[test]
fn development_compose_keeps_the_hardening_and_data_volume() {
    let compose = read("compose.yml");
    assert!(compose.contains("read_only: true"));
    assert!(compose.contains("cap_drop:"));
    assert!(compose.contains("no-new-privileges:true"));
    assert!(compose.contains("ion-data:/data"));
    assert!(compose.contains("ION_OF_NOTES_OWNER_PASSWORD_FILE"));
    assert!(compose.contains("healthcheck") || compose.contains("/health"));
}

#[test]
fn production_compose_puts_caddy_in_front_and_keeps_the_app_private() {
    let compose = read("compose.prod.yml");
    assert!(compose.contains("caddy:"));
    assert!(compose.contains("443:443"));
    assert!(compose.contains("expose:"));
    assert!(compose.contains("8787"));
    assert!(compose.contains("depends_on:"));
    assert!(compose.contains("condition: service_healthy"));
    assert!(!compose.contains("ports:\n      - \"8787:8787\""));
}

#[test]
fn caddyfiles_proxy_the_app_and_use_tls() {
    let production = read("deploy/Caddyfile");
    assert!(production.contains("reverse_proxy ion-of-notes:8787"));
    assert!(production.contains("Strict-Transport-Security"));
    let local = read("deploy/Caddyfile.local");
    assert!(local.contains("tls internal"));
    assert!(local.contains("reverse_proxy ion-of-notes:8787"));
}

#[test]
fn build_profiles_and_desktop_feature_seam_stay_lean() {
    let cargo = read("Cargo.toml");
    assert!(cargo.contains("default = [\"desktop\"]"));
    assert!(cargo.contains("[profile.dev]"));
    assert!(cargo.contains("[profile.test]"));
    assert!(cargo.contains("debug = 0"));
    for test in ["ui", "fidelity", "ui_crdt", "screenshot"] {
        assert!(
            cargo.contains(&format!("name = \"{test}\"")),
            "missing desktop test target {test}"
        );
    }
}

#[test]
fn toolchain_and_update_automation_are_pinned() {
    let toolchain = read("rust-toolchain.toml");
    assert!(toolchain.contains("1.99.1"));
    assert!(toolchain.contains("rustfmt"));
    assert!(toolchain.contains("clippy"));
    assert!(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("renovate.json")
            .exists()
    );
}
