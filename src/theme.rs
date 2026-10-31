//! Tokens and the painting primitives the whole app is drawn with.
//!
//! This is NativeCompute's design language, taken from its website value for
//! value: void black, chrome and one signal blue; Anybody for display and UI,
//! JetBrains Mono for HUD readouts; hairline borders, 2px corners, and no
//! glass, glows or shadows. One look, with a dark cast and a light one.
//!
//! The field names still say "glass" and "solid" in places; both are opaque
//! panel fills now, kept so every caller reads the palette the same way.

// The kit is kept whole: a primitive with no caller today is still part of
// the language.
#![allow(dead_code)]

use std::sync::RwLock;

use crate::model::Element;

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Margin, Mesh, Pos2, Rect, Rounding,
    Shape, Stroke, Vec2,
};

// ----------------------------------------------------------------- palette --

/// Every colour the app draws with. Copied out of the global on each read, so
/// it is deliberately small and plain.
#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,

    /// What the window is cleared to.
    pub backdrop: Color32,

    // panels that sit over the page
    pub glass: Color32,
    pub glass_hi: Color32,
    pub glass_line: Color32,
    pub glass_edge: Color32,

    // panels you work in
    pub solid: Color32,
    pub solid_hi: Color32,
    pub sunken: Color32,
    pub raised: Color32,

    pub line: Color32,
    pub line_strong: Color32,

    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    /// Text laid on a primary-filled shape.
    pub ink: Color32,

    pub primary: Color32,
    pub primary_light: Color32,
    pub primary_deep: Color32,
    pub primary_quiet: Color32,
    pub primary_quiet_hi: Color32,
    pub prim_grad: (Color32, Color32),

    pub sec: Color32,
    pub sec_light: Color32,
    pub sec_deep: Color32,
    pub sec_grad: (Color32, Color32),

    pub danger: Color32,
    pub danger_light: Color32,
    pub warn: Color32,
    pub ok: Color32,

    pub groups: [Color32; 6],
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Palette {
    pub fn group(&self, hue: usize) -> Color32 {
        self.groups[hue % self.groups.len()]
    }
}

/// Blackline's one look, NativeCompute's: void black, chrome, and one signal
/// blue, with a light paper variant in the same language.
pub fn palette(dark: bool) -> Palette {
    // white or black at a given strength, for the hairlines
    let a = |base: u8, alpha: u8| Color32::from_rgba_unmultiplied(base, base, base, alpha);
    if dark {
        Palette {
            dark,
            backdrop: c(0x00, 0x00, 0x00),
            glass: c(0x07, 0x08, 0x0D),
            glass_hi: c(0x13, 0x16, 0x24),
            glass_line: a(255, 31),
            glass_edge: a(255, 66),
            solid: c(0x07, 0x08, 0x0D),
            solid_hi: c(0x0C, 0x0E, 0x16),
            sunken: c(0x00, 0x00, 0x00),
            raised: c(0x13, 0x16, 0x24),
            line: a(255, 31),
            line_strong: a(255, 66),
            text: c(0xF2, 0xF3, 0xF7),
            text_dim: c(0xA6, 0xAB, 0xC2),
            text_faint: c(0x8A, 0x8F, 0xA8),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0x3B, 0x4C, 0xFF),
            primary_light: c(0x6E, 0x7B, 0xFF),
            primary_deep: c(0x2A, 0x36, 0xC8),
            primary_quiet: Color32::from_rgba_unmultiplied(0x3B, 0x4C, 0xFF, 36),
            primary_quiet_hi: Color32::from_rgba_unmultiplied(0x3B, 0x4C, 0xFF, 56),
            prim_grad: (c(0x3B, 0x4C, 0xFF), c(0x3B, 0x4C, 0xFF)),
            sec: c(0xC9, 0xCD, 0xDB),
            sec_light: c(0xF2, 0xF3, 0xF7),
            sec_deep: c(0x8A, 0x8F, 0xA8),
            // chrome: the slabs of the mark, lit from above
            sec_grad: (c(0x8A, 0x8F, 0xA8), c(0xF2, 0xF3, 0xF7)),
            danger: c(0xFF, 0x6B, 0x7A),
            danger_light: c(0xFF, 0x9A, 0xA5),
            warn: c(0xF5, 0xC4, 0x51),
            ok: c(0x5E, 0xE0, 0xA0),
            groups: [
                c(0x6E, 0x7B, 0xFF),
                c(0x5C, 0xC8, 0xFF),
                c(0xA7, 0x8B, 0xFA),
                c(0x5E, 0xE0, 0xA0),
                c(0xF5, 0xC4, 0x51),
                c(0xFF, 0x6B, 0x7A),
            ],
        }
    } else {
        Palette {
            dark,
            backdrop: c(0xF4, 0xF5, 0xF8),
            glass: c(0xFF, 0xFF, 0xFF),
            glass_hi: c(0xEC, 0xEE, 0xF4),
            glass_line: a(0, 31),
            glass_edge: a(0, 66),
            solid: c(0xFF, 0xFF, 0xFF),
            solid_hi: c(0xF6, 0xF7, 0xFA),
            sunken: c(0xEC, 0xEE, 0xF4),
            raised: c(0xE6, 0xE9, 0xF1),
            line: a(0, 31),
            line_strong: a(0, 66),
            text: c(0x0B, 0x0C, 0x12),
            text_dim: c(0x45, 0x4A, 0x5E),
            text_faint: c(0x5F, 0x64, 0x7A),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0x3B, 0x4C, 0xFF),
            primary_light: c(0x2B, 0x3A, 0xE0),
            primary_deep: c(0x1F, 0x2A, 0xB0),
            primary_quiet: Color32::from_rgba_unmultiplied(0x3B, 0x4C, 0xFF, 26),
            primary_quiet_hi: Color32::from_rgba_unmultiplied(0x3B, 0x4C, 0xFF, 44),
            prim_grad: (c(0x3B, 0x4C, 0xFF), c(0x3B, 0x4C, 0xFF)),
            sec: c(0x2A, 0x2D, 0x3A),
            sec_light: c(0x0B, 0x0C, 0x12),
            sec_deep: c(0x5F, 0x64, 0x7A),
            sec_grad: (c(0x05, 0x06, 0x0A), c(0x3A, 0x3E, 0x4E)),
            danger: c(0xD1, 0x24, 0x3B),
            danger_light: c(0xE5, 0x4B, 0x5E),
            warn: c(0xB7, 0x7B, 0x00),
            ok: c(0x14, 0x8A, 0x4F),
            groups: [
                c(0x3B, 0x4C, 0xFF),
                c(0x00, 0x86, 0xC2),
                c(0x7C, 0x4D, 0xE8),
                c(0x14, 0x8A, 0x4F),
                c(0xB7, 0x7B, 0x00),
                c(0xD1, 0x24, 0x3B),
            ],
        }
    }
}

