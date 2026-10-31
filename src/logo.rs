//! The mark.
//!
//! Blackline's mark is drawn in NativeCompute's language: the company mark is
//! four slanted slabs of liquid chrome, and Blackline's is one of them, with
//! the black line beside it set in the signal blue. A wide capsule leaning
//! right, a thin one parallel to it.
//!
//! Everything is painted from primitives so the mark scales to any size and
//! never depends on an image file being found.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::theme::{self, pal};

/// The signal blue, the one colour the mark always carries.
pub const SIGNAL: Color32 = Color32::from_rgb(0x3B, 0x4C, 0xFF);

/// The two strokes of the mark in a unit square: (from, to, width), from the
/// bottom end to the top end.
type Stroke2 = ((f32, f32), (f32, f32), f32);
const STROKES: [Stroke2; 2] = [
    ((0.27, 0.83), (0.49, 0.17), 0.21),
    ((0.585, 0.83), (0.805, 0.17), 0.075),
];

/// A slanted capsule from `a` to `b`, `w` wide, with round ends.
fn capsule(a: Pos2, b: Pos2, w: f32) -> Vec<Pos2> {
    let along = (b - a).normalized();
    let side = Vec2::new(-along.y, along.x);
    let r = w * 0.5;
    let steps = 10;
    let mut pts = Vec::with_capacity(2 * steps + 2);
    let base = side.y.atan2(side.x);
    // around the far end, then the near one
    for (c, from) in [(b, base + std::f32::consts::PI), (a, base)] {
        for k in 0..=steps {
            let t = from + std::f32::consts::PI * k as f32 / steps as f32;
            pts.push(Pos2::new(c.x + t.cos() * r, c.y + t.sin() * r));
        }
    }
    pts
}

/// The mark's two shapes, laid into `rect`: the slab, then the line.
fn shapes(rect: Rect) -> [Vec<Pos2>; 2] {
    let side = rect.width().min(rect.height());
    let origin = rect.center() - Vec2::splat(side * 0.5);
    let at = |(x, y): (f32, f32)| origin + Vec2::new(x * side, y * side);
    STROKES.map(|(a, b, w)| capsule(at(a), at(b), w * side))
}

/// Paint the mark into `rect`. The slab is chrome on a dark ground and ink on
/// a light one; `_glow` is kept for callers and adds nothing: the language
/// has no glows.
pub fn paint(painter: &egui::Painter, rect: Rect, _glow: f32) {
    let p = pal();
    if rect.width().min(rect.height()) < 4.0 {
        return;
    }
    let [slab, line] = shapes(rect);
    let (lit, deep) = if p.dark {
        (Color32::from_rgb(0xF2, 0xF3, 0xF7), Color32::from_rgb(0x8A, 0x8F, 0xA8))
    } else {
        (Color32::from_rgb(0x2A, 0x2D, 0x3A), Color32::from_rgb(0x05, 0x06, 0x0A))
    };
    theme::fill_grad_poly(painter, &slab, lit, deep, Vec2::new(0.35, 1.0));
    painter.add(egui::Shape::convex_polygon(line, SIGNAL, Stroke::NONE));
}

/// The circular line spinner: one arc chasing its tail, drawn with a taper so
/// it looks like a stroke rather than a segment.
pub fn spinner(painter: &egui::Painter, centre: Pos2, radius: f32, t: f32, color: Color32) {
    let head = t * std::f32::consts::TAU * 1.35;
    // the sweep breathes between a short dash and most of the circle
    let sweep = 0.55 + 0.85 * (0.5 - 0.5 * (t * std::f32::consts::TAU * 0.9).cos());
    let steps = 64;
    let w = radius * 0.14;
    for i in 0..steps {
        let f0 = i as f32 / steps as f32;
        let f1 = (i + 1) as f32 / steps as f32;
        let a0 = head - sweep * (1.0 - f0);
        let a1 = head - sweep * (1.0 - f1);
        let fade = f0.powf(0.55);
        let col = theme::wash(color, (235.0 * fade) as u8);
        painter.line_segment(
            [
                Pos2::new(centre.x + a0.cos() * radius, centre.y + a0.sin() * radius),
                Pos2::new(centre.x + a1.cos() * radius, centre.y + a1.sin() * radius),
            ],
            Stroke::new(w * (0.55 + 0.45 * fade), col),
        );
    }
    // round the leading end off
    painter.circle_filled(
        Pos2::new(centre.x + head.cos() * radius, centre.y + head.sin() * radius),
        w * 0.5,
        color,
    );
}

/// The mark as an SVG, for the launcher icon: white on a black tile, the way
/// NativeCompute's own icon is white on a blue one. Written at install time
/// so the launcher icon is the mark the app draws.
pub fn svg() -> String {
    let poly = |pts: &[Pos2]| -> String {
        pts.iter()
            .map(|q| format!("{:.2},{:.2}", q.x, q.y))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let [slab, line] = shapes(Rect::from_min_size(Pos2::new(56.0, 56.0), Vec2::splat(400.0)));
    let mut s = String::new();
    s.push_str(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\" \
         width=\"512\" height=\"512\">\n  <title>Blackline</title>\n",
    );
    s.push_str("  <rect width=\"512\" height=\"512\" rx=\"116\" fill=\"#000000\"/>\n");
    s.push_str(&format!("  <polygon points=\"{}\" fill=\"#F2F3F7\"/>\n", poly(&slab)));
    s.push_str(&format!("  <polygon points=\"{}\" fill=\"#3B4CFF\"/>\n", poly(&line)));
    s.push_str("</svg>\n");
    s
}
