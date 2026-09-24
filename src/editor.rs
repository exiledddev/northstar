//! The page: a column of typed blocks, one screenplay element each.
//!
//! Every keystroke that has structural meaning (Enter, Tab, Backspace at the
//! head of a block, Alt+Arrow) is intercepted *before* the TextEdit sees it,
//! turned into an `Act`, and applied after the frame is laid out. That keeps
//! every mutation out of the render pass, which is what stops the borrow
//! checker and the layout from fighting each other.
//!
//! The look follows the design language Tesseract set down: the
//! page is the island itself, so nothing here draws a box. The block you are
//! in gets a wash of the accent that fades away to the right; a scene heading
//! carries a four-pointed star in the gutter — Northstar's marker, standing
//! where Tesseract has its rhombus — that lights while you are in its scene;
//! and where a printed page will break, a rule fades in and out of nothing.

use std::collections::HashMap;

use eframe::egui::{self, text::CCursor, Key, Modifiers, Pos2, Rect, Sense, Vec2};

use crate::anim;
use crate::caret;
use crate::model::{base_character, complete, guess_element, Document, Element, PAGE_COLS};
use crate::theme::{self, pal};

/// Room to the left of the text column for the element tag and the star.
pub const GUTTER: f32 = 74.0;
/// Where the scene star sits, from the column's left edge.
const STAR_X: f32 = 26.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Caret {
    Start,
    End,
    At(usize),
    /// Select a run of characters — a find result.
    Range(usize, usize),
}

pub struct EditorState {
    pub focus_block: Option<u64>,
    pub pending_focus: Option<(u64, Caret)>,
    pub scroll_to_focus: bool,
    pub font_px: f32,
    /// Blocks seen on screen so far, so a new one can spring open (and an
    /// existing one does not, when a script is opened).
    seen: std::collections::HashSet<u64>,
    /// Remembered block heights, for growing a new block's slot.
    heights: HashMap<u64, f32>,
    /// Set when a different script is opened: everything on it is primed as
    /// already open.
    pub fresh: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            focus_block: None,
            pending_focus: None,
            scroll_to_focus: false,
            font_px: 16.0,
            seen: Default::default(),
            heights: Default::default(),
            fresh: true,
        }
    }
}

impl EditorState {
    pub fn focus(&mut self, id: u64, caret: Caret) {
        self.pending_focus = Some((id, caret));
    }

    pub fn jump_to(&mut self, id: u64) {
        self.pending_focus = Some((id, Caret::End));
        self.scroll_to_focus = true;
    }

    /// Put the caret on a find result and bring it into view.
    pub fn select(&mut self, id: u64, from: usize, to: usize) {
        self.pending_focus = Some((id, Caret::Range(from, to)));
        self.scroll_to_focus = true;
    }

    /// Element of the block the caret is in, for the palette highlight.
    pub fn current_element(&self, doc: &Document) -> Option<Element> {
        self.focus_block
            .and_then(|id| doc.block(id))
            .map(|b| b.element)
    }
}

/// How the page should be drawn this frame — everything the shell knows that
/// the page needs.
pub struct View<'a> {
    /// Block id → the printed page that starts at it.
    pub page_starts: &'a HashMap<u64, usize>,
    pub page_breaks: bool,
    pub scene_numbers: bool,
    pub smart_type: bool,
    pub typewriter: bool,
    /// Dim everything outside the scene being written.
    pub focus_mode: bool,
    /// What Find is looking for, if the bar is open.
    pub find: Option<&'a str>,
    /// The result Find is on: (block, first character).
    pub current: Option<(u64, usize)>,
    /// A faint band behind each block in its element's colour.
    pub element_colors: bool,
    /// Each speaker's colour, when their lines should wear it.
    pub char_colors: Option<&'a HashMap<String, egui::Color32>>,
}

/// Retype the focused block. Used by the palette and Ctrl+1..7.
pub fn set_element(doc: &mut Document, st: &mut EditorState, element: Element) -> bool {
    let Some(id) = st.focus_block else {
        return false;
    };
    apply(
        doc,
        st,
        Act::SetElement {
            id,
            element,
            at: usize::MAX,
        },
    )
}

enum Act {
    Split { id: u64, at: usize, same: bool },
    SetElement { id: u64, element: Element, at: usize },
    MergePrev { id: u64 },
    MergeNext { id: u64 },
    FocusPrev { id: u64 },
    FocusNext { id: u64 },
    MoveUp { id: u64 },
    MoveDown { id: u64 },
    Explode { id: u64 },
    Accept { id: u64, rest: String },
}