static CURRENT: RwLock<Option<Palette>> = RwLock::new(None);

/// The palette everything paints with.
pub fn pal() -> Palette {
    if let Ok(g) = CURRENT.read() {
        if let Some(p) = *g {
            return p;
        }
    }
    palette(true)
}

/// Dark or light.
pub fn set_palette(dark: bool) {
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(palette(dark));
    }
}

// ----------------------------------------------------------------- metrics --

/// The window's own corner radius.
pub const R_WINDOW: f32 = 2.0;
pub const R_SM: f32 = 2.0;
pub const R_MD: f32 = 2.0;
pub const R_CTRL: f32 = 2.0;
pub const R_CARD: f32 = 2.0;
pub const R_ISLAND: f32 = 2.0;
pub const R_PANEL: f32 = 2.0;
pub const R_PILL: f32 = 2.0;

pub const RAIL_W: f32 = 268.0;
pub const OUTLINE_W: f32 = 246.0;
pub const INSPECT_W: f32 = 272.0;
/// Title bar plus ribbon, with room around the ribbon's controls for the light
/// they cast — clip that and the glow gets a straight edge.
pub const TOPBAR_H: f32 = 92.0;
pub const DOC_MAX: f32 = 820.0;
/// The gap of blurred backdrop between the islands. Everything breathes on
/// this one number.
pub const GAP: f32 = 14.0;

pub const T_TITLE: f32 = 26.0;
pub const T_H: f32 = 17.0;
pub const T_BODY: f32 = 14.5;
pub const T_SM: f32 = 13.0;
pub const T_LABEL: f32 = 12.0;
pub const T_CAP: f32 = 11.0;
pub const T_MICRO: f32 = 10.0;

// ------------------------------------------------------------- colour math --

pub fn wash(col: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), alpha)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

pub fn lighten(col: Color32, t: f32) -> Color32 {
    mix(col, Color32::WHITE, t)
}

pub fn darken(col: Color32, t: f32) -> Color32 {
    mix(col, Color32::BLACK, t)
}

/// Readable text colour for something sitting on `bg`.
pub fn on(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 150.0 {
        c(0x10, 0x12, 0x11)
    } else {
        c(0xF4, 0xF7, 0xF5)
    }
}

// ------------------------------------------------------------------ shapes --

