//! The start-up card.
//!
//! Not a full-screen wash: the window itself opens small, shows the card, and
//! grows into the application when it is done. That is what makes a splash
//! feel like a splash rather than a loading screen.

use web_time::Instant;

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

                // the card is the window: void, with the site's column guides
                painter.rect_filled(rect, round, theme::wash(p.backdrop, (255.0 * alpha) as u8));
                if alpha > 0.99 {
                    crate::chrome::column_guides(painter, rect);
                }
                painter.rect_stroke(
                    rect.shrink(0.5),
                    round,
                    egui::Stroke::new(1.0_f32, theme::wash(p.line_strong, (p.line_strong.a() as f32 * alpha) as u8)),
                );

                let ink = |c: egui::Color32, k: f32| theme::wash(c, (c.a() as f32 * alpha * k) as u8);
                let arrive = anim::ease_out_expo((t / 0.55).clamp(0.0, 1.0));
                let drift = (1.0 - arrive) * 10.0;
                let left = rect.left() + 40.0;
                let width = rect.width() - 80.0;

                // the eyebrow: the mark, then a HUD line
                let mark = Rect::from_min_size(Pos2::new(left, rect.top() + 36.0), Vec2::splat(22.0));
                logo::paint(painter, mark, 0.0);
                theme::tracked_text(
                    painter,
                    Pos2::new(left + 32.0, mark.center().y),
                    concat!("V", env!("CARGO_PKG_VERSION"), " / SCREENWRITING IN INDUSTRY FORMAT"),
                    theme::font_hud(theme::T_MICRO + 0.5),
                    ink(p.text_dim, 1.0),
                    0.8,
                );

                // the name, set giant across the card, the way the site sets
                // its page titles
                let probe = painter.layout_no_wrap("BLACKLINE".into(), theme::font_display(100.0), p.text);
                let size = 100.0 * width / probe.rect.width().max(1.0);
                let name = painter.layout_no_wrap("BLACKLINE".into(), theme::font_display(size), ink(p.text, 1.0));
                painter.galley(Pos2::new(left + drift, rect.top() + 84.0), name, ink(p.text, 1.0));

                // a hairline, then the foot: who makes it, and how far along
                let foot_y = rect.bottom() - 40.0;
                painter.line_segment(
                    [Pos2::new(left, foot_y - 26.0), Pos2::new(left + width, foot_y - 26.0)],
                    egui::Stroke::new(1.0_f32, ink(p.line, 1.0)),
                );
                theme::tracked_text(
                    painter,
                    Pos2::new(left, foot_y),
                    "BY NATIVECOMPUTE",
                    theme::font_hud(theme::T_MICRO + 0.5),
                    ink(p.text_faint, 1.0),
                    0.8,
                );
                // a 2px track filling in the light signal blue, as the site's
                // range input is drawn
                let track = Rect::from_min_size(Pos2::new(left + width - 140.0, foot_y - 1.0), Vec2::new(140.0, 2.0));
                painter.rect_filled(track, egui::Rounding::ZERO, ink(p.line_strong, 1.0));
                let done = anim::ease_out_expo((t / HOLD).clamp(0.0, 1.0));
                painter.rect_filled(
                    Rect::from_min_size(track.min, Vec2::new(track.width() * done, 2.0)),
                    egui::Rounding::ZERO,
                    ink(p.primary_light, 1.0),
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
