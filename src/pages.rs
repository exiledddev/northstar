//! Reading mode: the script exactly as it will print, one page at a time.
//!
//! Drawn from the same composition pass the PDF is written from, so a line
//! that ends a page here ends the page on paper. None of the editor's
//! scaffolding comes through — no element bands, no tags, no stars — only
//! the typeset page (and, if asked for, each speaker's colour).
//!
//! It reads like a book, not a contact sheet: a single page, sized so the
//! whole of it is in view, turned with the arrow keys, Page Up and Down, the
//! scroll wheel or the control at the foot. Click a line to go and write it.

use eframe::egui::{self, Key, Modifiers, Pos2, Rect, Sense, Vec2};
use std::time::Instant;

use crate::anim;
use crate::export::{compose, paginate, Line};
use crate::icons::Icon;
use crate::model::{base_character, Document, Element, LINES_PER_PAGE, PAGE_COLS};
use crate::theme::{self, pal};
use crate::ui;

/// US letter, 8.5 by 11.
const ASPECT: f32 = 11.0 / 8.5;
/// Room under the page for the control that turns it.
const FOOT: f32 = 58.0;
/// How far the wheel has to travel to turn a page.
const WHEEL_STEP: f32 = 90.0;

pub struct PagesState {
    /// The sheet on show: 0 is the title page, 1 the first page of the body.
    pub current: usize,
    /// Title page plus body pages, as of the last frame.
    pub sheets: usize,
    /// Open on the page that holds this block, next frame.
    pub show_block: Option<u64>,
    /// Where the sheet on show was drawn.
    pub sheet_rect: Rect,
    /// When the page last turned, and which way (+1 forward).
    turned: Option<(Instant, f32)>,
    wheel: f32,
}

impl Default for PagesState {
    fn default() -> Self {
        PagesState {
            current: 0,
            sheets: 1,
            show_block: None,
            sheet_rect: Rect::NOTHING,
            turned: None,
            wheel: 0.0,
        }
    }
}

impl PagesState {
    pub fn turn_to(&mut self, sheet: usize) {
        let sheet = sheet.min(self.sheets.saturating_sub(1));
        if sheet != self.current {
            let dir = if sheet > self.current { 1.0 } else { -1.0 };
            self.current = sheet;
            self.turned = Some((Instant::now(), dir));
        }
    }
}