/// The outline of a rounded rectangle, as a convex polygon. Corners get a few
/// segments each, which is plenty at the radii we use and keeps the meshes
/// small enough to throw away every frame.
pub fn rounded_poly(rect: Rect, radius: f32) -> Vec<Pos2> {
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5).max(0.0);
    if r <= 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let steps = ((r * 0.9) as usize).clamp(5, 18);
    let mut pts = Vec::with_capacity(steps * 4 + 4);
    let corners = [
        (rect.right() - r, rect.top() + r, -std::f32::consts::FRAC_PI_2),
        (rect.right() - r, rect.bottom() - r, 0.0),
        (rect.left() + r, rect.bottom() - r, std::f32::consts::FRAC_PI_2),
        (rect.left() + r, rect.top() + r, std::f32::consts::PI),
    ];
    for (cx, cy, a0) in corners {
        for i in 0..=steps {
            let a = a0 + std::f32::consts::FRAC_PI_2 * (i as f32 / steps as f32);
            pts.push(Pos2::new(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    pts
}

/// Fill a convex polygon with a linear gradient running along `dir`.
pub fn fill_grad_poly(painter: &egui::Painter, pts: &[Pos2], a: Color32, b: Color32, dir: Vec2) {
    painter.add(grad_poly_shape(pts, a, b, dir));
}

/// The same gradient as a shape, for anything that has to be slotted in
/// underneath content that was laid out after it.
pub fn grad_poly_shape(pts: &[Pos2], a: Color32, b: Color32, dir: Vec2) -> Shape {
    if pts.len() < 3 {
        return Shape::Noop;
    }
    // egui feathers the shapes it tessellates itself; a mesh built by hand gets
    // none, which is what makes a gradient's edge look chewed. One extra ring
    // a pixel out at zero alpha does the same job.
    let dir = if dir.length_sq() < 1e-6 {
        Vec2::new(0.0, 1.0)
    } else {
        dir.normalized()
    };
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for p in pts {
        let t = p.to_vec2().dot(dir);
        lo = lo.min(t);
        hi = hi.max(t);
    }
    let span = (hi - lo).max(0.0001);
    let at = |p: Pos2| -> Color32 { mix(a, b, (p.to_vec2().dot(dir) - lo) / span) };

    let mut centre = Vec2::ZERO;
    for p in pts {
        centre += p.to_vec2();
    }
    let centre = (centre / pts.len() as f32).to_pos2();

    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, at(centre));
    for p in pts {
        mesh.colored_vertex(*p, at(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }

    const FEATHER: f32 = 0.9;
    let base = mesh.vertices.len() as u32;
    for p in pts {
        let out = (*p - centre).normalized() * FEATHER;
        let q = *p + out;
        mesh.colored_vertex(q, wash(at(*p), 0));
    }
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.add_triangle(1 + i, base + i, base + j);
        mesh.add_triangle(1 + i, base + j, 1 + j);
    }
    Shape::mesh(mesh)
}

/// A rounded rectangle filled with a gradient. `dir` defaults to top→bottom.
pub fn grad_rect(painter: &egui::Painter, rect: Rect, radius: f32, a: Color32, b: Color32, dir: Vec2) {
    let pts = rounded_poly(rect, radius);
    fill_grad_poly(painter, &pts, a, b, dir);
}

/// A rhombus filled with a gradient — the theme swatch's marker.
pub fn grad_diamond(painter: &egui::Painter, centre: Pos2, r: f32, a: Color32, b: Color32) {
    let pts = vec![
        Pos2::new(centre.x, centre.y - r),
        Pos2::new(centre.x + r, centre.y),
        Pos2::new(centre.x, centre.y + r),
        Pos2::new(centre.x - r, centre.y),
    ];
    fill_grad_poly(painter, &pts, a, b, Vec2::new(0.6, 1.0));
}

/// An arc that fades out at both ends — the two quarters of outline on a
/// glass orb.
pub fn fading_arc(
    painter: &egui::Painter,
    centre: Pos2,
    radius: f32,
    from: f32,
    to: f32,
    width: f32,
    color: Color32,
) {
    let steps = 22;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let a0 = from + (to - from) * t0;
        let a1 = from + (to - from) * t1;
        // full strength in the middle of the arc, nothing at the ends
        let mid = (t0 + t1) * 0.5;
        let fade = (mid * std::f32::consts::PI).sin().powf(0.7);
        let col = wash(color, (color.a() as f32 * fade) as u8);
        painter.line_segment(
            [
                Pos2::new(centre.x + a0.cos() * radius, centre.y + a0.sin() * radius),
                Pos2::new(centre.x + a1.cos() * radius, centre.y + a1.sin() * radius),
            ],
            Stroke::new(width, col),
        );
    }
}

// ------------------------------------------------------------------- bloom --

/// How finely a bloom is tessellated. Wider blooms get more rings, but the
/// alpha is interpolated *between* them by the rasteriser, so this is about
/// following the falloff curve, not about hiding steps.
fn bloom_rings(spread: f32) -> usize {
    ((spread * 0.40) as usize).clamp(10, 30)
}

/// The curve a glow falls off along.
///
/// The point of a neon surface is that the *object* is bright and the light
/// around it is a wash you barely notice until it is not there. A hot, tight
/// halo reads as a sticker of a glow; this is mostly a long, low tail with
/// just enough near the object to seat it.
fn falloff(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Deliberately no spike at t=0: a glow that is at full strength right at
    // the object's edge draws a bright rim around it, which is exactly the
    // look this is trying to get away from.
    let near = (-3.2 * t).exp();
    let far = (1.0 - t).powf(2.6);
    (0.22 * near + 0.78 * far) * 0.82
}

/// A bloom drawn as **one mesh**: concentric rings of vertices whose alpha
/// falls away smoothly. Stacked translucent shapes are what band; a single
/// mesh blends once per pixel and the rasteriser interpolates the alpha
/// between rings, so the result is as smooth as the curve itself.
///
/// `ring` is handed a distance from the shape and returns the outline there.
/// Every ring must carry the same number of points.
/// Glows are not part of the language.
const GLOWS: bool = false;

fn bloom_mesh<F>(
    painter: &egui::Painter,
    centre: Pos2,
    spread: f32,
    color: Color32,
    strength: f32,
    ring: F,
) where
    F: Fn(f32) -> Vec<Pos2>,
{
    // The NativeCompute language has no glows: every bloom is off. The
    // primitive stays so its callers keep their shape.
    if !GLOWS || strength <= 0.002 || spread <= 0.1 {
        return;
    }
    // Low. A glow is atmosphere, not a second object drawn behind the first.
    let peak = (strength.clamp(0.0, 2.0) * 62.0).min(255.0) as u8;
    let rings = bloom_rings(spread);

    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, wash(color, peak));

    let mut prev: Option<(u32, usize)> = None;
    for k in 0..=rings {
        let t = k as f32 / rings as f32;
        let pts = ring(spread * t);
        if pts.len() < 3 {
            return;
        }
        let col = wash(color, (peak as f32 * falloff(t)) as u8);
        let base = mesh.vertices.len() as u32;
        for q in &pts {
            mesh.colored_vertex(*q, col);
        }
        let n = pts.len() as u32;
        match prev {
            None => {
                for i in 0..n {
                    mesh.add_triangle(0, base + i, base + (i + 1) % n);
                }
            }
            Some((pbase, pcount)) => {
                if pcount != pts.len() {
                    return;
                }
                for i in 0..n {
                    let j = (i + 1) % n;
                    mesh.add_triangle(pbase + i, base + i, base + j);
                    mesh.add_triangle(pbase + i, base + j, pbase + j);
                }
            }
        }
        prev = Some((base, pts.len()));
    }
    painter.add(Shape::mesh(mesh));
}

const BLOOM_SEGMENTS: usize = 44;

/// A round neon bloom.
pub fn glow_circle(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, |grow| {
        let rr = r + grow;
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                Pos2::new(centre.x + a.cos() * rr, centre.y + a.sin() * rr)
            })
            .collect()
    });
}

