//! Pages: the script exactly as it will print.
//!
//! Drawn from the same composition pass the PDF is written from, so a line
//! that ends a page here ends the page on paper. Each sheet is a solid
//! surface laid on the island — something you read — with the island's own
//! soft shadow, and two sheets sit side by side when there is room. Click a
//! line to go and write it.

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use crate::export::{compose, paginate, Line};
use crate::model::{Document, Element, LINES_PER_PAGE, PAGE_COLS};
use crate::theme::{self, pal};

/// US letter, 8.5 by 11.
const ASPECT: f32 = 11.0 / 8.5;
const GAP: f32 = 22.0;

#[derive(Default)]
pub struct PagesState {
    /// Where each sheet was drawn, title page first, for the tests.
    pub last_rects: Vec<Rect>,
}

/// Returns the block to go and edit, if a line was clicked.
pub fn show(
    ui: &mut egui::Ui,
    doc: &Document,
    st: &mut PagesState,
    scene_numbers: bool,
) -> Option<u64> {
    let p = pal();
    let pages = paginate(&compose(doc));
    let avail = ui.available_width() - 2.0 * GAP;
    let two_up = avail >= 2.0 * 460.0 + GAP;
    let page_w = if two_up {
        ((avail - GAP) * 0.5).min(620.0)
    } else {
        avail.clamp(240.0, 600.0)
    };
    let page_h = page_w * ASPECT;
    let cols = if two_up { 2 } else { 1 };
    let grid_w = cols as f32 * page_w + (cols as f32 - 1.0) * GAP;
    let left = ui.min_rect().left() + ((ui.available_width() - grid_w) * 0.5).max(GAP);
    let top = ui.cursor().top() + 26.0;

    // the title page, then every page of the body
    let sheets = 1 + pages.len();
    let rows = sheets.div_ceil(cols);
    let (_, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 26.0 + rows as f32 * (page_h + GAP) + 60.0),
        Sense::hover(),
    );

    // Everything is measured in the page's own units: an inch is page_w / 8.5.
    let inch = page_w / 8.5;
    let line_h = inch / 6.0;
    let font_px = {
        // size the page font so sixty characters fill exactly six inches
        let probe = theme::font_page(10.0);
        let w10 = ui.fonts(|f| f.glyph_width(&probe, 'M')).max(0.1);
        (6.0 * inch / PAGE_COLS as f32) / w10 * 10.0
    };
    let char_w = 6.0 * inch / PAGE_COLS as f32;
    let font = theme::font_page(font_px);
    let bold = theme::font_page_bold(font_px);
    let paper = if p.dark { p.raised } else { p.solid };
    let ink = if p.dark { theme::mix(p.text, p.text_dim, 0.1) } else { p.text };

    st.last_rects.clear();
    let clip = ui.clip_rect();
    let mut jump = None;

    for s in 0..sheets {
        let (r, c) = (s / cols, s % cols);
        let sheet = Rect::from_min_size(
            Pos2::new(left + c as f32 * (page_w + GAP), top + r as f32 * (page_h + GAP)),
            Vec2::new(page_w, page_h),
        );
        st.last_rects.push(sheet);
        if !sheet.intersects(clip) {
            continue;
        }
        let painter = ui.painter();
        // the island's own shadow: low and soft
        painter.add(
            egui::epaint::Shadow {
                offset: Vec2::new(0.0, 3.0),
                blur: 16.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(if p.dark { 90 } else { 30 }),
            }
            .as_shape(sheet, egui::Rounding::same(theme::R_SM)),
        );
        painter.rect_filled(sheet, egui::Rounding::same(theme::R_SM), paper);

        let text_left = sheet.left() + 1.5 * inch;
        let body_top = sheet.top() + inch;

        if s == 0 {
            title_page(painter, doc, sheet, inch, line_h, char_w, &font, &bold, ink);
            continue;
        }
        let n = s; // body page number
        if n > 1 {
            let num = format!("{n}.");
            painter.text(
                Pos2::new(text_left + PAGE_COLS as f32 * char_w, sheet.top() + 0.5 * inch + line_h * 0.5),
                egui::Align2::RIGHT_CENTER,
                num,
                font.clone(),
                ink,
            );
        }
        let page: &Vec<Line> = &pages[n - 1];
        for (row, line) in page.iter().enumerate().take(LINES_PER_PAGE + 5) {
            if line.text.trim().is_empty() {
                continue;
            }
            let y = body_top + row as f32 * line_h;
            let x = text_left + line.indent as f32 * char_w;
            let is_heading = matches!(line.element, Some(Element::SceneHeading));
            let color = match line.element {
                Some(Element::Parenthetical) | Some(Element::Transition) => {
                    theme::mix(ink, p.text_dim, 0.4)
                }
                _ => ink,
            };
            painter.text(
                Pos2::new(x, y),
                egui::Align2::LEFT_TOP,
                &line.text,
                if is_heading { bold.clone() } else { font.clone() },
                color,
            );
            if scene_numbers {
                if let Some(num) = line.scene {
                    let label = format!("{num}");
                    painter.text(
                        Pos2::new(text_left - 0.5 * inch, y),
                        egui::Align2::RIGHT_TOP,
                        &label,
                        bold.clone(),
                        ink,
                    );
                    painter.text(
                        Pos2::new(text_left + PAGE_COLS as f32 * char_w + 0.2 * inch, y),
                        egui::Align2::LEFT_TOP,
                        &label,
                        bold.clone(),
                        ink,
                    );
                }
            }
            // a line is a way back into the script
            if let Some(block) = line.block {
                let hit = Rect::from_min_size(
                    Pos2::new(x, y),
                    Vec2::new(line.text.chars().count() as f32 * char_w, line_h),
                );
                let resp = ui.interact(hit, egui::Id::new(("ns-page-line", n, row)), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(
                        hit.expand2(Vec2::new(3.0, 0.0)),
                        egui::Rounding::same(3.0),
                        theme::wash(p.primary, 30),
                    );
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    jump = Some(block);
                }
            }
        }
    }
    jump
}

#[allow(clippy::too_many_arguments)]
fn title_page(
    painter: &egui::Painter,
    doc: &Document,
    sheet: Rect,
    inch: f32,
    line_h: f32,
    char_w: f32,
    font: &egui::FontId,
    bold: &egui::FontId,
    ink: egui::Color32,
) {
    let text_left = sheet.left() + 1.5 * inch;
    let body_top = sheet.top() + inch;
    let centre = text_left + PAGE_COLS as f32 * char_w * 0.5;
    let put = |text: &str, line: f32, f: &egui::FontId| {
        painter.text(
            Pos2::new(centre, body_top + line * line_h),
            egui::Align2::CENTER_TOP,
            text,
            f.clone(),
            ink,
        );
    };
    let title = if doc.meta.title.trim().is_empty() {
        "UNTITLED SCRIPT".to_string()
    } else {
        doc.meta.title.trim().to_uppercase()
    };
    // the same lines the PDF puts them on
    put(&title, 19.0, bold);
    if !doc.meta.author.trim().is_empty() {
        put("written by", 23.0, font);
        put(doc.meta.author.trim(), 25.0, font);
    }
    if !doc.meta.draft.trim().is_empty() {
        put(doc.meta.draft.trim(), 43.0, font);
    }
    if !doc.meta.contact.trim().is_empty() {
        painter.text(
            Pos2::new(text_left, body_top + 47.0 * line_h),
            egui::Align2::LEFT_TOP,
            doc.meta.contact.trim(),
            font.clone(),
            ink,
        );
    }
}
