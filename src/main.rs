//! Entry point: desktop window (default or `--screenshot`) or `--serve`.

use ion_of_notes::app;
use ion_of_notes::cli::{Args, Mode};
use ion_of_notes::server::serve_axum;
use ion_of_notes::ui::NotionApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args());
    match args.mode {
        Mode::Serve { addr } => {
            // SQLite is opt-in; JSON remains the default local adapter.
            let server = match args.database {
                Some(database) => app::build_server_sqlite(database)?,
                None => app::build_server(),
            };
            serve_axum(&addr, server)?;
            Ok(())
        }
        Mode::Desktop { screenshot } => {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([1322.0, 861.0])
                    .with_min_inner_size([900.0, 600.0])
                    .with_title("ion-of-notes")
                    // The window frame (tab strip + minimize/maximize/close) is
                    // already drawn in Rust by `ui::render::render_title_bar`.
                    // Asking the OS for decorations too would show two frames.
                    .with_decorations(false),
                ..Default::default()
            };
            let creator: eframe::AppCreator<'_> =
                Box::new(move |cc: &eframe::CreationContext<'_>| {
                    let app: Box<dyn eframe::App> = Box::new(NotionApp::new(cc, screenshot));
                    Ok(app)
                });
            eframe::run_native("ion-of-notes", options, creator)?;
            Ok(())
        }
    }
}