/// A neon bloom around a rounded rectangle.
pub fn glow_rect(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    let steps = 9usize;
    bloom_mesh(painter, rect.center(), spread, color, strength, move |grow| {
        let r = rect.expand(grow);
        let rad = (radius + grow).min(r.width() * 0.5).min(r.height() * 0.5);
        let quarter = std::f32::consts::FRAC_PI_2;
        let corners = [
            (Pos2::new(r.right() - rad, r.top() + rad), -quarter),
            (Pos2::new(r.right() - rad, r.bottom() - rad), 0.0),
            (Pos2::new(r.left() + rad, r.bottom() - rad), quarter),
            (Pos2::new(r.left() + rad, r.top() + rad), std::f32::consts::PI),
        ];
        let mut pts = Vec::with_capacity(steps * 4 + 4);
        for (c, a0) in corners {
            for i in 0..=steps {
                let a = a0 + quarter * (i as f32 / steps as f32);
                pts.push(Pos2::new(c.x + a.cos() * rad, c.y + a.sin() * rad));
            }
        }
        pts
    });
}

/// A bloom the shape of the rhombus it comes off, so the marker itself is the
/// light source rather than something with a lamp behind it. The halo rounds
/// off as it spreads, the way a real one does.
pub fn glow_diamond(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, move |grow| {
        let rr = r + grow;
        // a superellipse that starts as a diamond and relaxes towards a circle
        let n = 1.0 + 1.15 * (grow / spread.max(0.001)).clamp(0.0, 1.0);
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                let (c, s) = (a.cos(), a.sin());
                let d = (c.abs().powf(n) + s.abs().powf(n)).powf(1.0 / n);
                let k = rr / d.max(0.001);
                Pos2::new(centre.x + c * k, centre.y + s * k)
            })
            .collect()
    });
}
/// The four points of Blackline's star, clockwise from the top, with the
/// waist between each pair. `waist` is the inner radius as a fraction of `r`.
pub fn star_points(centre: Pos2, r: f32, waist: f32) -> [Pos2; 8] {
    let w = r * waist * std::f32::consts::FRAC_1_SQRT_2;
    [
        Pos2::new(centre.x, centre.y - r),
        Pos2::new(centre.x + w, centre.y - w),
        Pos2::new(centre.x + r, centre.y),
        Pos2::new(centre.x + w, centre.y + w),
        Pos2::new(centre.x, centre.y + r),
        Pos2::new(centre.x - w, centre.y + w),
        Pos2::new(centre.x - r, centre.y),
        Pos2::new(centre.x - w, centre.y - w),
    ]
}

/// The old four-pointed star, filled with a gradient. `grad_star` now draws
/// the square marker; this is kept for anything that still wants the star.
///
/// A star is not convex, so it is laid down as four kites (tip, waist, centre,
/// waist), each convex, each feathered, all sharing one gradient so the seams
/// never show.
///
/// In the NativeCompute language a marker is a small square, the way the
/// site marks the page you are on: `r` sets its size, `b` its colour.
pub fn grad_star(painter: &egui::Painter, centre: Pos2, r: f32, _a: Color32, b: Color32) {
    painter.add(marker(centre, r, b));
}

/// The square marker on its own.
pub fn marker(centre: Pos2, r: f32, color: Color32) -> Shape {
    let side = (r * 1.1).round().max(4.0);
    Shape::rect_filled(Rect::from_center_size(centre, Vec2::splat(side)), Rounding::ZERO, color)
}

/// The same marker as shapes, for slotting in under content laid out later.
pub fn star_shapes(centre: Pos2, r: f32, _waist: f32, _a: Color32, b: Color32) -> Vec<Shape> {
    vec![marker(centre, r, b)]
}

