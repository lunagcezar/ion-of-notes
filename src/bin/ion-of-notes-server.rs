//! Server-only entry point for containers and headless deployments.
//!
//! Build it with `--no-default-features` to omit eframe/egui/wgpu:
//! `cargo build --release --no-default-features --bin ion-of-notes-server`.

use ion_of_notes::app;
use ion_of_notes::cli::{Args, Mode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args());
    let Mode::Serve { addr } = args.mode else {
        eprintln!("usage: ion-of-notes-server --serve <addr> [--database <path>]");
        std::process::exit(2);
    };
    app::serve(&addr, args.database)?;
    Ok(())
}
