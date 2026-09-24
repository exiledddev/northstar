//! Thin wrappers over egui's `TextEditState`.
//!
//! These are the only places that reach into egui's text-edit internals, so if
//! a future egui release renames something this is the one file to patch.
//! Everything here degrades to `None` / no-op rather than panicking, and the
//! editor treats "caret unknown" as "caret at the end of the block".

use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
};

/// Selection as (start, end) in *characters*, ordered.
pub fn range(ctx: &egui::Context, id: egui::Id) -> Option<(usize, usize)> {
    let state = egui::TextEdit::load_state(ctx, id)?;
    let r = state.cursor.char_range()?;
    let a = r.primary.index;
    let b = r.secondary.index;
    Some((a.min(b), a.max(b)))
}

/// Caret position when there is no selection.
pub fn caret(ctx: &egui::Context, id: egui::Id) -> Option<usize> {
    let (a, b) = range(ctx, id)?;
    if a == b {
        Some(a)
    } else {
        None
    }
}

pub fn set(ctx: &egui::Context, id: egui::Id, index: usize) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let c = CCursor::new(index);
    state.cursor.set_char_range(Some(CCursorRange::one(c)));
    state.store(ctx, id);
}

/// Split a string at a character index (not a byte index).
pub fn split_at_char(s: &str, index: usize) -> (String, String) {
    let head: String = s.chars().take(index).collect();
    let tail: String = s.chars().skip(index).collect();
    (head, tail)
}

pub fn char_len(s: &str) -> usize {
    s.chars().count()
}