/// The four-pointed star, kept for anything that still wants it drawn.
pub fn star_kites(centre: Pos2, r: f32, waist: f32, a: Color32, b: Color32) -> Vec<Shape> {
    let pts = star_points(centre, r, waist);
    // one gradient over the whole star, not four copies of it
    let dir = Vec2::new(0.6, 1.0).normalized();
    let (lo, hi) = (
        (centre.to_vec2() - Vec2::splat(r)).dot(dir),
        (centre.to_vec2() + Vec2::splat(r)).dot(dir),
    );
    let at = |p: Pos2| mix(a, b, (p.to_vec2().dot(dir) - lo) / (hi - lo).max(0.0001));
    (0..4)
        .map(|k| {
            let tip = pts[k * 2];
            let right = pts[k * 2 + 1];
            let left = pts[(k * 2 + 7) % 8];
            let kite = [tip, right, centre, left];
            let (ca, cb) = (at(tip), at(centre));
            // run each kite's own gradient between the colours its ends take
            // in the shared one
            let kd = centre - tip;
            let kd = if kd.length_sq() < 1e-6 { dir } else { kd };
            grad_poly_shape(&kite, ca, cb, kd)
        })
        .collect()
}

/// A bloom the shape of the four-pointed star it comes off, relaxing into a
/// circle as it spreads. A no-op while the language has no glows.
pub fn glow_star(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, move |grow| {
        let rr = r + grow;
        // n < 1 is a concave, four-pointed curve; by n = 2 it is a circle
        let n = 0.72 + 1.43 * (grow / spread.max(0.001)).clamp(0.0, 1.0);
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                let (c, s) = (a.cos(), a.sin());
                let d = (c.abs().powf(n) + s.abs().powf(n)).powf(1.0 / n);
                let k = rr / d.max(0.001);
                Pos2::new(centre.x + c * k, centre.y + s * k)
            })
            .collect()
    });
}

/// The body of a pane of glass.
///
/// White-on-white is not frosted glass, it is nothing at all — so on a light
/// page the pane is tinted *down* towards the sunken tone instead of up
/// towards white, and it is the sheen and the rim that put the light back.
pub fn glass_body(opacity: f32) -> Color32 {
    let _ = opacity;
    pal().glass
}

/// What every control does when you reach for it.
///
/// The surface takes a *breath* of the accent — it does not become a different
/// colour — and only the outline goes bright. Anything more and a row of
/// buttons turns into a row of lamps.
pub fn hover_surface(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    accent: Color32,
    t: f32,
    on: bool,
) {
    // NativeCompute's quiet control: a panel fill comes in under it, and a
    // chosen one also wears a hairline in the signal colour
    let p = pal();
    let t = t.clamp(0.0, 1.0);
    if t <= 0.005 && !on {
        return;
    }
    let lit = t.max(if on { 1.0 } else { 0.0 });
    let fill = if p.dark { p.raised } else { p.sunken };
    painter.rect_filled(rect, Rounding::same(radius), wash(fill, (255.0 * lit) as u8));
    let edge = if on { accent } else { p.line_strong };
    painter.rect_stroke(
        rect.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(1.0_f32, wash(edge, ((if on { 255.0 } else { 140.0 }) * lit) as u8)),
    );
}

/// A glassmorphic surface: a pane of frosted glass with light caught along its
/// top edge.
///
/// Three layers, in the order real glass gives you them — a dark translucent
/// body so text stays readable over whatever is behind, a sheen falling down
/// from the top, and a lit rim. Anything that floats above the page uses this:
/// alerts, questions, popovers, the index cards.
pub fn glass_surface(painter: &egui::Painter, rect: Rect, radius: f32, opacity: f32) {
    // the site's panel: opaque, a hairline round it, and a faint light along
    // its top edge
    let _ = opacity;
    let p = pal();
    painter.rect_filled(rect, Rounding::same(radius), p.solid_hi);
    painter.rect_stroke(rect.shrink(0.5), Rounding::same(radius), Stroke::new(1.0_f32, p.line_strong));
    top_edge(painter, rect);
}

/// The inset light along a panel's top edge (`--edge` on the site).
pub fn top_edge(painter: &egui::Painter, rect: Rect) {
    let p = pal();
    let a = if p.dark { 15 } else { 0 };
    if a > 0 {
        painter.line_segment(
            [rect.left_top() + Vec2::new(1.0, 1.5), rect.right_top() + Vec2::new(-1.0, 1.5)],
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, a)),
        );
    }
}

/// Glass: a translucent plate with a soft lift down it, so it reads as a pane
/// of something rather than a flat rectangle.
pub fn glass_plate(painter: &egui::Painter, rect: Rect, radius: f32, opacity: f32) {
    let _ = opacity;
    let p = pal();
    painter.rect_filled(rect, Rounding::same(radius), p.glass);
    painter.rect_stroke(rect.shrink(0.5), Rounding::same(radius), Stroke::new(1.0_f32, p.line));
}

