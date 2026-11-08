//! Tests for command-line parsing (public API only).

use not_notion::cli::{Args, Mode};
use std::path::PathBuf;

#[test]
fn cli_parses_screenshot_path() {
    let args = Args::parse(vec![
        "not-notion".to_string(),
        "--screenshot".to_string(),
        "shots/now.png".to_string(),
    ]);
    assert_eq!(
        args.mode,
        Mode::Desktop {
            screenshot: Some(PathBuf::from("shots/now.png"))
        }
    );
}

#[test]
fn cli_parses_serve_flag() {
    let args = Args::parse(vec![
        "not-notion".to_string(),
        "--serve".to_string(),
        "127.0.0.1:8787".to_string(),
    ]);
    assert_eq!(
        args.mode,
        Mode::Serve {
            addr: "127.0.0.1:8787".to_string()
        }
    );
}

#[test]
fn cli_defaults_to_plain_desktop() {
    let args = Args::parse(vec!["not-notion".to_string()]);
    assert_eq!(args.mode, Mode::Desktop { screenshot: None });
}
