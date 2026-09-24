//! Visual identity: near-black greys, a single hot red accent, soft corners.

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Margin, Rounding, Stroke,
};

use crate::model::Element;

// ---------- palette ----------

pub const BG: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0C); // window backdrop
pub const RAIL: Color32 = Color32::from_rgb(0x10, 0x10, 0x13); // sidebar
pub const SURFACE: Color32 = Color32::from_rgb(0x15, 0x15, 0x19); // cards, top bar
pub const SURFACE_HI: Color32 = Color32::from_rgb(0x1D, 0x1D, 0x22); // hover
pub const PAGE: Color32 = Color32::from_rgb(0x12, 0x12, 0x16); // the "paper"
pub const LINE: Color32 = Color32::from_rgb(0x27, 0x27, 0x2E); // hairlines
pub const TEXT: Color32 = Color32::from_rgb(0xE9, 0xE7, 0xE4);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9A, 0x9A, 0xA4);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x62, 0x62, 0x6C);

pub const RED: Color32 = Color32::from_rgb(0xE1, 0x21, 0x3A);
pub const RED_HOT: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x52);
pub const RED_DEEP: Color32 = Color32::from_rgb(0x7A, 0x0D, 0x1D);
pub const RED_WASH: Color32 = Color32::from_rgb(0x2A, 0x0E, 0x14); // tinted fill

pub const R_CARD: f32 = 14.0;
pub const R_CTRL: f32 = 10.0;
pub const R_PILL: f32 = 999.0;

/// Accent colour per screenplay element, used by gutter tags and the palette.
pub fn element_color(e: Element) -> Color32 {
    match e {
        Element::SceneHeading => RED,
        Element::Shot => RED_HOT,
        Element::Character => Color32::from_rgb(0xF0, 0xA8, 0x3C),
        Element::Parenthetical => Color32::from_rgb(0x8E, 0x8E, 0x9C),
        Element::Dialogue => Color32::from_rgb(0x8F, 0xC7, 0xE8),
        Element::Transition => Color32::from_rgb(0xC1, 0x7F, 0xE0),
        Element::Action => TEXT_DIM,
    }
}

/// Colour of the text itself inside the page.
pub fn element_text_color(e: Element) -> Color32 {
    match e {
        Element::SceneHeading => Color32::from_rgb(0xFF, 0xD3, 0xD8),
        Element::Character => Color32::from_rgb(0xF3, 0xE7, 0xD2),
        Element::Parenthetical => TEXT_DIM,
        Element::Transition => Color32::from_rgb(0xE8, 0xD6, 0xF3),
        Element::Shot => Color32::from_rgb(0xFF, 0xC9, 0xCF),
        _ => TEXT,
    }
}

// ---------- style ----------

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(0x0C, 0x0C, 0x0F);
    visuals.faint_bg_color = SURFACE_HI;
    visuals.window_stroke = Stroke::new(1.0, LINE);
    visuals.window_rounding = Rounding::same(R_CARD);
    visuals.menu_rounding = Rounding::same(R_CTRL);
    visuals.selection.bg_fill = RED_DEEP.linear_multiply(1.4);
    visuals.selection.stroke = Stroke::new(1.0, RED_HOT);
    visuals.hyperlink_color = RED_HOT;

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.weak_bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    w.noninteractive.rounding = Rounding::same(R_CTRL);

    w.inactive.bg_fill = SURFACE_HI;
    w.inactive.weak_bg_fill = SURFACE;
    w.inactive.bg_stroke = Stroke::new(1.0, LINE);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.rounding = Rounding::same(R_CTRL);

    w.hovered.bg_fill = Color32::from_rgb(0x24, 0x24, 0x2B);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x24, 0x24, 0x2B);
    w.hovered.bg_stroke = Stroke::new(1.0, RED_DEEP);
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.rounding = Rounding::same(R_CTRL);
    w.hovered.expansion = 1.0;

    w.active.bg_fill = RED_WASH;
    w.active.weak_bg_fill = RED_WASH;
    w.active.bg_stroke = Stroke::new(1.0, RED);
    w.active.fg_stroke = Stroke::new(1.0, RED_HOT);
    w.active.rounding = Rounding::same(R_CTRL);

    w.open.bg_fill = SURFACE_HI;
    w.open.weak_bg_fill = SURFACE_HI;
    w.open.bg_stroke = Stroke::new(1.0, LINE);
    w.open.fg_stroke = Stroke::new(1.0, TEXT);
    w.open.rounding = Rounding::same(R_CTRL);

    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.window_margin = Margin::same(14.0);
        style.spacing.menu_margin = Margin::same(8.0);
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.floating = true;
        style.visuals.clip_rect_margin = 2.0;
    });
}

/// A rounded card frame.
pub fn card(fill: Color32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(Stroke::new(1.0, LINE))
        .rounding(Rounding::same(R_CARD))
        .inner_margin(Margin::same(14.0))
}

// ---------- fonts ----------

/// Fonts we would like for the screenplay page, best first. Courier Prime is
/// the modern screenwriting standard; the rest are graceful fallbacks that
/// ship on most Debian systems.
const MONO_CANDIDATES: &[&str] = &[
    "CourierPrime-Regular.ttf",
    "Courier Prime.ttf",
    "NimbusMonoPS-Regular.otf",
    "LiberationMono-Regular.ttf",
    "DejaVuSansMono.ttf",
    "JetBrainsMono-Regular.ttf",
];

/// Fonts for the chrome around the page.
const UI_CANDIDATES: &[&str] = &[
    "Inter-Regular.ttf",
    "InterVariable.ttf",
    "Cantarell-Regular.otf",
    "Ubuntu-R.ttf",
    "NotoSans-Regular.ttf",
    "DejaVuSans.ttf",
];

fn font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = vec![
        std::path::PathBuf::from("/usr/share/fonts"),
        std::path::PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
    }
    dirs
}

/// Shallow-ish recursive hunt for a font file by name.
fn find_font(candidates: &[&str]) -> Option<(String, Vec<u8>)> {
    let mut stack: Vec<std::path::PathBuf> = font_dirs();
    let mut seen = 0usize;
    let mut best: Option<(usize, std::path::PathBuf)> = None;

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > 20_000 {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Some(rank) = candidates.iter().position(|c| c.eq_ignore_ascii_case(name)) {
                let better = best.as_ref().map(|(r, _)| rank < *r).unwrap_or(true);
                if better {
                    best = Some((rank, path.clone()));
                }
                if rank == 0 {
                    break;
                }
            }
        }
    }

    let (_, path) = best?;
    let bytes = std::fs::read(&path).ok()?;
    let name = path.file_stem()?.to_str()?.to_string();
    Some((name, bytes))
}

/// Installs system fonts if we can find them. egui's built-ins (Hack for
/// monospace, Ubuntu-Light for UI) remain as the fallback chain, so a missing
/// font is a cosmetic downgrade and never a crash.
pub fn install_fonts(ctx: &egui::Context) -> Option<String> {
    let mut fonts = FontDefinitions::default();
    let mut mono_name = None;

    if let Some((name, bytes)) = find_font(MONO_CANDIDATES) {
        fonts
            .font_data
            .insert(name.clone(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, name.clone());
        mono_name = Some(name);
    }

    if let Some((name, bytes)) = find_font(UI_CANDIDATES) {
        fonts
            .font_data
            .insert(name.clone(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, name);
    }

    ctx.set_fonts(fonts);
    mono_name
}