pub fn block_id(b: u64) -> egui::Id {
    egui::Id::new(("ns-block", b))
}

fn hint_for(e: Element) -> &'static str {
    match e {
        Element::SceneHeading => "INT. LOCATION - DAY",
        Element::Action => "What we see and hear.",
        Element::Character => "CHARACTER",
        Element::Parenthetical => "(under her breath)",
        Element::Dialogue => "What they say.",
        Element::Transition => "CUT TO:",
        Element::Shot => "ANGLE ON",
    }
}

/// The width the column needs at the current text size: gutter, sixty
/// characters of page, and a little air on the right.
pub fn column_width(ui: &egui::Ui, font_px: f32) -> f32 {
    let font = theme::font_page(font_px);
    let char_w = ui.fonts(|f| f.glyph_width(&font, 'M')).max(1.0);
    GUTTER + char_w * PAGE_COLS as f32 + 28.0
}

/// Draws the script's blocks into `ui`, which the caller has already sized to
/// the column. Returns true if the document was modified this frame.
pub fn show(ui: &mut egui::Ui, doc: &mut Document, st: &mut EditorState, view: &View) -> bool {
    let font = theme::font_page(st.font_px);
    let font_bold = theme::font_page_bold(st.font_px);
    let char_w = ui.fonts(|f| f.glyph_width(&font, 'M')).max(1.0);
    let row_h = ui.fonts(|f| f.row_height(&font));

    if st.fresh {
        // a different script: everything on it is already open, so only
        // blocks made from here on spring into place
        st.seen.clear();
        st.heights.clear();
        for b in &doc.blocks {
            st.seen.insert(b.id);
            ui.ctx()
                .animate_bool_with_time(egui::Id::new(("ns-grow", b.id)), true, 0.0);
        }
        st.fresh = false;
    }

    let mut acts: Vec<Act> = Vec::new();
    let mut changed = false;
    ui.style_mut().spacing.item_spacing.y = 0.0;
    changed |= page_body(
        ui, doc, st, view, &mut acts, &font, &font_bold, char_w, row_h,
    );

    for act in acts {
        changed |= apply(doc, st, act);
    }
    doc.ensure_not_empty();
    changed
}