/// A solid island: opaque, rounded, hairlined, with a drop shadow so it lifts
/// off the blurred backdrop.
pub fn island_frame(radius: f32) -> egui::Frame {
    // the site's panel: `--deep`, a hairline, square corners, no shadow
    let p = pal();
    egui::Frame::none()
        .fill(p.solid)
        .rounding(Rounding::same(radius))
        .stroke(Stroke::new(1.0_f32, p.line))
}

/// Letter-spaced text, laid out as one run.
///
/// Placing glyphs one at a time — measuring each and adding the tracking by
/// hand — snaps every glyph to a whole pixel on its own and drops the font's
/// kerning, and the gaps between letters come out uneven. egui can space a
/// run itself; laid out once, the spacing is even at any size. Returns the
/// width drawn.
pub fn tracked_text(
    painter: &egui::Painter,
    left_centre: Pos2,
    text: &str,
    font: egui::FontId,
    color: Color32,
    tracking: f32,
) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: tracking,
            ..Default::default()
        },
    );
    let galley = painter.ctx().fonts(|f| f.layout_job(job));
    let size = galley.rect.size();
    painter.galley(
        Pos2::new(left_centre.x, left_centre.y - size.y * 0.5),
        galley,
        color,
    );
    // the tracking after the last letter is not part of the word
    (size.x - tracking).max(0.0)
}

/// The width `tracked_text` would draw `text` at.
pub fn tracked_width(ctx: &egui::Context, text: &str, font: egui::FontId, tracking: f32) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font,
            extra_letter_spacing: tracking,
            ..Default::default()
        },
    );
    let w = ctx.fonts(|f| f.layout_job(job)).rect.width();
    (w - tracking).max(0.0)
}

/// A soft lift under something that floats: several shadows, each wider and
/// fainter than the last, rather than one hard one or a coloured glow — the
/// way light really falls off under a card held above a page. `lift` 0..1.
pub fn lift_shadow(painter: &egui::Painter, rect: Rect, radius: f32, lift: f32) {
    // nothing floats in this language: panels sit flat, told apart by their
    // hairlines
    if !GLOWS {
        let _ = (painter, rect, radius, lift);
        return;
    }
    let p = pal();
    let lift = lift.clamp(0.0, 1.0);
    if lift <= 0.01 {
        return;
    }
    let rounding = Rounding::same(radius.min(rect.width() * 0.5).min(rect.height() * 0.5));
    let base = if p.dark { 1.0 } else { 0.32 };
    for (dy, blur, alpha) in [(1.0_f32, 2.0_f32, 60.0_f32), (4.0, 10.0, 38.0), (12.0, 28.0, 26.0)] {
        painter.add(
            egui::epaint::Shadow {
                offset: Vec2::new(0.0, dy * (0.5 + 0.5 * lift)),
                blur: blur * (0.6 + 0.4 * lift),
                spread: 0.0,
                color: Color32::from_black_alpha((alpha * base * lift) as u8),
            }
            .as_shape(rect, rounding),
        );
    }
}

// ------------------------------------------------------------------ visuals --

pub fn apply(ctx: &egui::Context) {
    let p = pal();
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    v.override_text_color = Some(p.text);
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = p.solid;
    // text fields: the site's are black with a strong hairline
    v.extreme_bg_color = if p.dark { Color32::BLACK } else { Color32::WHITE };
    v.faint_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0_f32, p.line);
    v.window_rounding = Rounding::same(R_PANEL);
    v.menu_rounding = Rounding::same(R_MD);
    v.selection.bg_fill = wash(p.primary, if p.dark { 150 } else { 90 });
    v.selection.stroke = Stroke::new(1.0_f32, p.primary);
    v.hyperlink_color = p.primary_light;
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_shadow = v.popup_shadow;
    v.text_cursor.stroke = Stroke::new(2.0_f32, p.primary_light);
    v.window_stroke = Stroke::new(1.0_f32, p.line_strong);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.solid;
    w.noninteractive.weak_bg_fill = p.solid;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_dim);
    w.noninteractive.rounding = Rounding::same(R_CTRL);

    w.inactive.bg_fill = p.sunken;
    w.inactive.weak_bg_fill = p.sunken;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, p.line_strong);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.inactive.rounding = Rounding::same(R_CTRL);

    w.hovered.bg_fill = p.solid_hi;
    w.hovered.weak_bg_fill = p.solid_hi;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, p.line_strong);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.hovered.rounding = Rounding::same(R_CTRL);
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.primary_quiet_hi;
    w.active.weak_bg_fill = p.primary_quiet_hi;
    w.active.bg_stroke = Stroke::new(1.0_f32, p.primary);
    w.active.fg_stroke = Stroke::new(1.0_f32, p.primary_light);
    w.active.rounding = Rounding::same(R_CTRL);

    w.open.bg_fill = p.raised;
    w.open.weak_bg_fill = p.raised;
    w.open.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.open.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.open.rounding = Rounding::same(R_CTRL);

    // The palette says light or dark, not the system. egui keeps a style for
    // each and picks by the system's setting — which a browser follows, and
    // which can disagree with the theme chosen here — so both get this one.
    ctx.all_styles_mut(|s| {
        s.visuals = v.clone();
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(11.0, 6.0);
        s.spacing.window_margin = Margin::same(18.0);
        s.spacing.menu_margin = Margin::same(6.0);
        s.spacing.scroll.bar_width = 8.0;
        s.spacing.scroll.floating = true;
        s.visuals.clip_rect_margin = 2.0;
    });
}


