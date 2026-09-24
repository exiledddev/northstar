//! The start-up card.
//!
//! Not a full-screen wash: the window itself opens small, shows the card, and
//! grows into the application when it is done. That is what makes a splash
//! feel like a splash rather than a loading screen.

use std::time::Instant;

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use crate::anim;
use crate::logo;
use crate::theme::{self, pal};

/// The window's size while the card is up.
pub const CARD: Vec2 = Vec2::new(540.0, 296.0);

const HOLD: f32 = 1.55;
const FADE: f32 = 0.40;

pub struct Splash {
    born: Instant,
    /// Seconds in when it was asked to go early, if it was.
    skip_at: Option<f32>,
}

impl Default for Splash {
    fn default() -> Self {
        Splash {
            born: Instant::now(),
            skip_at: None,
        }
    }
}

impl Splash {
    /// Paint it. Returns false once it is finished and can be dropped.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let p = pal();
        let t = self.born.elapsed().as_secs_f32();
        if !anim::enabled() {
            return false;
        }
        let starts = self.skip_at.unwrap_or(HOLD);
        let leaving = ((t - starts) / FADE).clamp(0.0, 1.0);
        if leaving >= 1.0 {
            return false;
        }
        ctx.request_repaint();
        let alpha = 1.0 - anim::smootherstep(leaving);

        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("ns-splash"))
            .order(egui::Order::Tooltip)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                let painter = ui.painter();
                let round = egui::Rounding::same(theme::R_WINDOW);

                // the card is the window
                painter.rect_filled(rect, round, theme::wash(p.sunken, (255.0 * alpha) as u8));
                // a wash of the theme sweeping up from the bottom-left corner,
                // in the spirit of the ribbons JetBrains puts on theirs
                theme::fill_grad_poly(
                    painter,
                    &theme::rounded_poly(rect, theme::R_WINDOW),
                    theme::wash(p.prim_grad.0, (105.0 * alpha) as u8),
                    theme::wash(p.sec_grad.0, 0),
                    Vec2::new(-0.75, 1.0),
                );
                painter.rect_stroke(
                    rect.shrink(0.5),
                    round,
                    egui::Stroke::new(1.0_f32, theme::wash(p.text, (34.0 * alpha) as u8)),
                );

                let ink = |c: egui::Color32, k: f32| theme::wash(c, (255.0 * alpha * k) as u8);
                let arrive = anim::ease_out_expo((t / 0.55).clamp(0.0, 1.0));
                let drift = (1.0 - arrive) * 10.0;

                // the mark, top left, glowing
                let mark = Rect::from_center_size(
                    Pos2::new(rect.left() + 70.0, rect.top() + 74.0),
                    Vec2::splat(52.0),
                );
                // kept clear of the corner, or the bloom spills past the card
                theme::glow_circle(
                    painter,
                    mark.center(),
                    13.0,
                    p.sec_grad.1,
                    44.0,
                    0.55 * alpha * (0.7 + 0.3 * anim::breathe(ui.ctx(), 2.6)),
                );
                logo::paint(painter, mark, 0.0);

                // the name, set large and tracked, the way a product name is
                theme::tracked_text(
                    painter,
                    Pos2::new(rect.left() + 112.0 + drift, rect.top() + 64.0),
                    "NORTHSTAR",
                    theme::font_bold(30.0),
                    ink(p.text, 1.0),
                    2.2,
                );
                painter.text(
                    Pos2::new(rect.left() + 114.0 + drift, rect.top() + 90.0),
                    egui::Align2::LEFT_CENTER,
                    concat!("v", env!("CARGO_PKG_VERSION")),
                    theme::font_mono(theme::T_CAP),
                    ink(p.sec_light, 1.0),
                );

                painter.text(
                    Pos2::new(rect.left() + 40.0, rect.top() + 138.0),
                    egui::Align2::LEFT_CENTER,
                    "Screenwriting, in industry format",
                    theme::font(theme::T_BODY + 1.0),
                    ink(p.text_dim, 1.0),
                );

                // a hairline, then the foot
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + 40.0, rect.bottom() - 62.0),
                        Pos2::new(rect.right() - 40.0, rect.bottom() - 62.0),
                    ],
                    egui::Stroke::new(1.0_f32, ink(p.text_faint, 0.30)),
                );
                theme::tracked_text(
                    painter,
                    Pos2::new(rect.left() + 40.0, rect.bottom() - 36.0),
                    "MARKEDEXILED SOFTWARE",
                    theme::font_semi(theme::T_MICRO),
                    ink(p.text_faint, 1.0),
                    1.6,
                );
                logo::spinner(
                    painter,
                    Pos2::new(rect.right() - 52.0, rect.bottom() - 38.0),
                    10.0,
                    anim::clock(ui.ctx()) * 0.9,
                    ink(p.sec_light, 0.95),
                );

                if resp.clicked() && self.skip_at.is_none() {
                    self.skip_at = Some(t);
                }
            });

        if self.skip_at.is_none()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter))
        {
            self.skip_at = Some(t);
        }
        true
    }
}

/// Put a window of `size` in the middle of the screen it is on.
pub fn centre(ctx: &egui::Context, size: Vec2) {
    if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
        let at = ((monitor - size) * 0.5).max(Vec2::ZERO);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(at.to_pos2()));
    }
}