/// Returns the block to go and edit, if a line was clicked.
pub fn show(
    ui: &mut egui::Ui,
    doc: &Document,
    st: &mut PagesState,
    scene_numbers: bool,
    voices: Option<&std::collections::HashMap<String, egui::Color32>>,
) -> Option<u64> {
    let p = pal();
    let pages = paginate(&compose(doc));
    st.sheets = 1 + pages.len();
    if st.current >= st.sheets {
        st.current = st.sheets - 1;
    }
    // open where the writing was
    if let Some(block) = st.show_block.take() {
        if let Some(k) = pages
            .iter()
            .position(|pg| pg.iter().any(|l| l.block == Some(block)))
        {
            st.current = k + 1;
            st.turned = None;
        }
    }

    // who is speaking on every line, worked out once across all the pages so
    // a speech that runs over a page break keeps its colour
    let mut speaking: Vec<Vec<Option<egui::Color32>>> = Vec::with_capacity(pages.len());
    {
        let mut who: Option<String> = None;
        for page in &pages {
            let mut row = Vec::with_capacity(page.len());
            for line in page {
                match line.element {
                    Some(Element::Character) => {
                        let n = base_character(&line.text);
                        if !n.is_empty() {
                            who = Some(n);
                        }
                    }
                    Some(Element::Dialogue) | Some(Element::Parenthetical) | None => {}
                    _ => who = None,
                }
                row.push(who.as_ref().and_then(|n| voices.and_then(|m| m.get(n))).copied());
            }
            speaking.push(row);
        }
    }

    // ---- turning the page ----
    let area = ui.max_rect();
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if !typing {
        let (fwd, back, first, last) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowRight)
                    || i.consume_key(Modifiers::NONE, Key::PageDown)
                    || i.consume_key(Modifiers::NONE, Key::ArrowDown)
                    || i.consume_key(Modifiers::NONE, Key::Space),
                i.consume_key(Modifiers::NONE, Key::ArrowLeft)
                    || i.consume_key(Modifiers::NONE, Key::PageUp)
                    || i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Home),
                i.consume_key(Modifiers::NONE, Key::End),
            )
        });
        if fwd {
            st.turn_to(st.current + 1);
        }
        if back && st.current > 0 {
            st.turn_to(st.current - 1);
        }
        if first {
            st.turn_to(0);
        }
        if last {
            st.turn_to(st.sheets - 1);
        }
    }
    if ui.rect_contains_pointer(area) {
        let dy = ui.input(|i| i.raw_scroll_delta.y);
        if dy != 0.0 {
            st.wheel += dy;
            if st.wheel <= -WHEEL_STEP {
                st.wheel = 0.0;
                st.turn_to(st.current + 1);
            } else if st.wheel >= WHEEL_STEP {
                st.wheel = 0.0;
                if st.current > 0 {
                    st.turn_to(st.current - 1);
                }
            }
        }
    }

    // ---- the sheet: the whole of it in view ----
    let room = Rect::from_min_max(
        area.min + Vec2::new(24.0, 24.0),
        Pos2::new(area.right() - 24.0, area.bottom() - FOOT),
    );
    let page_h = room.height().min(room.width() * ASPECT).max(120.0);
    let page_w = page_h / ASPECT;
    let rest = Rect::from_center_size(room.center(), Vec2::new(page_w, page_h));

    // a turn slides the new page a little way in from the side it came from
    let (shift, fade) = match st.turned {
        Some((at, dir)) if anim::enabled() => {
            let t = (at.elapsed().as_secs_f32() / 0.30).clamp(0.0, 1.0);
            if t >= 1.0 {
                st.turned = None;
            } else {
                anim::keep_going(ui.ctx());
            }
            let e = anim::ease_out_expo(t);
            ((1.0 - e) * 26.0 * dir, 0.25 + 0.75 * e)
        }
        _ => (0.0, 1.0),
    };
    let sheet = rest.translate(Vec2::new(shift, 0.0));
    st.sheet_rect = rest;

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
    let a = |c: egui::Color32| theme::wash(c, (c.a() as f32 * fade) as u8);

    let mut jump = None;
    {
        let painter = ui.painter();
        theme::lift_shadow(painter, sheet, theme::R_SM, fade);
        painter.rect_filled(sheet, egui::Rounding::same(theme::R_SM), a(paper));
    }
    let text_left = sheet.left() + 1.5 * inch;
    let body_top = sheet.top() + inch;

    if st.current == 0 {
        title_page(ui.painter(), doc, sheet, inch, line_h, char_w, &font, &bold, a(ink));
    } else {
        let n = st.current;
        if n > 1 {
            ui.painter().text(
                Pos2::new(text_left + PAGE_COLS as f32 * char_w, sheet.top() + 0.5 * inch + line_h * 0.5),
                egui::Align2::RIGHT_CENTER,
                format!("{n}."),
                font.clone(),
                a(ink),
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
            let color = match (speaking[n - 1][row], line.element) {
                (Some(v), _) => v,
                (None, Some(Element::Parenthetical)) | (None, Some(Element::Transition)) => {
                    theme::mix(ink, p.text_dim, 0.4)
                }
                _ => ink,
            };
            // a line is a way back into the script
            if let Some(block) = line.block {
                let hit = Rect::from_min_size(
                    Pos2::new(x, y),
                    Vec2::new(line.text.chars().count() as f32 * char_w, line_h),
                );
                let resp = ui.interact(hit, egui::Id::new(("ns-page-line", n, row)), Sense::click());
                let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
                if hot > 0.01 {
                    ui.painter().rect_filled(
                        hit.expand2(Vec2::new(3.0, 0.0)),
                        egui::Rounding::same(3.0),
                        theme::wash(p.primary, (28.0 * hot) as u8),
                    );
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.on_hover_text("Click to write this line").clicked() {
                    jump = Some(block);
                }
            }
            ui.painter().text(
                Pos2::new(x, y),
                egui::Align2::LEFT_TOP,
                &line.text,
                if is_heading { bold.clone() } else { font.clone() },
                a(color),
            );
            if scene_numbers {
                if let Some(num) = line.scene {
                    let label = format!("{num}");
                    ui.painter().text(
                        Pos2::new(text_left - 0.5 * inch, y),
                        egui::Align2::RIGHT_TOP,
                        &label,
                        bold.clone(),
                        a(ink),
                    );
                    ui.painter().text(
                        Pos2::new(text_left + PAGE_COLS as f32 * char_w + 0.2 * inch, y),
                        egui::Align2::LEFT_TOP,
                        &label,
                        bold.clone(),
                        a(ink),
                    );
                }
            }
        }
    }

    // ---- the control at the foot: previous, where you are, next ----
    let label = if st.current == 0 {
        format!("Title page  ·  {} pages", st.sheets - 1)
    } else {
        format!("Page {} of {}", st.current, st.sheets - 1)
    };
    let text_w = ui
        .painter()
        .layout_no_wrap(label.clone(), theme::font_med(theme::T_LABEL), p.text)
        .rect
        .width();
    let bar_w = text_w.max(118.0) + 2.0 * 30.0 + 28.0;
    let bar = Rect::from_center_size(
        Pos2::new(area.center().x, area.bottom() - FOOT * 0.5 - 2.0),
        Vec2::new(bar_w, 34.0),
    );
    theme::lift_shadow(ui.painter(), bar, bar.height() * 0.5, 0.6);
    theme::glass_surface(ui.painter(), bar, bar.height() * 0.5, 0.94);
    let mut foot = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar.shrink2(Vec2::new(6.0, 2.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    foot.spacing_mut().item_spacing.x = 4.0;
    let can_back = st.current > 0;
    let can_fwd = st.current + 1 < st.sheets;
    if ui::icon_button(&mut foot, Icon::ChevronLeft, "Previous page  ·  ←", 28.0) && can_back {
        st.turn_to(st.current - 1);
    }
    let (mid, _) = foot.allocate_exact_size(
        Vec2::new(foot.available_width() - 32.0, 28.0),
        Sense::hover(),
    );
    foot.painter().text(
        mid.center(),
        egui::Align2::CENTER_CENTER,
        &label,
        theme::font_med(theme::T_LABEL),
        p.text_dim,
    );
    if ui::icon_button(&mut foot, Icon::ChevronRight, "Next page  ·  →", 28.0) && can_fwd {
        st.turn_to(st.current + 1);
    }
    ui.allocate_rect(area, Sense::hover());
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
