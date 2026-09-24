//! The widget kit.
//!
//! Every control in the app is built from these, so the neon-and-glass
//! language reaches the ribbon, the script page, the library and the index
//! cards without each of them re-deciding what a hover looks like. It is the
//! same kit Tesseract is built from, so a control means the same thing in
//! both apps.
//!
//! Two rules run through the lot: nothing draws a box until you reach for it,
//! and everything that lights up lights up *from the glyph outward*.

// The shared kit is kept whole and in step with Tesseract's copy: a
// primitive with no caller in Northstar today is still part of the language.
#![allow(dead_code)]

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, Response, Sense, Stroke, Vec2};

use crate::anim;
use crate::icons::{self, Icon};
use crate::theme::{self, pal};

// ------------------------------------------------------------------ pieces --

/// The faint slot that appears under a ribbon control when you reach for it.
pub fn ghost_slot(ui: &egui::Ui, rect: Rect, hot: bool, on: bool) {
    let p = pal();
    let t = anim::ease(ui.ctx(), rect.min.x as i32 * 7919 + rect.min.y as i32, hot, 0.14);
    theme::hover_surface(ui.painter(), rect, theme::R_SM, p.primary, t, on);
}

/// A vertical hairline, the only divider the ribbon uses.
pub fn rule(ui: &mut egui::Ui, height: f32) {
    let p = pal();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, height), Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        Stroke::new(1.0_f32, theme::wash(p.text_faint, 60)),
    );
}

/// A small square button with an icon that *itself* glows on hover.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tip: &str, size: f32) -> bool {
    icon_button_full(ui, icon, tip, size, None, None)
}

pub fn icon_button_tinted(
    ui: &mut egui::Ui,
    icon: Icon,
    tip: &str,
    size: f32,
    tint: Option<Color32>,
) -> bool {
    icon_button_full(ui, icon, tip, size, tint, None)
}

/// The same again, with an id you choose, for anything that has to be found
/// later — by a hit test, or by the headless tests.
pub fn icon_button_full(
    ui: &mut egui::Ui,
    icon: Icon,
    tip: &str,
    size: f32,
    tint: Option<Color32>,
    id: Option<egui::Id>,
) -> bool {
    let p = pal();
    let (rect, resp) = match id {
        Some(id) => {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
            let resp = ui.interact(rect, id, Sense::click());
            (rect, resp)
        }
        None => ui.allocate_exact_size(Vec2::splat(size), Sense::click()),
    };
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), 0.14);
    let accent = tint.unwrap_or(p.sec);
    let glyph = rect.shrink(size * 0.28);

    // A glyph this small does not want a halo — it wants to change colour.
    theme::hover_surface(ui.painter(), rect, theme::R_SM, accent, hot, false);
    let ink = theme::mix(tint.unwrap_or(p.text_faint), theme::lighten(accent, 0.35), hot);
    icons::draw(ui.painter(), glyph, icon, ink);

    if tip.is_empty() {
        resp.clicked()
    } else {
        resp.on_hover_text(tip).clicked()
    }
}

/// A button carrying a label, and optionally an icon before it. `accent` fills
/// it with the primary gradient; otherwise it is a quiet outline.
pub fn button(ui: &mut egui::Ui, label: &str, icon: Option<Icon>, accent: bool) -> bool {
    button_sized(ui, label, icon, accent, None)
}