// ------------------------------------------------------------ script colours --


/// The accent an element wears in the gutter tag and the element bar. Taken
/// from the theme, never fixed, so every element follows the palette.
///
/// Seven elements, seven hues a theme already owns: the scene heading takes
/// the energy colour, a cue the accent, and the rest are spread across the
/// theme's group tints so no two elements that sit next to each other on a
/// page share a colour. Action — most of any script — stays neutral.
pub fn element_color(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::SceneHeading => p.sec_light,
        Element::Action => p.text_faint,
        Element::Character => p.primary_light,
        Element::Parenthetical => p.group(3),
        Element::Dialogue => p.group(2),
        Element::Transition => p.group(4),
        Element::Shot => p.group(5),
        Element::Act => p.primary_light,
    }
}

/// The band behind a block in the editor, that tells one element from the
/// next at a glance. Faint enough to read through, never on the printed page.
pub fn element_band(e: Element) -> Color32 {
    let p = pal();
    match e {
        // action is most of any script: it carries no band at all, and an
        // act draws its own
        Element::Action | Element::Act => Color32::TRANSPARENT,
        _ => tint(element_color(e), if p.dark { 0.035 } else { 0.05 }),
    }
}

/// `col` laid over whatever is under it at exactly `k` of its strength.
///
/// `wash` goes through egui's unmultiplied constructor, which premultiplies
/// in linear light — so a very low alpha on a bright colour comes out several
/// times stronger than the number says (alpha 6 on a pale colour lands near
/// 15%). For the faintest tints, the ones that must stay a whisper, the
/// colour is premultiplied here, in the same space it is blended in.
pub fn tint(col: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let f = |v: u8| (v as f32 * k).round() as u8;
    Color32::from_rgba_premultiplied(f(col.r()), f(col.g()), f(col.b()), (255.0 * k).round() as u8)
}

/// A colour of its own for each speaking character, in order of first
/// appearance — a golden-ratio walk round the hue circle, so neighbours are
/// far apart and a new character never repaints the ones before it. `seed`
/// turns the whole wheel, for Settings' Shuffle.
pub fn character_colors(names: &[String], seed: u32) -> std::collections::HashMap<String, Color32> {
    voice_colors(names, seed, &[])
}

/// The saturation and lightness a character's colour is drawn at in the app.
fn voice_sl() -> (f32, f32) {
    if pal().dark {
        (0.62, 0.72)
    } else {
        (0.66, 0.36)
    }
}

/// Every speaker's colour, with the ones `chosen` for them (name, hue 0–359)
/// kept for continuity: a chosen hue is drawn at the same depth as a dealt
/// one, so it reads in this theme. Anyone not chosen is dealt one from the
/// wheel, as in Random. An empty `chosen` is exactly Random.
pub fn voice_colors(names: &[String], seed: u32, chosen: &[(String, u16)]) -> std::collections::HashMap<String, Color32> {
    let (s, l) = voice_sl();
    with_chosen(voices_at(names, seed, s, l), chosen, s, l)
}

/// The same, as ink for paper.
pub fn voice_inks(names: &[String], seed: u32, chosen: &[(String, u16)]) -> std::collections::HashMap<String, [u8; 3]> {
    with_chosen(voices_at(names, seed, 0.70, 0.34), chosen, 0.70, 0.34)
        .into_iter()
        .map(|(n, c)| (n, [c.r(), c.g(), c.b()]))
        .collect()
}

/// A hue as a character's colour in this theme: what a swatch in the colour
/// picker shows.
pub fn voice_of_hue(hue: u16) -> Color32 {
    let (s, l) = voice_sl();
    hsl(hue as f32 / 360.0, s, l)
}

fn with_chosen(
    mut dealt: std::collections::HashMap<String, Color32>,
    chosen: &[(String, u16)],
    s: f32,
    l: f32,
) -> std::collections::HashMap<String, Color32> {
    for (name, hue) in chosen {
        if let Some(c) = dealt.get_mut(name) {
            *c = hsl(*hue as f32 / 360.0, s, l);
        }
    }
    dealt
}

/// The same colours for paper: each speaker keeps their hue — so they are
/// recognisably the colour they wear in the app — at a depth that reads as
/// ink on white, whatever theme the app is in.
pub fn character_inks(names: &[String], seed: u32) -> std::collections::HashMap<String, [u8; 3]> {
    voice_inks(names, seed, &[])
}

fn voices_at(names: &[String], seed: u32, s: f32, l: f32) -> std::collections::HashMap<String, Color32> {
    let start = (seed as f32 * 0.137_5).fract();
    names
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let h = (start + k as f32 * 0.618_034).fract();
            (n.clone(), hsl(h, s, l))
        })
        .collect()
}

/// Hue, saturation, lightness, all 0..1, to a colour.
pub fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.fract() + 1.0).fract() * 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    let to = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    c3(to(r), to(g), to(b))
}

