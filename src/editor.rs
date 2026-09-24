//! The page: a column of typed blocks, one screenplay element each.
//!
//! Every keystroke that has structural meaning (Enter, Tab, Backspace at the
//! head of a block, Alt+Arrow) is intercepted *before* the TextEdit sees it,
//! turned into an `Act`, and applied after the frame is laid out. That keeps
//! every mutation out of the render pass, which is what stops the borrow
//! checker and the layout from fighting each other.

use eframe::egui::{self, Key, Modifiers, Sense, Stroke};

use crate::caret;
use crate::model::{guess_element, Document, Element, PAGE_COLS};
use crate::theme;

const GUTTER: f32 = 74.0;
const PAGE_PAD: f32 = 34.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Caret {
    Start,
    End,
    At(usize),
}

pub struct EditorState {
    pub focus_block: Option<u64>,
    pub pending_focus: Option<(u64, Caret)>,
    pub scroll_to_focus: bool,
    pub font_px: f32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            focus_block: None,
            pending_focus: None,
            scroll_to_focus: false,
            font_px: 16.0,
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

    /// Element of the block the caret is in, for the palette highlight.
    pub fn current_element(&self, doc: &Document) -> Option<Element> {
        self.focus_block
            .and_then(|id| doc.block(id))
            .map(|b| b.element)
    }
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
}

fn block_id(b: u64) -> egui::Id {
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

/// Draws the page. Returns true if the document was modified this frame.
pub fn show(ui: &mut egui::Ui, doc: &mut Document, st: &mut EditorState) -> bool {
    let font = egui::FontId::monospace(st.font_px);
    let char_w = ui.fonts(|f| f.glyph_width(&font, 'M')).max(1.0);
    let row_h = ui.fonts(|f| f.row_height(&font));
    let text_w = char_w * PAGE_COLS as f32;
    let page_w = (text_w + GUTTER + PAGE_PAD * 2.0).min(ui.available_width() - 16.0);

    let mut changed = false;
    let mut acts: Vec<Act> = Vec::new();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(26.0);
            let avail = ui.available_width();
            let left_pad = ((avail - page_w) * 0.5).max(0.0);

            ui.horizontal(|ui| {
                ui.add_space(left_pad);
                ui.vertical(|ui| {
                    ui.set_max_width(page_w);
                    egui::Frame::none()
                        .fill(theme::PAGE)
                        .stroke(Stroke::new(1.0, theme::LINE))
                        .rounding(egui::Rounding::same(theme::R_CARD))
                        .inner_margin(egui::Margin::symmetric(PAGE_PAD, PAGE_PAD + 6.0))
                        .show(ui, |ui| {
                            ui.set_min_width(page_w - PAGE_PAD * 2.0);
                            ui.style_mut().spacing.item_spacing.y = 0.0;
                            changed |= page_body(ui, doc, st, &mut acts, &font, char_w, row_h);
                        });
                });
            });
            ui.add_space(120.0);
        });

    for act in acts {
        changed |= apply(doc, st, act);
    }
    doc.ensure_not_empty();
    changed
}

fn page_body(
    ui: &mut egui::Ui,
    doc: &mut Document,
    st: &mut EditorState,
    acts: &mut Vec<Act>,
    font: &egui::FontId,
    char_w: f32,
    row_h: f32,
) -> bool {
    let mut changed = false;
    let content_left = ui.max_rect().left();
    let mut any_focus = false;

    for i in 0..doc.blocks.len() {
        let bid = doc.blocks[i].id;
        let element = doc.blocks[i].element;
        let id = block_id(bid);
        let focused = ui.memory(|m| m.has_focus(id));

        // ---- vertical rhythm between elements ----
        if i > 0 {
            ui.add_space(element.blank_lines_before() as f32 * row_h * 0.62 + 2.0);
        }

        // ---- structural keys, before the TextEdit consumes them ----
        if focused {
            any_focus = true;
            st.focus_block = Some(bid);
            let len = caret::char_len(&doc.blocks[i].text);
            let pos = caret::caret(ui.ctx(), id);
            let at = pos.unwrap_or(len);

            ui.input_mut(|inp| {
                if inp.consume_key(Modifiers::NONE, Key::Enter) {
                    acts.push(Act::Split {
                        id: bid,
                        at,
                        same: false,
                    });
                } else if inp.consume_key(Modifiers::SHIFT, Key::Enter) {
                    acts.push(Act::Split {
                        id: bid,
                        at,
                        same: true,
                    });
                } else if inp.consume_key(Modifiers::NONE, Key::Tab) {
                    acts.push(Act::SetElement {
                        id: bid,
                        element: element.cycle(true),
                        at,
                    });
                } else if inp.consume_key(Modifiers::SHIFT, Key::Tab) {
                    acts.push(Act::SetElement {
                        id: bid,
                        element: element.cycle(false),
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

        // ---- the row ----
        let bg_idx = ui.painter().add(egui::Shape::Noop);
        let indent_px = element.indent_cols() as f32 * char_w;
        let width_px = element.width_cols() as f32 * char_w;

        let response = ui
            .horizontal(|ui| {
                ui.add_space(GUTTER + indent_px);
                let block = &mut doc.blocks[i];
                let te = egui::TextEdit::multiline(&mut block.text)
                    .id(id)
                    .font(font.clone())
                    .text_color(theme::element_text_color(element))
                    .desired_width(width_px)
                    .desired_rows(1)
                    .frame(false)
                    .lock_focus(false)
                    .hint_text(
                        egui::RichText::new(hint_for(element)).color(theme::TEXT_FAINT),
                    );
                ui.add(te)
            })
            .inner;

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
        }

        // ---- focused row treatment ----
        let row = response.rect;
        let strip = egui::Rect::from_min_max(
            egui::pos2(content_left, row.top() - 4.0),
            egui::pos2(content_left + ui.max_rect().width(), row.bottom() + 4.0),
        );
        if focused {
            ui.painter().set(
                bg_idx,
                egui::Shape::rect_filled(
                    strip,
                    egui::Rounding::same(8.0),
                    theme::SURFACE_HI,
                ),
            );
            let bar = egui::Rect::from_min_size(
                egui::pos2(content_left, row.top() - 2.0),
                egui::vec2(3.0, (row.height() + 4.0).max(row_h)),
            );
            ui.painter()
                .rect_filled(bar, egui::Rounding::same(2.0), theme::RED);
        } else if element == Element::SceneHeading {
            let bar = egui::Rect::from_min_size(
                egui::pos2(content_left, row.top()),
                egui::vec2(3.0, row.height().max(row_h)),
            );
            ui.painter()
                .rect_filled(bar, egui::Rounding::same(2.0), theme::RED_DEEP);
        }

        // ---- gutter tag ----
        if focused || response.hovered() {
            let color = theme::element_color(element);
            ui.painter().text(
                egui::pos2(content_left + 12.0, row.top() + row_h * 0.5),
                egui::Align2::LEFT_CENTER,
                element.short(),
                egui::FontId::proportional(10.0),
                color,
            );
        }

        // ---- deferred focus ----
        if let Some((fid, target)) = st.pending_focus {
            if fid == bid {
                response.request_focus();
                let len = caret::char_len(&doc.blocks[i].text);
                let index = match target {
                    Caret::Start => 0,
                    Caret::End => len,
                    Caret::At(n) => n.min(len),
                };
                caret::set(ui.ctx(), id, index);
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

fn apply(doc: &mut Document, st: &mut EditorState, act: Act) -> bool {
    match act {
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