pub fn button_sized(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<Icon>,
    accent: bool,
    width: Option<f32>,
) -> bool {
    let p = pal();
    let text_color = if accent { p.ink } else { p.text_dim };
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), theme::font_semi(theme::T_SM), text_color);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let w = width.unwrap_or(galley.rect.width() + icon_w + 24.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), 0.14);

    if accent {
        if hot > 0.01 {
            theme::glow_rect(ui.painter(), rect, theme::R_CTRL, p.sec, 16.0, hot * 0.35);
        }
        theme::grad_rect(
            ui.painter(),
            rect,
            theme::R_CTRL,
            theme::mix(p.prim_grad.1, p.sec_grad.1, hot),
            theme::mix(p.prim_grad.0, p.sec_grad.0, hot),
            Vec2::new(0.3, 1.0),
        );
    } else {
        theme::hover_surface(ui.painter(), rect, theme::R_CTRL, p.primary, hot, false);
    }

    let ink = if accent {
        p.ink
    } else {
        theme::mix(p.text_dim, p.text, hot)
    };
    let mut x = rect.center().x - (galley.rect.width() + icon_w) * 0.5;
    if let Some(i) = icon {
        icons::draw(
            ui.painter(),
            Rect::from_center_size(Pos2::new(x + 7.0, rect.center().y), Vec2::splat(13.0)),
            i,
            ink,
        );
        x += icon_w;
    }
    ui.painter().text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::font_semi(theme::T_SM),
        ink,
    );
    resp.clicked()
}

/// A pill of static text — a tag, a count, a folder name.
pub fn chip(ui: &mut egui::Ui, text: &str, tint: Option<Color32>) -> Response {
    let p = pal();
    let col = tint.unwrap_or(p.text_faint);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), theme::font(theme::T_MICRO + 0.5), col);
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(galley.rect.width() + 15.0, 19.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(theme::R_PILL), theme::wash(col, 30));
    ui.painter()
        .galley(rect.center() - galley.rect.size() * 0.5, galley, col);
    resp
}

/// A clickable pill, for the tag editor and the like.
pub fn chip_button(ui: &mut egui::Ui, text: &str) -> bool {
    let p = pal();
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        theme::font(theme::T_MICRO + 0.5),
        p.text_faint,
    );
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(galley.rect.width() + 17.0, 19.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), 0.13);
    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(theme::R_PILL),
        theme::wash(p.primary, (18.0 + 34.0 * hot) as u8),
    );
    ui.painter().galley(
        rect.center() - galley.rect.size() * 0.5,
        galley,
        theme::mix(p.text_faint, p.primary_light, hot),
    );
    resp.clicked()
}

/// A switch. Returns true when it was just flipped.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> bool {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(34.0, 19.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
    }
    let t = anim::ease(ui.ctx(), resp.id.with("on"), *on, 0.2);
    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(theme::R_PILL),
        theme::mix(theme::wash(p.text_faint, 55), p.primary_deep, t),
    );
    let knob = Pos2::new(rect.left() + 9.5 + t * (rect.width() - 19.0), rect.center().y);
    ui.painter()
        .circle_filled(knob, 6.5, theme::mix(p.text_faint, p.sec_light, t));
    resp.clicked()
}

/// A labelled row with a switch on the right. Returns true when flipped.
pub fn toggle_row(ui: &mut egui::Ui, label: &str, hint: &str, on: &mut bool) -> bool {
    let p = pal();
    let mut flipped = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 46.0).max(40.0));
            ui.label(
                egui::RichText::new(label)
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            if !hint.is_empty() {
                ui.label(
                    egui::RichText::new(hint)
                        .font(theme::font(theme::T_CAP))
                        .color(p.text_faint),
                );
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            flipped = toggle(ui, on);
        });
    });
    ui.add_space(3.0);
    flipped
}

/// A slider whose handle reaches both ends of its track, because one that
/// stops short of the end while reading "max" is simply lying.
pub fn slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let p = pal();
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 22.0), Sense::click_and_drag());
    let (lo, hi) = (*range.start(), *range.end());
    let span = (hi - lo).max(0.0001);

    const KNOB: f32 = 7.0;
    // the travel is inset by the knob radius at both ends, so the *knob* — not
    // the track — is what reaches the extremes
    let x0 = rect.left() + KNOB;
    let x1 = rect.right() - KNOB;

    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(q) = ui.ctx().pointer_interact_pos() {
            let t = ((q.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
            let next = lo + t * span;
            if (next - *value).abs() > f32::EPSILON {
                *value = next;
                changed = true;
            }
        }
    }
    let t = ((*value - lo) / span).clamp(0.0, 1.0);
    let knob = Pos2::new(x0 + t * (x1 - x0), rect.center().y);

    let track = Rect::from_min_max(
        Pos2::new(rect.left(), rect.center().y - 2.5),
        Pos2::new(rect.right(), rect.center().y + 2.5),
    );
    ui.painter()
        .rect_filled(track, egui::Rounding::same(3.0), theme::wash(p.text_faint, 55));
    let filled = Rect::from_min_max(track.min, Pos2::new(knob.x, track.max.y));
    theme::grad_rect(
        ui.painter(),
        filled,
        3.0,
        p.prim_grad.0,
        p.prim_grad.1,
        Vec2::new(1.0, 0.0),
    );

    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() || resp.dragged(), 0.14);
    ui.painter()
        .circle_filled(knob, KNOB + hot, theme::lighten(p.sec_light, 0.1 + 0.2 * hot));
    changed
}

