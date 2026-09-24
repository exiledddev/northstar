//! Northstar — a screenwriting studio for Linux.
//!
//! Scripts are plain markdown on disk; the editor formats them to standard
//! master-scene screenplay geometry and exports print-ready PDF.


#[cfg(test)]
mod tests;

mod app;
mod caret;
mod editor;
mod export;
mod model;
mod storage;
mod theme;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Northstar")
            .with_app_id("northstar")
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([880.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Northstar",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
