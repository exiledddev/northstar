//! Icons drawn as vectors rather than typed as glyphs.
//!
//! A 24-unit grid at roughly 1.75 stroke, in the shape of Lucide. Reaching for
//! font characters instead (`▸`, `❝`, `•••`) meant the icon only appeared if the
//! user happened to have a font carrying that codepoint — on a stock Debian
//! desktop several of them simply did not render. These are painted from
//! primitives, so they always show up and always take the theme colour.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

// A set, kept whole: an icon that has no caller today is still part of the
// alphabet the app is drawn in.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    ChevronRight,
    ChevronLeft,
    ChevronDown,
    ChevronUp,
    More,
    Search,
    Plus,
    Minus,
    File,
    Folder,
    FolderOpen,
    FolderPlus,
    Link,
    Unlink,
    WikiLink,
    Bold,
    Italic,
    Underline,
    Strike,
    Heading,
    List,
    Quote,
    Code,
    Table,
    Image,
    Trash,
    Pencil,
    Download,
    Print,
    Copy,
    Tag,
    Group,
    Ungroup,
    Fit,
    Scissors,
    Graph,
    Timeline,
    Star,
    StarFilled,
    Palette,
    Eraser,
    Settings,
    Sun,
    Moon,
    Check,
    Close,
    Info,
    Grip,
    Eye,
    Layers,
    ArrowRight,
    Sidebar,
    Refresh,
    Sparkle,
    Warning,
    Cube,
    // ---- drawn for Northstar, in the same hand ----
    Users,
    Grid,
    Upload,
    History,
    Star4,
}

/// Paint `icon` inside `rect`.
pub fn draw(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    draw_weighted(painter, rect, icon, color, 1.0);
}

