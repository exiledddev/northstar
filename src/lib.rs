//! Northstar — a screenwriting studio.
//!
//! Scripts are plain markdown; the editor formats them to standard
//! master-scene screenplay geometry and exports print-ready PDF. Drawn in the
//! design language it shares with Tesseract.
//!
//! The whole app lives in this library. The desktop binary (`main.rs`) runs it
//! on a window over the user's own library folder; the web build runs the very
//! same code in a browser tab over the team library, through a different
//! [`backend::Store`]. One app, one design, two homes.
//!
//! MarkedExiled Software.

#[cfg(test)]
mod tests;
#[cfg(test)]
mod uitests;

pub mod alerts;
pub mod anim;
pub mod app;
pub mod backend;
pub mod blur;
pub mod cards;
pub mod caret;
pub mod chrome;
pub mod editor;
pub mod export;
pub mod fountain;
pub mod icons;
pub mod logo;
pub mod model;
pub mod pages;
pub mod settings;
pub mod splash;
pub mod storage;
pub mod theme;
pub mod ui;

/// The toolkit, re-exported so a crate building on this one draws with exactly
/// the same egui — the design system is only the same if the toolkit is.
pub use eframe;

/// Is this the browser build?
pub const WEB: bool = cfg!(target_arch = "wasm32");