/// A quiet heading inside a panel.
pub fn section(ui: &mut egui::Ui, label: &str) {
    let p = pal();
    ui.add_space(3.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 13.0), Sense::hover());
    theme::tracked_text(
        ui.painter(),
        Pos2::new(rect.left(), rect.center().y),
        &label.to_uppercase(),
        theme::font_semi(theme::T_MICRO - 0.5),
        p.text_faint,
        1.1,
    );
    ui.add_space(4.0);
}

pub fn separator(ui: &mut egui::Ui) {
    let p = pal();
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 13.0), Sense::hover());
    let y = rect.center().y;
    let col = theme::wash(p.text_faint, 60);
    let mid = rect.center().x;
    // fades in and out of nothing, so it divides without drawing a line across
    // the panel
    for (a, b, ca, cb) in [
        (rect.left(), mid, theme::wash(col, 0), col),
        (mid, rect.right(), col, theme::wash(col, 0)),
    ] {
        theme::fill_grad_poly(
            ui.painter(),
            &[
                Pos2::new(a, y - 0.5),
                Pos2::new(b, y - 0.5),
                Pos2::new(b, y + 0.5),
                Pos2::new(a, y + 0.5),
            ],
            ca,
            cb,
            Vec2::new(1.0, 0.0),
        );
    }
}

// ------------------------------------------------------------------ ribbon --

/// A ribbon control: nothing until you reach for it, because the ribbon is
/// glass and should not be a row of boxes.
pub fn ribbon_button(ui: &mut egui::Ui, icon: Icon, label: &str, tip: &str, on: bool) -> bool {
    let p = pal();
    let galley = if label.is_empty() {
        None
    } else {
        Some(ui.painter().layout_no_wrap(
            label.to_string(),
            theme::font_med(theme::T_LABEL),
            p.text_dim,
        ))
    };
    let w = 30.0 + galley.as_ref().map(|g| g.rect.width() + 6.0).unwrap_or(0.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), 0.14);
    theme::hover_surface(ui.painter(), rect, theme::R_SM, p.primary, hot, on);
    let ink = if on {
        p.primary_light
    } else {
        theme::mix(p.text_dim, p.text, hot)
    };
    let glyph = Rect::from_center_size(Pos2::new(rect.left() + 15.0, rect.center().y), Vec2::splat(14.0));
    icons::draw(ui.painter(), glyph, icon, ink);
    if let Some(g) = galley {
        ui.painter().galley(
            Pos2::new(rect.left() + 28.0, rect.center().y - g.rect.height() * 0.5),
            g,
            ink,
        );
    }
    if tip.is_empty() {
        resp.clicked()
    } else {
        resp.on_hover_text(tip).clicked()
    }
}

