//! The window itself.
//!
//! Blackline asks for no decorations, so it draws its own: a slim title bar
//! with the mark, the name and the three window buttons, and invisible grips
//! along the edges for resizing. The window is void black, with the faint
//! column guides of NativeCompute's site drawn across it — the "technical
//! drawing" grid — and the panels sit on it as hairlined slabs.

use eframe::egui::{self, Pos2, Rect, Sense, Stroke, Vec2, ViewportCommand};

use crate::anim;
use crate::logo;
use crate::theme::{self, pal};

/// Height of the title bar strip.
pub const TITLE_H: f32 = 32.0;

/// Is the window maximised right now? A browser tab always is: it fills the
/// page edge to edge, with square corners, and the browser is the frame.
pub fn maximized(ctx: &egui::Context) -> bool {
    crate::WEB || ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// The ground the whole window sits on: the fill, and the site's six faint
/// column guides inside a gutter.
pub fn backdrop(ctx: &egui::Context, fill: egui::Color32) {
    let p = pal();
    let rect = ctx.screen_rect();
    let r = if maximized(ctx) { 0.0 } else { theme::R_WINDOW };
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(rect, egui::Rounding::same(r), fill);
    column_guides(&painter, rect);
    if crate::WEB {
        // a browser tab's edge is the browser's
        return;
    }
    // the single hairline in the app: the edge of the window itself
    painter.rect_stroke(
        rect.shrink(0.5),
        egui::Rounding::same(r),
        Stroke::new(1.0_f32, theme::wash(if p.dark { egui::Color32::WHITE } else { p.line_strong }, if p.dark { 26 } else { 90 })),
    );
}

/// The six column guides: hairlines at a few percent, from the left gutter
/// across, the way the site's `body::before` draws them.
pub fn column_guides(painter: &egui::Painter, rect: Rect) {
    let p = pal();
    let gutter = if rect.width() > 900.0 { 40.0 } else { 16.0 };
    let col = if p.dark {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 9)
    } else {
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 12)
    };
    let span = rect.width() - 2.0 * gutter;
    if span <= 0.0 {
        return;
    }
    for i in 0..6 {
        let x = (rect.left() + gutter + span * i as f32 / 6.0).round() + 0.5;
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0_f32, col),
        );
    }
}

/// The title bar. The window buttons act on the viewport directly; `extra` is
/// drawn at the right-hand end, laid out right to left — where a team library
/// puts the way home and who you are signed in as. In a browser there are no
/// window buttons: the tab has its own.
pub fn title_bar(ui: &mut egui::Ui, ctx: &egui::Context, extra: impl FnOnce(&mut egui::Ui)) {
    let p = pal();
    let full = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), TITLE_H));

    // anywhere not covered by a control drags the window
    if !crate::WEB {
        let drag = ui.interact(full, ui.id().with("ns-titlebar"), Sense::click_and_drag());
        if drag.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized(ctx)));
        }
    }

    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(full)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 9.0;
    row.add_space(2.0);

    // the site's brand lock-up: the mark, then the name in mixed case
    row.spacing_mut().item_spacing.x = 6.0;
    let (mark, _) = row.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
    logo::paint(row.painter(), mark, 0.0);
    row.label(
        egui::RichText::new("Blackline")
            .font(egui::FontId::new(15.0, theme::semibold()))
            .color(p.text),
    );
    row.spacing_mut().item_spacing.x = 9.0;

    row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if !crate::WEB {
            if window_button(ui, Glyph::Close, p.danger) {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            let max = maximized(ctx);
            if window_button(ui, if max { Glyph::Restore } else { Glyph::Max }, p.primary) {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
            }
            if window_button(ui, Glyph::Min, p.primary) {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }
        }
        extra(ui);
    });

    ui.advance_cursor_after_rect(full);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Glyph {
    Min,
    Max,
    Restore,
    Close,
}