fn c3(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// The ink an element's text is set in on the page. Bodies sit a step down
/// from full-strength text; the cues and headings
/// that structure the page stay at full strength.
pub fn element_ink(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::SceneHeading | Element::Shot | Element::Character | Element::Act => p.text,
        Element::Dialogue => mix(p.text, p.text_dim, 0.15),
        Element::Action => mix(p.text, p.text_dim, 0.35),
        Element::Parenthetical | Element::Transition => p.text_dim,
    }
}

// -------------------------------------------------------------------- fonts --

/// NativeCompute's type ships with the app: Anybody for everything you read
/// and the giant display words, JetBrains Mono for the HUD readouts. Anybody
/// is a variable font with a width axis, which egui cannot drive, so it is
/// cut into the fixed instances the site's type roles use (OFL; see
/// assets/fonts).
const ANYBODY_REGULAR: &[u8] = include_bytes!("../assets/fonts/Anybody-Regular.ttf");
const ANYBODY_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Anybody-Medium.ttf");
/// 600 at width 112: buttons and headings.
const ANYBODY_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Anybody-SemiBold.ttf");
/// 700 at width 130: section titles, numbers.
const ANYBODY_BOLD: &[u8] = include_bytes!("../assets/fonts/Anybody-Bold.ttf");
/// 800 at width 150: the giant words.
const ANYBODY_DISPLAY: &[u8] = include_bytes!("../assets/fonts/Anybody-Display.ttf");
const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const MONO_MEDIUM: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");

/// Courier Prime, the screenwriting standard, ships with the app too: the
/// page is the one thing whose geometry has to be exact, and a fallback
/// monospace with different metrics would change where lines wrap.
const PAGE_REGULAR: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Regular.ttf");
const PAGE_BOLD: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Bold.ttf");

/// The name shown in Settings for the page font.
pub const PAGE_FONT_NAME: &str = "Courier Prime";

/// The heavier cuts, reachable from `FontId::new(size, family())`.
pub fn medium() -> FontFamily {
    FontFamily::Name("anybody-medium".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("anybody-semibold".into())
}
pub fn bold() -> FontFamily {
    FontFamily::Name("anybody-bold".into())
}
/// The giant display words: 800, extended.
pub fn display() -> FontFamily {
    FontFamily::Name("anybody-display".into())
}
/// The HUD: JetBrains Mono at 500.
pub fn hud() -> FontFamily {
    FontFamily::Name("mono-medium".into())
}
/// The screenplay page.
pub fn page() -> FontFamily {
    FontFamily::Name("page".into())
}
pub fn page_bold() -> FontFamily {
    FontFamily::Name("page-bold".into())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    for (key, bytes) in [
        ("Anybody", ANYBODY_REGULAR),
        ("AnybodyMedium", ANYBODY_MEDIUM),
        ("AnybodySemiBold", ANYBODY_SEMIBOLD),
        ("AnybodyBold", ANYBODY_BOLD),
        ("AnybodyDisplay", ANYBODY_DISPLAY),
        ("JetBrainsMono", MONO_REGULAR),
        ("JetBrainsMonoMedium", MONO_MEDIUM),
        ("CourierPrime", PAGE_REGULAR),
        ("CourierPrimeBold", PAGE_BOLD),
    ] {
        fonts
            .font_data
            .insert(key.to_string(), FontData::from_static(bytes));
    }

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Anybody".to_string());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "JetBrainsMono".to_string());

    let proportional = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mono = fonts.families.get(&FontFamily::Monospace).cloned().unwrap_or_default();
    for (family, key, rest) in [
        (medium(), "AnybodyMedium", &proportional),
        (semibold(), "AnybodySemiBold", &proportional),
        (bold(), "AnybodyBold", &proportional),
        (display(), "AnybodyDisplay", &proportional),
        (hud(), "JetBrainsMonoMedium", &mono),
    ] {
        // each cut falls back to its normal stack for glyphs it lacks
        let mut stack = vec![key.to_string()];
        stack.extend(rest.iter().cloned());
        fonts.families.insert(family, stack);
    }

    // the page: Courier Prime first, then the monospace stack for anything it
    // does not carry
    let mut page_stack = vec!["CourierPrime".to_string()];
    page_stack.extend(mono.iter().cloned());
    fonts.families.insert(page(), page_stack.clone());
    let mut bold_stack = vec!["CourierPrimeBold".to_string()];
    bold_stack.extend(page_stack);
    fonts.families.insert(page_bold(), bold_stack);

    ctx.set_fonts(fonts);
}

pub fn font(size: f32) -> egui::FontId {
    egui::FontId::proportional(size)
}
pub fn font_med(size: f32) -> egui::FontId {
    egui::FontId::new(size, medium())
}
pub fn font_semi(size: f32) -> egui::FontId {
    egui::FontId::new(size, semibold())
}
pub fn font_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, bold())
}
pub fn font_mono(size: f32) -> egui::FontId {
    egui::FontId::monospace(size)
}
/// A giant display word: 800, extended. Uppercase it.
pub fn font_display(size: f32) -> egui::FontId {
    egui::FontId::new(size, display())
}
/// A HUD readout: mono at 500. Uppercase it and track it.
pub fn font_hud(size: f32) -> egui::FontId {
    egui::FontId::new(size, hud())
}
pub fn font_page(size: f32) -> egui::FontId {
    egui::FontId::new(size, page())
}
pub fn font_page_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, page_bold())
}