/// The Write / Cards / Pages switch.
///
/// A sled that glides between two slots, told apart by *contrast* rather than
/// by being lit: the trough is a breath of the accent, the sled is a solid
/// surface with a bright outline, and the label on it is full-strength text.
pub fn segmented(ui: &mut egui::Ui, options: &[(&str, Icon)], current: usize) -> Option<usize> {
    let p = pal();
    let seg_w = 94.0;
    let h = 28.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(seg_w * options.len() as f32 + 6.0, h + 6.0),
        Sense::hover(),
    );

    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(theme::R_CTRL + 2.0),
        theme::wash(p.primary, if p.dark { 20 } else { 26 }),
    );

    let slid = anim::glide(ui.ctx(), "ns-segmented", current as f32, 0.30);
    let sled = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0 + slid * seg_w, rect.top() + 3.0),
        Vec2::new(seg_w, h),
    );
    ui.painter().rect_filled(
        sled,
        egui::Rounding::same(theme::R_SM),
        theme::mix(p.solid, p.primary_quiet_hi, 0.85),
    );
    ui.painter().rect_stroke(
        sled.shrink(0.5),
        egui::Rounding::same(theme::R_SM),
        Stroke::new(1.0_f32, theme::wash(p.primary, 190)),
    );

    let mut picked = None;
    for (i, (label, icon)) in options.iter().enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0 + i as f32 * seg_w, rect.top() + 3.0),
            Vec2::new(seg_w, h),
        );
        let resp = ui.interact(cell, ui.id().with(("seg", i)), Sense::click());
        let on = i == current;
        let ink = if on {
            p.text
        } else if resp.hovered() {
            theme::mix(p.text_dim, p.text, 0.7)
        } else {
            p.text_dim
        };
        icons::draw(
            ui.painter(),
            Rect::from_center_size(Pos2::new(cell.left() + 17.0, cell.center().y), Vec2::splat(13.0)),
            *icon,
            if on { p.primary_light } else { ink },
        );
        ui.painter().text(
            Pos2::new(cell.left() + 31.0, cell.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            if on {
                theme::font_semi(theme::T_LABEL)
            } else {
                theme::font_med(theme::T_LABEL)
            },
            ink,
        );
        if resp.clicked() {
            picked = Some(i);
        }
    }
    picked
}

// ------------------------------------------------------------------ popups --

/// The frame every popover uses. A desktop menu, not a dialogue: tight
/// margins, a real shadow, no ceremony.
pub fn popover_frame() -> egui::Frame {
    let p = pal();
    egui::Frame::none()
        .fill(if p.dark {
            theme::wash(p.backdrop, 236)
        } else {
            theme::wash(Color32::WHITE, 244)
        })
        .stroke(Stroke::new(
            1.0_f32,
            theme::wash(Color32::WHITE, if p.dark { 30 } else { 150 }),
        ))
        .rounding(egui::Rounding::same(theme::R_MD))
        .inner_margin(egui::Margin::symmetric(5.0, 5.0))
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 12.0),
            blur: 34.0,
            spread: 0.0,
            color: Color32::from_black_alpha(if p.dark { 190 } else { 60 }),
        })
}

/// The width a context menu should be: as wide as its longest row, and no
/// wider. A desktop menu is never a panel.
pub fn menu_width(ui: &egui::Ui, labels: &[&str]) -> f32 {
    let widest = labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap((*l).to_string(), theme::font(theme::T_SM), Color32::WHITE)
                .rect
                .width()
        })
        .fold(0.0_f32, f32::max);
    // the text, plus its icon, plus a comfortable margin — no more
    (widest + 44.0).clamp(112.0, 236.0)
}

/// A row in a popover menu.
pub fn menu_item(
    ui: &mut egui::Ui,
    rects: &mut Vec<Rect>,
    label: &str,
    icon: Icon,
    critical: bool,
) -> bool {
    menu_item_enabled(ui, rects, label, icon, critical, true)
}

pub fn menu_item_enabled(
    ui: &mut egui::Ui,
    rects: &mut Vec<Rect>,
    label: &str,
    icon: Icon,
    critical: bool,
    enabled: bool,
) -> bool {
    let p = pal();
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(w, 25.0),
        if enabled { Sense::click() } else { Sense::hover() },
    );
    rects.push(rect);
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() && enabled, 0.1);
    if hot > 0.01 {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(theme::R_SM - 2.0),
            theme::wash(if critical { p.danger } else { p.primary }, (52.0 * hot) as u8),
        );
    }
    let base = if !enabled {
        theme::wash(p.text_faint, 110)
    } else if critical {
        p.danger_light
    } else {
        p.text_dim
    };
    let color = theme::mix(base, if critical { theme::lighten(p.danger_light, 0.2) } else { p.text }, hot);
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 15.0, rect.center().y), Vec2::splat(13.0)),
        icon,
        color,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 29.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        theme::font(theme::T_SM),
        color,
    );
    enabled && resp.clicked()
}