/// The workhorse. `weight` scales the stroke, which is what the neon pass
/// rides on.
pub fn draw_weighted(
    painter: &egui::Painter,
    rect: Rect,
    icon: Icon,
    color: Color32,
    weight: f32,
) {
    let s = rect.width().min(rect.height());
    let w = (s * 0.085).max(1.2) * weight;
    let stroke = Stroke::new(w, color);
    let p = |x: f32, y: f32| -> Pos2 { egui::pos2(rect.left() + s * x, rect.top() + s * y) };
    let line = |pts: Vec<Pos2>| {
        painter.add(egui::Shape::line(pts, stroke));
    };
    let seg = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
    let boxed = |a: Pos2, b: Pos2, r: f32| {
        painter.rect_stroke(Rect::from_two_pos(a, b), egui::Rounding::same(s * r), stroke)
    };
    let star_pts = |cx: f32, cy: f32, r: f32| -> Vec<Pos2> {
        let mut v = Vec::with_capacity(10);
        for i in 0..10 {
            let a = std::f32::consts::TAU * (i as f32) / 10.0 - std::f32::consts::FRAC_PI_2;
            let rad = if i % 2 == 0 { r } else { r * 0.44 };
            v.push(p(cx + a.cos() * rad, cy + a.sin() * rad));
        }
        v
    };

    let fill = |pts: Vec<Pos2>, c: Color32| {
        painter.add(egui::Shape::convex_polygon(pts, c, Stroke::NONE));
    };
    // Darkening rather than painting a background colour keeps the cut-outs
    // legible whatever the glyph is sitting on.
    let punch = |c: Color32| -> Color32 {
        if 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32 > 130.0 {
            Color32::from_black_alpha(165)
        } else {
            Color32::from_white_alpha(185)
        }
    };
    let file_body = |fill: &dyn Fn(Vec<Pos2>, Color32), p: &dyn Fn(f32, f32) -> Pos2, c: Color32| {
        // the sheet, with its top-right corner turned back
        fill(vec![p(0.14, 0.08), p(0.60, 0.08), p(0.60, 0.92), p(0.14, 0.92)], c);
        fill(vec![p(0.60, 0.30), p(0.88, 0.30), p(0.88, 0.92), p(0.60, 0.92)], c);
        fill(vec![p(0.62, 0.08), p(0.88, 0.28), p(0.62, 0.28)], c);
    };
    let folder_open = |fill: &dyn Fn(Vec<Pos2>, Color32), p: &dyn Fn(f32, f32) -> Pos2, c: Color32| {
        // back plate with its tab
        fill(vec![p(0.04, 0.34), p(0.04, 0.14), p(0.34, 0.14), p(0.45, 0.30)], c);
        fill(vec![p(0.04, 0.28), p(0.88, 0.28), p(0.88, 0.46), p(0.04, 0.46)], c);
        // the spine that peeks out on the left, below the flap's slant
        fill(vec![p(0.04, 0.40), p(0.20, 0.40), p(0.10, 0.84), p(0.04, 0.84)], c);
        // the front flap, leaning right
        fill(vec![p(0.22, 0.50), p(0.98, 0.50), p(0.80, 0.90), p(0.04, 0.90)], c);
    };

    match icon {
        Icon::ChevronRight => line(vec![p(0.40, 0.24), p(0.64, 0.50), p(0.40, 0.76)]),
        Icon::ChevronLeft => line(vec![p(0.62, 0.24), p(0.38, 0.50), p(0.62, 0.76)]),
        Icon::ChevronDown => line(vec![p(0.24, 0.40), p(0.50, 0.64), p(0.76, 0.40)]),
        Icon::ChevronUp => line(vec![p(0.24, 0.62), p(0.50, 0.38), p(0.76, 0.62)]),

        Icon::More => {
            for x in [0.24, 0.50, 0.76] {
                painter.circle_filled(p(x, 0.5), s * 0.075, color);
            }
        }

        Icon::Search => {
            painter.circle_stroke(p(0.44, 0.44), s * 0.24, stroke);
            seg(p(0.62, 0.62), p(0.80, 0.80));
        }

        Icon::Plus => {
            seg(p(0.5, 0.22), p(0.5, 0.78));
            seg(p(0.22, 0.5), p(0.78, 0.5));
        }

        Icon::Minus => {
            seg(p(0.22, 0.5), p(0.78, 0.5));
        }

        // ---- the filled family ------------------------------------------
        // Traced from the reference art: solid shapes, not hairline outlines,
        // which is what makes them read at 13 pixels.
        Icon::File => {
            file_body(&fill, &p, color);
            // three rules, punched through by darkening rather than by
            // painting the surface colour, so the glyph works on any ground
            let ink = punch(color);
            for (y, x1) in [(0.42_f32, 0.74_f32), (0.58, 0.74), (0.74, 0.50)] {
                fill(
                    vec![p(0.26, y - 0.045), p(x1, y - 0.045), p(x1, y + 0.045), p(0.26, y + 0.045)],
                    ink,
                );
            }
        }

        Icon::Folder => {
            // shut: the back plate with its tab, and the front face over it
            fill(
                vec![p(0.06, 0.30), p(0.06, 0.16), p(0.36, 0.16), p(0.47, 0.30)],
                color,
            );
            fill(
                vec![p(0.06, 0.28), p(0.94, 0.28), p(0.94, 0.86), p(0.06, 0.86)],
                color,
            );
        }

        Icon::FolderOpen => {
            folder_open(&fill, &p, color);
        }

        Icon::FolderPlus => {
            folder_open(&fill, &p, color);
            let ink = punch(color);
            fill(
                vec![p(0.44, 0.56), p(0.58, 0.56), p(0.58, 0.80), p(0.44, 0.80)],
                ink,
            );
            fill(
                vec![p(0.34, 0.62), p(0.68, 0.62), p(0.68, 0.74), p(0.34, 0.74)],
                ink,
            );
        }

        Icon::Link => {
            line(vec![p(0.42, 0.58), p(0.30, 0.70), p(0.20, 0.60), p(0.32, 0.48)]);
            line(vec![p(0.58, 0.42), p(0.70, 0.30), p(0.80, 0.40), p(0.68, 0.52)]);
            seg(p(0.40, 0.60), p(0.60, 0.40));
        }

        Icon::Unlink => {
            line(vec![p(0.40, 0.60), p(0.28, 0.72), p(0.18, 0.62), p(0.30, 0.50)]);
            line(vec![p(0.60, 0.40), p(0.72, 0.28), p(0.82, 0.38), p(0.70, 0.50)]);
            seg(p(0.20, 0.20), p(0.80, 0.80));
        }

        Icon::WikiLink => {
            for (x, dir) in [(0.22, 1.0f32), (0.36, 1.0), (0.64, -1.0), (0.78, -1.0)] {
                line(vec![
                    p(x + 0.08 * dir, 0.22),
                    p(x, 0.22),
                    p(x, 0.78),
                    p(x + 0.08 * dir, 0.78),
                ]);
            }
        }

        Icon::Bold => {
            line(vec![p(0.30, 0.18), p(0.56, 0.18), p(0.66, 0.32), p(0.56, 0.48), p(0.30, 0.48)]);
            line(vec![p(0.30, 0.48), p(0.60, 0.48), p(0.72, 0.64), p(0.60, 0.82), p(0.30, 0.82)]);
            seg(p(0.30, 0.18), p(0.30, 0.82));
        }

        Icon::Italic => {
            seg(p(0.40, 0.20), p(0.72, 0.20));
            seg(p(0.28, 0.80), p(0.60, 0.80));
            seg(p(0.56, 0.20), p(0.44, 0.80));
        }

        Icon::Underline => {
            line(vec![p(0.28, 0.18), p(0.28, 0.50)]);
            line(vec![p(0.72, 0.18), p(0.72, 0.50)]);
            painter.add(egui::Shape::line(
                vec![p(0.28, 0.50), p(0.34, 0.64), p(0.50, 0.68), p(0.66, 0.64), p(0.72, 0.50)],
                stroke,
            ));
            seg(p(0.24, 0.84), p(0.76, 0.84));
        }

        Icon::Strike => {
            line(vec![p(0.68, 0.26), p(0.60, 0.18), p(0.40, 0.18), p(0.32, 0.28), p(0.40, 0.40), p(0.58, 0.46)]);
            line(vec![p(0.34, 0.62), p(0.42, 0.76), p(0.62, 0.80), p(0.70, 0.70)]);
            seg(p(0.18, 0.52), p(0.82, 0.52));
        }

        Icon::Heading => {
            seg(p(0.26, 0.20), p(0.26, 0.80));
            seg(p(0.62, 0.20), p(0.62, 0.80));
            seg(p(0.26, 0.50), p(0.62, 0.50));
            seg(p(0.74, 0.52), p(0.74, 0.80));
        }

        Icon::List => {
            for y in [0.28, 0.50, 0.72] {
                painter.circle_filled(p(0.22, y), s * 0.055, color);
                seg(p(0.38, y), p(0.80, y));
            }
        }

        Icon::Quote => {
            for x in [0.24, 0.52] {
                line(vec![
                    p(x + 0.18, 0.28),
                    p(x, 0.28),
                    p(x, 0.50),
                    p(x + 0.18, 0.50),
                    p(x + 0.18, 0.62),
                    p(x + 0.06, 0.72),
                ]);
            }
        }

        Icon::Code => {
            line(vec![p(0.38, 0.26), p(0.18, 0.50), p(0.38, 0.74)]);
            line(vec![p(0.62, 0.26), p(0.82, 0.50), p(0.62, 0.74)]);
        }

        Icon::Table => {
            boxed(p(0.16, 0.22), p(0.84, 0.78), 0.06);
            seg(p(0.16, 0.40), p(0.84, 0.40));
            seg(p(0.16, 0.59), p(0.84, 0.59));
            seg(p(0.44, 0.22), p(0.44, 0.78));
        }

        Icon::Image => {
            boxed(p(0.16, 0.22), p(0.84, 0.78), 0.08);
            painter.circle_filled(p(0.36, 0.40), s * 0.06, color);
            line(vec![p(0.22, 0.70), p(0.44, 0.50), p(0.62, 0.66), p(0.72, 0.58), p(0.84, 0.70)]);
        }

        Icon::Trash => {
            seg(p(0.16, 0.30), p(0.84, 0.30));
            line(vec![p(0.36, 0.30), p(0.36, 0.20), p(0.64, 0.20), p(0.64, 0.30)]);
            line(vec![p(0.24, 0.30), p(0.28, 0.84), p(0.72, 0.84), p(0.76, 0.30)]);
            seg(p(0.42, 0.44), p(0.44, 0.72));
            seg(p(0.58, 0.44), p(0.56, 0.72));
        }

        Icon::Pencil => {
            line(vec![
                p(0.24, 0.76),
                p(0.62, 0.20),
                p(0.80, 0.34),
                p(0.42, 0.86),
                p(0.20, 0.88),
                p(0.24, 0.76),
            ]);
            seg(p(0.56, 0.28), p(0.74, 0.42));
        }

        Icon::Download => {
            seg(p(0.5, 0.16), p(0.5, 0.62));
            line(vec![p(0.32, 0.46), p(0.5, 0.64), p(0.68, 0.46)]);
            line(vec![p(0.20, 0.72), p(0.20, 0.84), p(0.80, 0.84), p(0.80, 0.72)]);
        }

        Icon::Print => {
            line(vec![p(0.30, 0.34), p(0.30, 0.16), p(0.70, 0.16), p(0.70, 0.34)]);
            line(vec![
                p(0.30, 0.70),
                p(0.18, 0.70),
                p(0.18, 0.40),
                p(0.82, 0.40),
                p(0.82, 0.70),
                p(0.70, 0.70),
            ]);
            boxed(p(0.30, 0.58), p(0.70, 0.88), 0.04);
        }

        Icon::Copy => {
            boxed(p(0.34, 0.34), p(0.84, 0.86), 0.07);
            line(vec![p(0.24, 0.66), p(0.16, 0.66), p(0.16, 0.14), p(0.66, 0.14), p(0.66, 0.24)]);
        }

        Icon::Group => {
            boxed(p(0.12, 0.18), p(0.48, 0.50), 0.05);
            boxed(p(0.52, 0.50), p(0.88, 0.82), 0.05);
            seg(p(0.48, 0.34), p(0.70, 0.34));
            seg(p(0.70, 0.34), p(0.70, 0.50));
        }

        Icon::Ungroup => {
            boxed(p(0.12, 0.18), p(0.44, 0.46), 0.05);
            boxed(p(0.56, 0.54), p(0.88, 0.82), 0.05);
            seg(p(0.52, 0.30), p(0.66, 0.30));
            seg(p(0.34, 0.60), p(0.48, 0.60));
        }

        Icon::Fit => {
            line(vec![p(0.16, 0.36), p(0.16, 0.16), p(0.36, 0.16)]);
            line(vec![p(0.64, 0.16), p(0.84, 0.16), p(0.84, 0.36)]);
            line(vec![p(0.84, 0.64), p(0.84, 0.84), p(0.64, 0.84)]);
            line(vec![p(0.36, 0.84), p(0.16, 0.84), p(0.16, 0.64)]);
        }

        Icon::Scissors => {
            painter.circle_stroke(p(0.28, 0.74), s * 0.13, stroke);
            painter.circle_stroke(p(0.72, 0.74), s * 0.13, stroke);
            seg(p(0.34, 0.64), p(0.78, 0.18));
            seg(p(0.66, 0.64), p(0.22, 0.18));
        }

        Icon::Graph => {
            painter.circle_stroke(p(0.24, 0.28), s * 0.12, stroke);
            painter.circle_stroke(p(0.76, 0.34), s * 0.12, stroke);
            painter.circle_stroke(p(0.46, 0.78), s * 0.12, stroke);
            seg(p(0.35, 0.31), p(0.64, 0.33));
            seg(p(0.29, 0.39), p(0.42, 0.66));
            seg(p(0.70, 0.45), p(0.53, 0.68));
        }

        Icon::Timeline => {
            seg(p(0.26, 0.14), p(0.26, 0.86));
            for y in [0.26, 0.52, 0.78] {
                painter.circle_filled(p(0.26, y), s * 0.075, color);
                seg(p(0.42, y), p(0.84, y));
            }
        }

        Icon::Tag => {
            line(vec![
                p(0.18, 0.46),
                p(0.46, 0.18),
                p(0.84, 0.18),
                p(0.84, 0.56),
                p(0.56, 0.84),
                p(0.18, 0.46),
            ]);
            painter.circle_filled(p(0.68, 0.34), s * 0.06, color);
        }

        Icon::Star => {
            let pts = star_pts(0.5, 0.52, 0.36);
            let mut closed = pts.clone();
            closed.push(pts[0]);
            line(closed);
        }

        Icon::StarFilled => {
            let pts = star_pts(0.5, 0.52, 0.36);
            // an inner pentagon plus five spikes, so every piece stays convex
            let pent: Vec<Pos2> = (1..10).step_by(2).map(|i| pts[i]).collect();
            painter.add(egui::Shape::convex_polygon(pent, color, Stroke::NONE));
            for i in (0..10).step_by(2) {
                painter.add(egui::Shape::convex_polygon(
                    vec![pts[(i + 9) % 10], pts[i], pts[(i + 1) % 10]],
                    color,
                    Stroke::NONE,
                ));
            }
        }

        Icon::Palette => {
            painter.circle_stroke(p(0.5, 0.5), s * 0.34, stroke);
            for (x, y) in [(0.36, 0.32), (0.62, 0.34), (0.70, 0.56)] {
                painter.circle_filled(p(x, y), s * 0.065, color);
            }
            line(vec![p(0.40, 0.74), p(0.52, 0.80), p(0.66, 0.74)]);
        }

        Icon::Eraser => {
            line(vec![p(0.20, 0.72), p(0.54, 0.24), p(0.80, 0.44), p(0.46, 0.84), p(0.20, 0.72)]);
            seg(p(0.34, 0.52), p(0.60, 0.70));
            seg(p(0.46, 0.84), p(0.84, 0.84));
        }

        Icon::Settings => {
            for (y, x) in [(0.26, 0.62), (0.50, 0.36), (0.74, 0.68)] {
                seg(p(0.14, y), p(0.86, y));
                painter.circle_filled(p(x, y), s * 0.10, color);
                painter.circle_stroke(p(x, y), s * 0.10, Stroke::new(w, color));
            }
        }

        Icon::Sun => {
            painter.circle_stroke(p(0.5, 0.5), s * 0.20, stroke);
            for i in 0..8 {
                let a = std::f32::consts::TAU * i as f32 / 8.0;
                seg(
                    p(0.5 + a.cos() * 0.30, 0.5 + a.sin() * 0.30),
                    p(0.5 + a.cos() * 0.40, 0.5 + a.sin() * 0.40),
                );
            }
        }

        Icon::Moon => {
            line(vec![
                p(0.66, 0.16),
                p(0.44, 0.20),
                p(0.28, 0.36),
                p(0.24, 0.60),
                p(0.38, 0.80),
                p(0.62, 0.86),
                p(0.80, 0.74),
                p(0.56, 0.68),
                p(0.44, 0.50),
                p(0.50, 0.28),
                p(0.66, 0.16),
            ]);
        }

        Icon::Check => line(vec![p(0.22, 0.52), p(0.42, 0.72), p(0.78, 0.28)]),

        Icon::Close => {
            seg(p(0.26, 0.26), p(0.74, 0.74));
            seg(p(0.74, 0.26), p(0.26, 0.74));
        }

        Icon::Info => {
            painter.circle_stroke(p(0.5, 0.5), s * 0.34, stroke);
            painter.circle_filled(p(0.5, 0.32), s * 0.055, color);
            seg(p(0.5, 0.44), p(0.5, 0.70));
        }

        Icon::Grip => {
            for y in [0.30, 0.50, 0.70] {
                for x in [0.38, 0.62] {
                    painter.circle_filled(p(x, y), s * 0.065, color);
                }
            }
        }

        Icon::Eye => {
            line(vec![
                p(0.12, 0.50),
                p(0.30, 0.30),
                p(0.50, 0.24),
                p(0.70, 0.30),
                p(0.88, 0.50),
                p(0.70, 0.70),
                p(0.50, 0.76),
                p(0.30, 0.70),
                p(0.12, 0.50),
            ]);
            painter.circle_stroke(p(0.5, 0.50), s * 0.13, stroke);
        }

        Icon::Layers => {
            line(vec![p(0.5, 0.14), p(0.88, 0.36), p(0.5, 0.58), p(0.12, 0.36), p(0.5, 0.14)]);
            line(vec![p(0.18, 0.52), p(0.5, 0.70), p(0.82, 0.52)]);
            line(vec![p(0.18, 0.66), p(0.5, 0.84), p(0.82, 0.66)]);
        }

        Icon::ArrowRight => {
            seg(p(0.18, 0.50), p(0.78, 0.50));
            line(vec![p(0.56, 0.30), p(0.80, 0.50), p(0.56, 0.70)]);
        }

        Icon::Sidebar => {
            boxed(p(0.14, 0.20), p(0.86, 0.80), 0.07);
            seg(p(0.42, 0.20), p(0.42, 0.80));
        }

        Icon::Refresh => {
            painter.circle_stroke(p(0.5, 0.5), s * 0.30, Stroke::new(w, color));
            painter.rect_filled(
                Rect::from_two_pos(p(0.52, 0.06), p(0.98, 0.36)),
                egui::Rounding::ZERO,
                Color32::TRANSPARENT,
            );
            line(vec![p(0.62, 0.14), p(0.80, 0.22), p(0.72, 0.40)]);
        }

        Icon::Sparkle => {
            line(vec![
                p(0.50, 0.12),
                p(0.60, 0.40),
                p(0.88, 0.50),
                p(0.60, 0.60),
                p(0.50, 0.88),
                p(0.40, 0.60),
                p(0.12, 0.50),
                p(0.40, 0.40),
                p(0.50, 0.12),
            ]);
        }

        Icon::Warning => {
            line(vec![p(0.50, 0.16), p(0.90, 0.82), p(0.10, 0.82), p(0.50, 0.16)]);
            seg(p(0.50, 0.40), p(0.50, 0.60));
            painter.circle_filled(p(0.50, 0.71), s * 0.05, color);
        }

        Icon::Cube => {
            line(vec![
                p(0.50, 0.12),
                p(0.86, 0.32),
                p(0.86, 0.68),
                p(0.50, 0.88),
                p(0.14, 0.68),
                p(0.14, 0.32),
                p(0.50, 0.12),
            ]);
            line(vec![p(0.14, 0.32), p(0.50, 0.52), p(0.86, 0.32)]);
            seg(p(0.50, 0.52), p(0.50, 0.88));
        }

        Icon::Users => {
            painter.circle_stroke(p(0.38, 0.34), s * 0.14, stroke);
            line(vec![
                p(0.14, 0.82),
                p(0.16, 0.70),
                p(0.24, 0.60),
                p(0.38, 0.56),
                p(0.52, 0.60),
                p(0.60, 0.70),
                p(0.62, 0.82),
            ]);
            line(vec![p(0.62, 0.22), p(0.72, 0.24), p(0.76, 0.34), p(0.72, 0.44), p(0.62, 0.46)]);
            line(vec![p(0.70, 0.58), p(0.80, 0.64), p(0.86, 0.82)]);
        }

        Icon::Grid => {
            boxed(p(0.16, 0.16), p(0.46, 0.46), 0.06);
            boxed(p(0.54, 0.16), p(0.84, 0.46), 0.06);
            boxed(p(0.16, 0.54), p(0.46, 0.84), 0.06);
            boxed(p(0.54, 0.54), p(0.84, 0.84), 0.06);
        }

        Icon::Upload => {
            seg(p(0.5, 0.62), p(0.5, 0.16));
            line(vec![p(0.32, 0.34), p(0.5, 0.16), p(0.68, 0.34)]);
            line(vec![p(0.20, 0.72), p(0.20, 0.84), p(0.80, 0.84), p(0.80, 0.72)]);
        }

        Icon::History => {
            // a clock face open at the top left, with the arrow that winds it back
            let c = (0.52_f32, 0.52_f32);
            let r = 0.32_f32;
            let mut arc = Vec::new();
            for i in 0..=20 {
                let a = -2.4 + (std::f32::consts::TAU - 0.9) * i as f32 / 20.0;
                arc.push(p(c.0 + a.cos() * r, c.1 + a.sin() * r));
            }
            line(arc);
            line(vec![p(0.14, 0.22), p(0.18, 0.36), p(0.32, 0.32)]);
            line(vec![p(0.52, 0.34), p(0.52, 0.54), p(0.66, 0.62)]);
        }

        Icon::Star4 => {
            line(vec![
                p(0.50, 0.10),
                p(0.59, 0.41),
                p(0.90, 0.50),
                p(0.59, 0.59),
                p(0.50, 0.90),
                p(0.41, 0.59),
                p(0.10, 0.50),
                p(0.41, 0.41),
                p(0.50, 0.10),
            ]);
        }
    }
}

/// Allocate space and paint an icon into it, for use inline in a layout.
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    draw(ui.painter(), rect, icon, color);
    resp
}
