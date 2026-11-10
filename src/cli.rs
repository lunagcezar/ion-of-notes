//! Command-line parsing for the desktop app.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Desktop { screenshot: Option<PathBuf> },
    Serve { addr: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub mode: Mode,
    /// Optional SQLite database for `--serve`; JSON remains the fallback.
    pub database: Option<PathBuf>,
    /// Optional remote server URL for the desktop (`--server`).
    pub server: Option<String>,
    /// Bearer token paired with `--server`.
    pub token: Option<String>,
}

impl Args {
    /// Parse an argv-like iterator (the first item is the program name).
    pub fn parse<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut mode = Mode::Desktop { screenshot: None };
        let mut database = None;
        let mut server = None;
        let mut token = None;
        let mut args = iter.into_iter();
        args.next();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--screenshot" => {
                    if let Some(path) = args.next() {
                        mode = Mode::Desktop {
                            screenshot: Some(PathBuf::from(path)),
                        };
                    }
                }
                "--serve" => {
                    if let Some(addr) = args.next() {
                        mode = Mode::Serve { addr };
                    }
                }
                "--database" => {
                    if let Some(path) = args.next() {
                        database = Some(PathBuf::from(path));
                    }
                }
                "--server" => {
                    if let Some(url) = args.next() {
                        server = Some(url);
                    }
                }
                "--token" => {
                    if let Some(value) = args.next() {
                        token = Some(value);
                    }
                }
                _ => {}
            }
        }
        Self {
            mode,
            database,
            server,
            token,
        }
    }
}