#[allow(clippy::too_many_arguments)]
fn page_body(
    ui: &mut egui::Ui,
    doc: &mut Document,
    st: &mut EditorState,
    view: &View,
    acts: &mut Vec<Act>,
    font: &egui::FontId,
    font_bold: &egui::FontId,
    char_w: f32,
    row_h: f32,
) -> bool {
    let p = pal();
    let mut changed = false;
    let col = ui.max_rect();
    let content_left = col.left();
    let mut any_focus = false;

    // the scene the caret is in lights its star, and focus mode keeps it lit
    let live_scene = st.focus_block.and_then(|id| doc.scene_of(id));
    let mut scene_no = 0usize;
    let mut in_scene: Option<usize> = None;

    let needle = view.find.map(|s| s.to_lowercase()).filter(|s| !s.is_empty());
    // who is talking: a cue starts it, its parentheticals and dialogue carry
    // it, anything else ends it
    let mut speaker: Option<String> = None;

    for i in 0..doc.blocks.len() {
        let bid = doc.blocks[i].id;
        let element = doc.blocks[i].element;
        match element {
            Element::Character => speaker = Some(base_character(&doc.blocks[i].text)),
            Element::Parenthetical | Element::Dialogue => {}
            _ => speaker = None,
        }
        let voice = speaker
            .as_ref()
            .and_then(|n| view.char_colors.and_then(|m| m.get(n)))
            .copied();
        let id = block_id(bid);
        let focused = ui.memory(|m| m.has_focus(id));
        if element == Element::SceneHeading {
            in_scene = Some(scene_no);
            scene_no += 1;
        }
        let dimmed = view.focus_mode && live_scene.is_some() && in_scene != live_scene;

        // ---- where a printed page begins ----
        if view.page_breaks {
            if let Some(n) = view.page_starts.get(&bid) {
                page_rule(ui, content_left + GUTTER, col.right() - 10.0, *n);
            }
        }

        // ---- vertical rhythm between elements ----
        if i > 0 {
            ui.add_space(element.blank_lines_before() as f32 * row_h * 0.62 + 2.0);
        }

        // ---- SmartType: what would finish this block ----
        let len = caret::char_len(&doc.blocks[i].text);
        let pos = if focused { caret::caret(ui.ctx(), id) } else { None };
        let ghost = if focused && view.smart_type && pos == Some(len) {
            complete(doc, bid, element, &doc.blocks[i].text)
        } else {
            None
        };

        // ---- structural keys, before the TextEdit consumes them ----
        if focused {
            any_focus = true;
            st.focus_block = Some(bid);
            let at = pos.unwrap_or(len);
            let ghost = ghost.clone();

            // egui matches a chord with *extra* Shift or Alt held as the plain
            // key, so the more specific chord must always be asked for first
            ui.input_mut(|inp| {
                if inp.consume_key(Modifiers::SHIFT, Key::Enter) {
                    acts.push(Act::Split {
                        id: bid,
                        at,
                        same: true,
                    });
                } else if inp.consume_key(Modifiers::NONE, Key::Enter) {
                    acts.push(Act::Split {
                        id: bid,
                        at,
                        same: false,
                    });
                } else if inp.consume_key(Modifiers::SHIFT, Key::Tab) {
                    acts.push(Act::SetElement {
                        id: bid,
                        element: element.cycle(false),
                        at,
                    });
                } else if let Some(rest) = ghost.filter(|_| inp.consume_key(Modifiers::NONE, Key::Tab)) {
                    acts.push(Act::Accept { id: bid, rest });
                } else if inp.consume_key(Modifiers::NONE, Key::Tab) {
                    acts.push(Act::SetElement {
                        id: bid,
                        element: element.cycle(true),
                        at,
                    });
                } else if inp.consume_key(Modifiers::ALT, Key::ArrowUp) {
                    acts.push(Act::MoveUp { id: bid });
                } else if inp.consume_key(Modifiers::ALT, Key::ArrowDown) {
                    acts.push(Act::MoveDown { id: bid });
                } else if pos == Some(0) && inp.consume_key(Modifiers::NONE, Key::Backspace) {
                    acts.push(Act::MergePrev { id: bid });
                } else if pos == Some(len) && inp.consume_key(Modifiers::NONE, Key::Delete) {
                    acts.push(Act::MergeNext { id: bid });
                } else if pos == Some(0) && inp.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    acts.push(Act::FocusPrev { id: bid });
                } else if pos == Some(len) && inp.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    acts.push(Act::FocusNext { id: bid });
                }
            });
        }

        // ---- a new block springs open rather than appearing whole ----
        let grow_id = egui::Id::new(("ns-grow", bid));
        if st.seen.insert(bid) {
            ui.ctx().animate_bool_with_time(grow_id, false, 0.0);
        }
        let raw = if anim::enabled() {
            ui.ctx().animate_bool_with_time(grow_id, true, 0.5)
        } else {
            1.0
        };
        let opening = raw < 0.999;

        // ---- the row ----
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let indent_px = element.indent_cols() as f32 * char_w;
        let width_px = element.width_cols() as f32 * char_w;
        let ink = match voice {
            // a speaker's colour, held back a little for the parenthetical
            Some(v) if element == Element::Parenthetical => theme::mix(v, p.text_dim, 0.35),
            Some(v) => v,
            None => theme::element_ink(element),
        };
        let ink = if dimmed { theme::wash(ink, 70) } else { ink };

        let row_top = ui.cursor().top();
        let lay_row = |ui: &mut egui::Ui, doc: &mut Document| {
            ui.horizontal(|ui| {
                ui.add_space(GUTTER + indent_px);
                let block = &mut doc.blocks[i];
                egui::TextEdit::multiline(&mut block.text)
                    .id(id)
                    .font(if element == Element::SceneHeading {
                        font_bold.clone()
                    } else {
                        font.clone()
                    })
                    .text_color(ink)
                    .desired_width(width_px)
                    .desired_rows(1)
                    .frame(false)
                    .margin(egui::Margin::symmetric(0.0, 1.0))
                    .lock_focus(false)
                    .hint_text(
                        egui::RichText::new(hint_for(element))
                            .color(theme::wash(p.text_faint, if dimmed { 60 } else { 160 })),
                    )
                    .show(ui)
            })
            .inner
        };
        let output = if opening {
            let known = st.heights.get(&bid).copied().unwrap_or(row_h + 2.0);
            let grow = anim::bounce(raw);
            let (slot, _) = ui.allocate_exact_size(
                Vec2::new(col.width(), (known * grow).max(0.0)),
                Sense::hover(),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(slot.min, Vec2::new(col.width(), known.max(row_h) + 40.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(slot.intersect(ui.clip_rect()));
            let out = lay_row(&mut child, doc);
            st.heights.insert(bid, child.min_rect().height());
            anim::keep_going(ui.ctx());
            out
        } else {
            let out = lay_row(ui, doc);
            st.heights.insert(bid, ui.cursor().top() - row_top);
            out
        };
        let response = output.response.clone();

        if response.changed() {
            changed = true;
            let block = &mut doc.blocks[i];
            if block.text.contains('\n') {
                acts.push(Act::Explode { id: bid });
            } else if element.is_upper() {
                let upper = block.text.to_uppercase();
                if upper != block.text {
                    block.text = upper;
                }
            }
            if view.typewriter {
                response.scroll_to_me(Some(egui::Align::Center));
            }
        }

        // ---- the row's own light ----
        let row = response.rect;
        let lit = anim::ease(ui.ctx(), ("ns-wash", bid), focused, 0.24);
        let mut under: Vec<egui::Shape> = Vec::new();
        // the element's band: where its text may run, in its colour
        if view.element_colors && !dimmed {
            // The band hugs the words themselves rather than the whole column
            // the element may use, so the page reads as text with a tint
            // under it, not as a stack of bars. A thin tick at its left edge,
            // in the element's own colour, is what the eye actually sorts by.
            let words = output.galley.rect.translate(output.galley_pos.to_vec2());
            let text_left = content_left + GUTTER + indent_px;
            let right = if doc.blocks[i].text.is_empty() {
                text_left + char_w * 8.0
            } else {
                words.right().max(text_left + char_w * 4.0)
            };
            let band = Rect::from_min_max(
                Pos2::new(text_left - 7.0, row.top() - 1.5),
                Pos2::new((right + 7.0).min(col.right() - 2.0), row.bottom() + 1.5),
            );
            // a speech wears its speaker's colour, when characters have them
            let (fill, tick) = match voice {
                Some(v) => (theme::wash(v, if p.dark { 8 } else { 12 }), v),
                None => (theme::element_band(element), theme::element_color(element)),
            };
            under.push(egui::Shape::rect_filled(band, egui::Rounding::same(5.0), fill));
            if element != Element::Action || voice.is_some() {
                under.push(egui::Shape::rect_filled(
                    Rect::from_min_size(
                        Pos2::new(band.left(), band.top() + 3.0),
                        Vec2::new(2.0, (band.height() - 6.0).max(4.0)),
                    ),
                    egui::Rounding::same(1.0),
                    theme::wash(tick, if p.dark { 150 } else { 170 }),
                ));
            }
        }
        if lit > 0.01 {
            // A wash that fades away to the right, not a bordered card. It
            // takes in the gutter, so the element's tag sits inside the light
            // with its line rather than outside it.
            let plate = Rect::from_min_max(
                Pos2::new(content_left + 2.0, row.top() - 6.0),
                Pos2::new(col.right(), row.bottom() + 6.0),
            );
            under.push(theme::grad_poly_shape(
                &theme::rounded_poly(plate, theme::R_CARD),
                theme::wash(p.primary, (34.0 * lit) as u8),
                theme::wash(p.primary, 0),
                Vec2::new(1.0, 0.0),
            ));
        }
        // find results, laid under the words they match
        if let Some(n) = &needle {
            let text_lower = doc.blocks[i].text.to_lowercase();
            if text_lower.contains(n.as_str()) {
                let n_len = n.chars().count();
                let chars: Vec<char> = text_lower.chars().collect();
                let pat: Vec<char> = n.chars().collect();
                let mut k = 0;
                while k + n_len <= chars.len() {
                    if chars[k..k + n_len] == pat[..] {
                        let a = output.galley.pos_from_ccursor(CCursor::new(k));
                        let b = output.galley.pos_from_ccursor(CCursor::new(k + n_len));
                        let r = if (a.min.y - b.min.y).abs() < 1.0 {
                            Rect::from_min_max(a.min, Pos2::new(b.min.x, a.max.y))
                        } else {
                            Rect::from_min_size(a.min, Vec2::new(char_w * n_len as f32, a.height()))
                        };
                        let r = r.translate(output.galley_pos.to_vec2()).expand2(Vec2::new(1.5, 0.0));
                        if view.current == Some((bid, k)) {
                            // the one Find is on glows, the way anything lit does
                            under.push(egui::Shape::rect_filled(
                                r.expand(1.0),
                                egui::Rounding::same(4.0),
                                theme::wash(p.sec_light, if p.dark { 150 } else { 120 }),
                            ));
                        } else {
                            under.push(egui::Shape::rect_filled(
                                r,
                                egui::Rounding::same(3.0),
                                theme::wash(p.sec, if p.dark { 80 } else { 60 }),
                            ));
                        }
                        k += n_len;
                    } else {
                        k += 1;
                    }
                }
            }
        }
        if !under.is_empty() {
            ui.painter().set(bg_idx, egui::Shape::Vec(under));
        }

        // ---- SmartType's suggestion, in ghost ink after the caret ----
        if let Some(rest) = &ghost {
            let end = output.galley.pos_from_ccursor(CCursor::new(len));
            let at = output.galley_pos + end.min.to_vec2();
            let g = ui.painter().layout_no_wrap(
                rest.clone(),
                if element == Element::SceneHeading {
                    font_bold.clone()
                } else {
                    font.clone()
                },
                theme::wash(p.text_faint, 170),
            );
            let w = g.rect.width();
            ui.painter().galley(at, g, theme::wash(p.text_faint, 170));
            // and the key that takes it
            let chip = Rect::from_min_size(
                Pos2::new(at.x + w + 8.0, at.y + (end.height() - 15.0) * 0.5),
                Vec2::new(28.0, 15.0),
            );
            ui.painter().rect_filled(
                chip,
                egui::Rounding::same(theme::R_PILL.min(7.5)),
                theme::wash(p.primary, 34),
            );
            ui.painter().text(
                chip.center(),
                egui::Align2::CENTER_CENTER,
                "tab",
                theme::font_mono(theme::T_MICRO - 1.0),
                p.primary_light,
            );
        }

        // ---- the gutter ----
        let mid_y = row.top() + row_h * 0.5;
        if element == Element::SceneHeading {
            let live = live_scene == in_scene && live_scene.is_some();
            scene_star(ui, Pos2::new(content_left + STAR_X, mid_y), bid, live, dimmed);
            if view.scene_numbers {
                ui.painter().text(
                    Pos2::new(content_left + STAR_X + 16.0, mid_y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", scene_no),
                    theme::font_mono(theme::T_CAP),
                    if dimmed { theme::wash(p.text_faint, 80) } else { p.text_faint },
                );
            }
        } else {
            let tag = anim::ease(ui.ctx(), ("ns-tag", bid), focused || response.hovered(), 0.14);
            if tag > 0.02 {
                let color = theme::element_color(element);
                theme::tracked_text(
                    ui.painter(),
                    Pos2::new(content_left + 10.0, mid_y),
                    element.short(),
                    theme::font_semi(theme::T_MICRO - 1.0),
                    theme::wash(color, (235.0 * tag) as u8),
                    0.9,
                );
            }
        }

        // ---- deferred focus ----
        if let Some((fid, target)) = st.pending_focus {
            if fid == bid {
                response.request_focus();
                let len = caret::char_len(&doc.blocks[i].text);
                match target {
                    Caret::Range(a, b) => caret::select(ui.ctx(), id, a.min(len), b.min(len)),
                    other => {
                        let index = match other {
                            Caret::Start => 0,
                            Caret::End => len,
                            Caret::At(n) => n.min(len),
                            Caret::Range(..) => len,
                        };
                        caret::set(ui.ctx(), id, index);
                    }
                }
                st.pending_focus = None;
                if st.scroll_to_focus {
                    response.scroll_to_me(Some(egui::Align::Center));
                    st.scroll_to_focus = false;
                }
            }
        }
    }

    if !any_focus {
        // keep the palette pointing at something sensible
        if st.focus_block.and_then(|id| doc.index_of(id)).is_none() {
            st.focus_block = doc.blocks.last().map(|b| b.id);
        }
    }

    // click the empty space under the script to keep writing
    let filler = ui.allocate_response(
        egui::vec2(ui.available_width().max(1.0), 90.0),
        Sense::click(),
    );
    if filler.clicked() {
        if let Some(last) = doc.blocks.last() {
            st.pending_focus = Some((last.id, Caret::End));
        }
    }

    changed
}

/// The star in a scene heading's gutter: Northstar's marker, drawn and lit
/// the way Tesseract draws its rhombus — a gradient shape with a bloom the
/// shape of itself and a bright core, lifting when you are in its scene.
fn scene_star(ui: &egui::Ui, c: Pos2, bid: u64, live: bool, dimmed: bool) {
    let p = pal();
    let lit = anim::ease(ui.ctx(), ("ns-star", bid), live, 0.26);
    let breath = if live {
        0.62 + 0.38 * anim::breathe(ui.ctx(), 2.8)
    } else {
        0.0
    };
    if live {
        anim::keep_going(ui.ctx());
    }
    let fade = if dimmed { 0.35 } else { 1.0 };
    // seated in a glass orb, as Tesseract's markers are: a soft pane and two
    // opposite quarters of its rim, fading out at their ends
    let orb_r = 14.0;
    ui.painter()
        .circle_filled(c, orb_r, theme::wash(p.glass, (175.0 * fade) as u8));
    let edge = theme::wash(p.glass_edge, (120.0 * fade) as u8);
    let pi = std::f32::consts::PI;
    theme::fading_arc(ui.painter(), c, orb_r - 0.6, 0.72 * pi, 1.28 * pi, 1.3, edge);
    theme::fading_arc(ui.painter(), c, orb_r - 0.6, -0.28 * pi, 0.28 * pi, 1.3, edge);
    let r = 8.6 * (1.0 + 0.14 * lit);
    let a = theme::wash(p.sec_grad.0, (255.0 * fade) as u8);
    let b = theme::wash(
        theme::mix(p.sec_grad.1, theme::lighten(p.sec_grad.1, 0.18), lit),
        (255.0 * fade) as u8,
    );
    theme::glow_star(
        ui.painter(),
        c,
        r * 0.9,
        b,
        orb_r * 3.0 * (0.6 + 0.4 * lit),
        (0.10 + 0.30 * lit * breath.max(0.7)) * fade,
    );
    theme::grad_star(ui.painter(), c, r, a, b);
    // the bright core that makes the star brighter than its own halo
    theme::grad_star(
        ui.painter(),
        c,
        r * 0.34,
        theme::lighten(b, 0.30 + 0.30 * lit),
        theme::lighten(b, 0.50 + 0.30 * lit),
    );
}

/// Where a printed page begins: a rule that fades in and out of nothing, with
/// the page's number where it will be printed, top right.
fn page_rule(ui: &mut egui::Ui, x0: f32, x1: f32, page: usize) {
    let p = pal();
    ui.add_space(6.0);
    let (slot, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
    let y = slot.center().y;
    let col = theme::wash(p.text_faint, 110);
    let label = format!("{page}.");
    let lw = ui
        .painter()
        .layout_no_wrap(label.clone(), theme::font_mono(theme::T_MICRO), col)
        .rect
        .width();
    let end = x1 - lw - 10.0;
    let mid = (x0 + end) * 0.5;
    for (a, b, ca, cb) in [
        (x0, mid, theme::wash(col, 0), col),
        (mid, end, col, theme::wash(col, 0)),
    ] {
        theme::fill_grad_poly(
            ui.painter(),
            &[
                Pos2::new(a, y - 0.5),
                Pos2::new(b, y - 0.5),
                Pos2::new(b, y + 0.5),
                Pos2::new(a, y + 0.5),
            ],
            ca,
            cb,
            Vec2::new(1.0, 0.0),
        );
    }
    ui.painter().text(
        Pos2::new(x1, y),
        egui::Align2::RIGHT_CENTER,
        label,
        theme::font_mono(theme::T_MICRO),
        p.text_faint,
    );
    ui.add_space(4.0);
}

fn apply(doc: &mut Document, st: &mut EditorState, act: Act) -> bool {
    match act {
        Act::Accept { id, rest } => {
            let Some(i) = doc.index_of(id) else {
                return false;
            };
            doc.blocks[i].text.push_str(&rest);
            let upper = doc.blocks[i].element.is_upper();
            if upper {
                doc.blocks[i].text = doc.blocks[i].text.to_uppercase();
            }
            st.focus(id, Caret::End);
            true
        }
        Act::Split { id, at, same } => {
            let Some(i) = doc.index_of(id) else {
                return false;
            };
            let element = doc.blocks[i].element;
            let (head, tail) = caret::split_at_char(&doc.blocks[i].text, at);
            let splitting_mid = !tail.trim().is_empty();
            doc.blocks[i].text = head;
            let new_element = if same || splitting_mid {
                element
            } else {
                element.on_enter()
            };
            let nb = doc.new_block(new_element, &tail);
            let nid = nb.id;
            doc.blocks.insert(i + 1, nb);
            st.focus(nid, Caret::Start);
            true
        }
        Act::SetElement { id, element, at } => {
            let Some(i) = doc.index_of(id) else {
                return false;
            };
            if doc.blocks[i].element == element {
                return false;
            }
            doc.blocks[i].element = element;
            if element.is_upper() {
                let upper = doc.blocks[i].text.to_uppercase();
                doc.blocks[i].text = upper;
            }
            st.focus(id, Caret::At(at));
            true
        }
        Act::MergePrev { id } => {
            let Some(i) = doc.index_of(id) else {
                return false;
            };
            if i == 0 {
                return false;
            }
            let cur_empty = doc.blocks[i].text.trim().is_empty();
            let same_kind = doc.blocks[i].element == doc.blocks[i - 1].element;
            let prev_id = doc.blocks[i - 1].id;
            let prev_len = caret::char_len(&doc.blocks[i - 1].text);

            if cur_empty {
                doc.blocks.remove(i);
                st.focus(prev_id, Caret::At(prev_len));
                doc.ensure_not_empty();
                true
            } else if same_kind {
                let cur = doc.blocks.remove(i);
                doc.blocks[i - 1].text.push_str(&cur.text);
                st.focus(prev_id, Caret::At(prev_len));
                true
            } else {
                // different element types: move the caret instead of mangling text
                st.focus(prev_id, Caret::At(prev_len));
                false
            }
        }
        Act::MergeNext { id } => {
            let Some(i) = doc.index_of(id) else {
                return false;
            };
            if i + 1 >= doc.blocks.len() {
                return false;
            }
            let next_empty = doc.blocks[i + 1].text.trim().is_empty();
            let same_kind = doc.blocks[i].element == doc.blocks[i + 1].element;
            if next_empty || same_kind {
                let next = doc.blocks.remove(i + 1);
                let at = caret::char_len(&doc.blocks[i].text);
                doc.blocks[i].text.push_str(&next.text);
                st.focus(id, Caret::At(at));
                true
            } else {
                false
            }
        }
        Act::FocusPrev { id } => {
            if let Some(i) = doc.index_of(id) {
                if i > 0 {
                    let pid = doc.blocks[i - 1].id;
                    st.focus(pid, Caret::End);
                }
            }
            false
        }
        Act::FocusNext { id } => {
            if let Some(i) = doc.index_of(id) {
                if i + 1 < doc.blocks.len() {
                    let nid = doc.blocks[i + 1].id;
                    st.focus(nid, Caret::Start);
                }
            }
            false
        }
        Act::MoveUp { id } => {
            if let Some(i) = doc.index_of(id) {
                if i > 0 {
                    doc.blocks.swap(i, i - 1);
                    st.focus(id, Caret::End);
                    return true;
                }
            }
            false
        }
        Act::MoveDown { id } => {
            if let Some(i) = doc.index_of(id) {
                if i + 1 < doc.blocks.len() {
                    doc.blocks.swap(i, i + 1);
                    st.focus(id, Caret::End);
                    return true;
                }
            }
            false
        }
        Act::Explode { id } => {
            // A paste that carried newlines: fan it out into typed blocks.
            let Some(i) = doc.index_of(id) else {
                return false;
            };
            let raw = std::mem::take(&mut doc.blocks[i].text);
            let mut lines = raw.split('\n');
            let first = lines.next().unwrap_or("").trim_end().to_string();
            let element = doc.blocks[i].element;
            doc.blocks[i].text = if element.is_upper() {
                first.to_uppercase()
            } else {
                first
            };

            let mut prev = element;
            let mut at = i;
            let mut last_id = id;
            for line in lines {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    prev = Element::Action;
                    continue;
                }
                let e = guess_element(trimmed, Some(prev));
                let text = if e.is_upper() {
                    trimmed.to_uppercase()
                } else {
                    trimmed.to_string()
                };
                let nb = doc.new_block(e, &text);
                last_id = nb.id;
                at += 1;
                doc.blocks.insert(at, nb);
                prev = e;
            }
            st.focus(last_id, Caret::End);
            true
        }
    }
}
