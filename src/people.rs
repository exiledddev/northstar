//! People, drawn: avatars and the small chips that say who did what.
//!
//! Only a team library has people in it. Everything here follows the glow
//! rules the rest of the app keeps: a face is a quiet circle with a hairline
//! edge, never lit.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::backend::Person;
use crate::theme::{self, pal};

/// A colour of their own for someone without a picture, the same every time
/// and in every window: it comes from their id, not from where they sit.
pub fn colour_of(person: &Person) -> Color32 {
    // FNV-1a: tiny, stable across builds and platforms
    let mut h: u32 = 0x811c_9dc5;
    for b in person.id.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    let hue = (h % 3600) as f32 / 3600.0;
    let p = pal();
    if p.dark {
        theme::hsl(hue, 0.42, 0.42)
    } else {
        theme::hsl(hue, 0.48, 0.58)
    }
}

/// Paint someone's face into `rect` (a square): their Discord picture, or
/// their initials on their colour.
pub fn avatar(ui: &egui::Ui, rect: Rect, person: &Person) {
    let p = pal();
    let r = rect.width().min(rect.height()) * 0.5;
    let c = rect.center();
    let square = Rect::from_center_size(c, Vec2::splat(r * 2.0));
    match texture(ui.ctx(), person) {
        Some(tex) => {
            egui::Image::new(egui::load::SizedTexture::new(tex.id(), square.size()))
                .rounding(egui::Rounding::same(r))
                .paint_at(ui, square);
        }
        None => {
            ui.painter().circle_filled(c, r, colour_of(person));
            ui.painter().text(
                c,
                egui::Align2::CENTER_CENTER,
                person.initials(),
                theme::font_semi((r * 0.92).max(7.0)),
                Color32::from_white_alpha(235),
            );
        }
    }
    // a hairline edge, so a dark picture still reads as a disc on dark glass
    ui.painter().circle_stroke(
        c,
        r - 0.5,
        Stroke::new(1.0_f32, theme::wash(if p.dark { Color32::WHITE } else { Color32::BLACK }, 28)),
    );
}

/// Lay out a face of `size` in the current row.
pub fn face(ui: &mut egui::Ui, person: &Person, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    avatar(ui, rect, person);
    resp.on_hover_text(&person.name)
}

/// A row of faces, overlapping a little, for "who has worked on this".
pub fn faces(ui: &mut egui::Ui, people: &[Person], size: f32, max: usize) {
    let shown = people.len().min(max);
    let step = size * 0.72;
    let width = if shown == 0 { 0.0 } else { size + step * (shown - 1) as f32 };
    let extra = people.len().saturating_sub(max);
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(width + if extra > 0 { 30.0 } else { 0.0 }, size),
        Sense::hover(),
    );
    let p = pal();
    for (k, person) in people.iter().take(max).enumerate().rev() {
        let at = Rect::from_min_size(Pos2::new(rect.left() + step * k as f32, rect.top()), Vec2::splat(size));
        // each face sits on a ring of the island behind it, so the overlap
        // reads as a stack rather than a smudge
        ui.painter().circle_filled(at.center(), size * 0.5 + 1.5, p.solid);
        avatar(ui, at, person);
    }
    if extra > 0 {
        ui.painter().text(
            Pos2::new(rect.left() + width + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("+{extra}"),
            theme::font_mono(theme::T_MICRO),
            p.text_faint,
        );
    }
    let names: Vec<&str> = people.iter().map(|x| x.name.as_str()).collect();
    if !names.is_empty() {
        let _ = resp.on_hover_text(names.join(", "));
    }
}

/// The decoded picture as a texture, made once and kept in egui's memory.
fn texture(ctx: &egui::Context, person: &Person) -> Option<egui::TextureHandle> {
    let image = person.avatar.as_ref()?;
    let id = egui::Id::new(("ns-avatar", person.id.as_str(), image.size));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let t = ctx.load_texture(
        format!("avatar-{}", person.id),
        (**image).clone(),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

/// "Alex · 2h ago", or "you · 2h ago".
pub fn by_line(person: &Person, me: Option<&Person>, when: &str) -> String {
    let who = if me.map(|m| m.id == person.id).unwrap_or(false) {
        "you".to_string()
    } else {
        person.first_name().to_string()
    };
    if when.is_empty() {
        who
    } else {
        format!("{who} · {when}")
    }
}
