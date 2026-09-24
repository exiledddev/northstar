//! The mark.
//!
//! A north star: a four-pointed compass star with four shorter rays set
//! between its points, cut into facets with the light showing between them.
//! It is built the way Tesseract's mark is built — every facet pulled in toward
//! its own middle so the gaps draw the edges, filled from the theme's
//! secondary gradient, lit from the upper left — so the two marks sit on a
//! desktop as siblings.
//!
//! Everything is painted from primitives so the mark takes the current theme,
//! scales to any size and never depends on an image file being found.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use crate::theme::{self, pal};

/// One facet of the star: its corners, and how much light it catches (0..1).
struct Facet {
    pts: Vec<Pos2>,
    shade: f32,
}

/// The star's facets, back to front. `r` is the reach of the long points.
fn facets(centre: Pos2, r: f32) -> Vec<Facet> {
    let light = Vec2::new(-0.42, -1.0).normalized();
    let at = |angle: f32, dist: f32| -> Pos2 {
        Pos2::new(centre.x + angle.cos() * dist, centre.y + angle.sin() * dist)
    };
    let quarter = std::f32::consts::FRAC_PI_2;
    let eighth = std::f32::consts::FRAC_PI_4;
    let mut out = Vec::new();

    // the four short rays on the diagonals, behind the long ones
    for k in 0..4 {
        let a = -quarter + eighth + quarter * k as f32;
        let tip = at(a, r * 0.64);
        let l = at(a - 0.42, r * 0.30);
        let rr = at(a + 0.42, r * 0.30);
        for (pts, bias) in [(vec![tip, rr, centre], 0.10_f32), (vec![tip, centre, l], -0.10)] {
            let mid = (pts[0].to_vec2() + pts[1].to_vec2() + pts[2].to_vec2()) / 3.0;
            let v = (mid - centre.to_vec2()).normalized();
            // the short rays sit a step back, so they read as further away
            let shade = ((0.5 + 0.5 * v.dot(light)) * 0.55 + bias).clamp(0.0, 1.0);
            out.push(Facet { pts, shade });
        }
    }

    // the four long points, each split down its spine into a lit half and a
    // half turned away from the light
    for k in 0..4 {
        let a = -quarter + quarter * k as f32;
        let tip = at(a, r);
        let l = at(a - eighth, r * 0.36);
        let rr = at(a + eighth, r * 0.36);
        for (pts, bias) in [(vec![tip, rr, centre], 0.12_f32), (vec![tip, centre, l], -0.12)] {
            let mid = (pts[0].to_vec2() + pts[1].to_vec2() + pts[2].to_vec2()) / 3.0;
            let v = (mid - centre.to_vec2()).normalized();
            let shade = (0.5 + 0.5 * v.dot(light) + bias).clamp(0.0, 1.0);
            out.push(Facet { pts, shade });
        }
    }
    out
}

fn shrink_toward_middle(pts: &[Pos2], gap: f32) -> Vec<Pos2> {
    let mut mid = Vec2::ZERO;
    for q in pts {
        mid += q.to_vec2();
    }
    let mid = (mid / pts.len() as f32).to_pos2();
    pts.iter()
        .map(|q| {
            let away = *q - mid;
            let len = away.length().max(0.001);
            *q - away / len * gap
        })
        .collect()
}

/// Paint the mark into `rect`. `glow` in 0..=1 adds the bloom.
pub fn paint(painter: &egui::Painter, rect: Rect, glow: f32) {
    let p = pal();
    let centre = rect.center();
    let r = rect.width().min(rect.height()) * 0.49;
    if r < 2.0 {
        return;
    }
    let gap = (r * 0.05).max(0.45);

    if glow > 0.01 {
        theme::glow_star(painter, centre, r * 0.5, p.sec_grad.1, r * 2.2, glow);
    }

    let (a, b) = p.sec_grad;
    let lit = theme::lighten(b, 0.35);
    let deep = theme::darken(a, 0.3);
    for f in facets(centre, r) {
        let fill = theme::mix(deep, lit, f.shade);
        theme::fill_grad_poly(
            painter,
            &shrink_toward_middle(&f.pts, gap),
            theme::lighten(fill, 0.10),
            theme::darken(fill, 0.10),
            Vec2::new(0.3, 1.0),
        );
    }
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

/// The mark as an SVG, for the desktop icon. Written at install time so the
/// launcher icon matches whatever the app draws.
pub fn svg() -> String {
    // Bloodmoon dark — Northstar's own red, the way Tesseract's icon is Zen —
    // since a launcher icon cannot follow the in-app theme.
    let p = theme::palette(theme::ThemeId::Bloodmoon, true);
    let hex = |c: Color32| format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
    let poly = |pts: &[Pos2]| -> String {
        pts.iter()
            .map(|q| format!("{:.2},{:.2}", q.x, q.y))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let centre = Pos2::new(64.0, 64.0);
    let r = 44.0;

    let mut s = String::new();
    s.push_str(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 128 128\" \
         width=\"128\" height=\"128\">\n",
    );
    s.push_str(&format!(
        "  <rect width=\"128\" height=\"128\" rx=\"30\" fill=\"{}\"/>\n",
        hex(p.sunken)
    ));
    s.push_str(&format!(
        "  <rect x=\"1\" y=\"1\" width=\"126\" height=\"126\" rx=\"29.5\" \
         fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>\n",
        hex(p.line)
    ));
    // Flat tones rather than gradients: an icon is rendered at 24 pixels as
    // often as 128, by whichever SVG renderer the desktop carries, and flat
    // tones survive all of them.
    let (a, b) = p.sec_grad;
    let lit = theme::lighten(b, 0.30);
    let deep = theme::darken(a, 0.25);
    for f in facets(centre, r) {
        s.push_str(&format!(
            "  <polygon points=\"{}\" fill=\"{}\"/>\n",
            poly(&shrink_toward_middle(&f.pts, 1.6)),
            hex(theme::mix(deep, lit, f.shade))
        ));
    }
    s.push_str("</svg>\n");
    s
}