fn window_button(ui: &mut egui::Ui, glyph: Glyph, tint: egui::Color32) -> bool {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 24.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    if hot > 0.01 {
        // close goes the danger colour; the others take a panel fill
        let fill = if glyph == Glyph::Close {
            theme::wash(tint, (200.0 * hot) as u8)
        } else {
            theme::wash(p.raised, (255.0 * hot) as u8)
        };
        ui.painter().rect_filled(rect.shrink2(Vec2::new(3.0, 2.0)), egui::Rounding::same(theme::R_SM), fill);
    }
    let ink = theme::mix(p.text_dim, if glyph == Glyph::Close { egui::Color32::BLACK } else { p.text }, hot);
    let c = rect.center();
    let s = 4.6;
    let stroke = Stroke::new(1.3_f32, ink);
    match glyph {
        Glyph::Min => {
            ui.painter().line_segment(
                [Pos2::new(c.x - s, c.y + 0.5), Pos2::new(c.x + s, c.y + 0.5)],
                stroke,
            );
        }
        Glyph::Max => {
            ui.painter().rect_stroke(
                Rect::from_center_size(c, Vec2::splat(s * 1.9)),
                egui::Rounding::ZERO,
                stroke,
            );
        }
        Glyph::Restore => {
            ui.painter().rect_stroke(
                Rect::from_center_size(c + Vec2::new(-1.2, 1.2), Vec2::splat(s * 1.7)),
                egui::Rounding::ZERO,
                stroke,
            );
            ui.painter().line_segment(
                [
                    Pos2::new(c.x - s * 0.05, c.y - s * 0.85),
                    Pos2::new(c.x + s * 1.05, c.y - s * 0.85),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    Pos2::new(c.x + s * 1.05, c.y - s * 0.85),
                    Pos2::new(c.x + s * 1.05, c.y + s * 0.2),
                ],
                stroke,
            );
        }
        Glyph::Close => {
            ui.painter()
                .line_segment([Pos2::new(c.x - s, c.y - s), Pos2::new(c.x + s, c.y + s)], stroke);
            ui.painter()
                .line_segment([Pos2::new(c.x + s, c.y - s), Pos2::new(c.x - s, c.y + s)], stroke);
        }
    }
    resp.clicked()
}

/// Invisible strips along the window edges that begin a resize — needed
/// because there is no frame to grab.
pub fn resize_handles(ctx: &egui::Context) {
    use egui::viewport::ResizeDirection as D;
    const GRIP: f32 = 6.0;
    if maximized(ctx) {
        return;
    }

    let s = ctx.screen_rect();
    let edges: [(&str, Rect, D, egui::CursorIcon); 8] = [
        (
            "n",
            Rect::from_min_max(
                s.left_top() + Vec2::new(GRIP, 0.0),
                Pos2::new(s.right() - GRIP, s.top() + GRIP),
            ),
            D::North,
            egui::CursorIcon::ResizeNorth,
        ),
        (
            "s",
            Rect::from_min_max(
                Pos2::new(s.left() + GRIP, s.bottom() - GRIP),
                s.right_bottom() - Vec2::new(GRIP, 0.0),
            ),
            D::South,
            egui::CursorIcon::ResizeSouth,
        ),
        (
            "w",
            Rect::from_min_max(
                Pos2::new(s.left(), s.top() + GRIP),
                Pos2::new(s.left() + GRIP, s.bottom() - GRIP),
            ),
            D::West,
            egui::CursorIcon::ResizeWest,
        ),
        (
            "e",
            Rect::from_min_max(
                Pos2::new(s.right() - GRIP, s.top() + GRIP),
                Pos2::new(s.right(), s.bottom() - GRIP),
            ),
            D::East,
            egui::CursorIcon::ResizeEast,
        ),
        (
            "nw",
            Rect::from_min_size(s.left_top(), Vec2::splat(GRIP)),
            D::NorthWest,
            egui::CursorIcon::ResizeNorthWest,
        ),
        (
            "ne",
            Rect::from_min_size(Pos2::new(s.right() - GRIP, s.top()), Vec2::splat(GRIP)),
            D::NorthEast,
            egui::CursorIcon::ResizeNorthEast,
        ),
        (
            "sw",
            Rect::from_min_size(Pos2::new(s.left(), s.bottom() - GRIP), Vec2::splat(GRIP)),
            D::SouthWest,
            egui::CursorIcon::ResizeSouthWest,
        ),
        (
            "se",
            Rect::from_min_size(
                Pos2::new(s.right() - GRIP, s.bottom() - GRIP),
                Vec2::splat(GRIP),
            ),
            D::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
        ),
    ];

    egui::Area::new(egui::Id::new("ns-resize"))
        .order(egui::Order::Foreground)
        .fixed_pos(s.min)
        .interactable(true)
        .show(ctx, |ui| {
            for (name, rect, dir, cursor) in edges {
                let r = ui.interact(
                    rect,
                    egui::Id::new(("ns-resize", name)),
                    Sense::click_and_drag(),
                );
                if r.hovered() || r.dragged() {
                    ui.ctx().set_cursor_icon(cursor);
                }
                if r.drag_started() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }
            }
        });
}
