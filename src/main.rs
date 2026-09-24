//! Northstar — a screenwriting studio for Linux.
//!
//! Scripts are plain markdown on disk; the editor formats them to standard
//! master-scene screenplay geometry and exports print-ready PDF. Drawn in the
//! Starforge design language it shares with Tesseract.
//!
//! Starforge Software.

#[cfg(test)]
mod tests;
#[cfg(test)]
mod uitests;

mod alerts;
mod anim;
mod app;
mod blur;
mod cards;
mod caret;
mod chrome;
mod editor;
mod export;
mod fountain;
mod icons;
mod logo;
mod model;
mod pages;
mod settings;
mod splash;
mod storage;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    // `northstar --emit-icon <path>` writes the mark as an SVG. install.sh uses
    // it so the launcher icon is generated from the same code the app draws.
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--emit-icon") {
        let path = args
            .get(i + 1)
            .cloned()
            .unwrap_or_else(|| "northstar.svg".to_string());
        theme::set_palette(theme::ThemeId::Bloodmoon, true);
        match std::fs::write(&path, logo::svg()) {
            Ok(()) => println!("wrote {path}"),
            Err(e) => eprintln!("could not write {path}: {e}"),
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("northstar {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // The window opens at the size of the splash card and grows into the app
    // once it has been shown, so the card is a card and not a full screen.
    let small = storage::read_settings().splash;
    let size: [f32; 2] = if small {
        [splash::CARD.x, splash::CARD.y]
    } else {
        [1320.0, 900.0]
    };

    // The window is transparent so a compositor that can blur has something to
    // blur; when none can, the app paints its own opaque ground instead.
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Northstar")
            .with_app_id("northstar")
            .with_transparent(true)
            // Northstar draws its own title bar, buttons and resize grips, so
            // the desktop's frame would only be a second, uglier one.
            .with_decorations(false)
            .with_drag_and_drop(true)
            .with_inner_size(size)
            .with_min_inner_size(if small { size } else { [880.0, 580.0] }),
        ..Default::default()
    };

    // anything else on the command line is a script to open or import
    let files: Vec<std::path::PathBuf> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_file())
        .collect();

    eframe::run_native(
        "Northstar",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc);
            app.arrivals = files;
            Ok(Box::new(app))
        }),
    )
}
