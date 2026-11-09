//! Tests for command-line parsing (public API only).

use ion_of_notes::cli::{Args, Mode};
use std::path::PathBuf;

#[test]
fn cli_parses_screenshot_path() {
    let args = Args::parse(vec![
        "ion-of-notes".to_string(),
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
        "ion-of-notes".to_string(),
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
    let args = Args::parse(vec!["ion-of-notes".to_string()]);
    assert_eq!(args.mode, Mode::Desktop { screenshot: None });
    assert_eq!(args.database, None);
}

#[test]
fn cli_parses_a_database_path() {
    let args = Args::parse(vec![
        "ion-of-notes".to_string(),
        "--serve".to_string(),
        "127.0.0.1:8787".to_string(),
        "--database".to_string(),
        "data/workspace.sqlite3".to_string(),
    ]);
    assert_eq!(
        args.mode,
        Mode::Serve {
            addr: "127.0.0.1:8787".to_string()
        }
    );
    assert_eq!(args.database, Some(PathBuf::from("data/workspace.sqlite3")));
}