/// A dropdown's closed state — the thing you click to open it.
pub fn dropdown(ui: &mut egui::Ui, icon: Icon, label: &str, open: bool, width: f32) -> Response {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() || open, 0.13);
    theme::hover_surface(ui.painter(), rect, theme::R_SM, p.primary, hot, open);
    let ink = theme::mix(p.text_dim, p.text, hot);
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 15.0, rect.center().y), Vec2::splat(13.0)),
        icon,
        ink,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 29.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        elide(label, ((width - 54.0) / 6.4) as usize),
        theme::font_med(theme::T_LABEL),
        ink,
    );
    let turn = anim::ease(ui.ctx(), resp.id.with("turn"), open, 0.18);
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.right() - 13.0, rect.center().y), Vec2::splat(11.0)),
        if turn > 0.5 { Icon::ChevronUp } else { Icon::ChevronDown },
        p.text_faint,
    );
    resp
}

/// Watches for a click that lands outside a popover, so it can close itself.
pub struct Dismisser {
    pub opened_on: u64,
}

impl Dismisser {
    pub fn should_close(&self, ctx: &egui::Context, frame_no: u64, areas: &[Rect]) -> bool {
        if frame_no <= self.opened_on + 1 {
            return false;
        }
        if !ctx.input(|i| i.pointer.any_click()) {
            return false;
        }
        ctx.pointer_interact_pos()
            .map(|p| !areas.iter().any(|r| r.expand(4.0).contains(p)))
            .unwrap_or(false)
    }
}

pub fn elide(s: &str, max: usize) -> String {
    let max = max.max(4);
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{head}\u{2026}")
    }
}

fn text_width(ui: &egui::Ui, s: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(s.to_string(), theme::font_mono(theme::T_CAP), Color32::WHITE)
        .rect
        .width()
}

/// Trim to whole words rather than cutting a word in half.
pub fn fit_text(ui: &egui::Ui, s: &str, room: f32) -> String {
    if room <= 20.0 {
        return String::new();
    }
    if text_width(ui, s) <= room {
        return s.to_string();
    }
    let mut words: Vec<&str> = s.split(' ').collect();
    while words.len() > 1 {
        words.pop();
        let candidate = format!("{}\u{2026}", words.join(" "));
        if text_width(ui, &candidate) <= room {
            return candidate;
        }
    }
    String::new()
}

/// The first of `options` that fits, or nothing at all.
pub fn pick_that_fits(ui: &egui::Ui, options: &[&str], room: f32) -> String {
    for o in options {
        if text_width(ui, o) <= room {
            return (*o).to_string();
        }
    }
    String::new()
}

/// A colour swatch button, for the text colour picker.
pub fn swatch(
    ui: &mut egui::Ui,
    color: Color32,
    tip: &str,
    selected: bool,
    id: Option<egui::Id>,
) -> bool {
    let (rect, resp) = match id {
        Some(id) => {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(19.0), Sense::hover());
            let resp = ui.interact(rect, id, Sense::click());
            (rect, resp)
        }
        None => ui.allocate_exact_size(Vec2::splat(19.0), Sense::click()),
    };
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), 0.13);
    ui.painter()
        .circle_filled(rect.center(), 6.0 + 1.0 * hot, theme::lighten(color, 0.15 * hot));
    if selected {
        ui.painter().circle_stroke(
            rect.center(),
            8.5,
            Stroke::new(1.4_f32, theme::wash(color, 220)),
        );
    }
    resp.on_hover_text(tip).clicked()
}
