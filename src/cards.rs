//! Cards: the script as a wall of index cards, one per scene.
//!
//! A card is glass laid over the page, the way everything that floats over
//! the work is glass in this design — lit from the object outward, lifting
//! when you reach for it, with the scene's colour as a dot and a breath of
//! tint through the pane rather than a painted bar. Drag a card by its head
//! to move the whole scene; write the synopsis straight onto it.

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use crate::anim;
use crate::icons::Icon;
use crate::model::{eighths_label, Document, Scene};
use crate::theme::{self, pal};
use crate::ui;

pub const CARD_W: f32 = 236.0;
pub const CARD_H: f32 = 156.0;
const GAP: f32 = 18.0;
const HEAD_H: f32 = 36.0;

#[derive(Default)]
pub struct CardsState {
    /// The scene being dragged, by index.
    pub dragging: Option<usize>,
    /// Where it would land: the index it would be moved in front of.
    pub drop_at: Option<usize>,
    /// The card whose menu is open, and where.
    pub menu: Option<(usize, Pos2, u64)>,
    /// Where each card was drawn, for the headless tests.
    pub last_rects: Vec<Rect>,
    pub new_card_rect: Option<Rect>,
}

/// What the page asks the shell to do.
pub enum Act {
    Open(u64),
    Move(usize, usize),
    Tint(u64, Option<usize>),
    Delete(usize),
    NewScene,
}

pub struct Out {
    pub changed: bool,
    pub act: Option<Act>,
}

/// Page, eighths and cast line for each scene, from the shell's layout pass.
pub fn show(
    ui: &mut egui::Ui,
    doc: &mut Document,
    st: &mut CardsState,
    lengths: &std::collections::HashMap<u64, (usize, usize)>,
    live: Option<u64>,
    frame_no: u64,
) -> Out {
    let p = pal();
    let scenes = doc.scenes();
    let mut out = Out {
        changed: false,
        act: None,
    };

    let avail = ui.available_width();
    let cols = (((avail - 2.0 * GAP) + GAP) / (CARD_W + GAP)).floor().max(1.0) as usize;
    let grid_w = cols as f32 * CARD_W + (cols as f32 - 1.0) * GAP;
    let left = ui.min_rect().left() + ((avail - grid_w) * 0.5).max(GAP);
    let top = ui.cursor().top() + 24.0;

    let slot = |k: usize| -> Rect {
        let (r, c) = (k / cols, k % cols);
        Rect::from_min_size(
            Pos2::new(left + c as f32 * (CARD_W + GAP), top + r as f32 * (CARD_H + GAP)),
            Vec2::new(CARD_W, CARD_H),
        )
    };
    let total = scenes.len() + 1; // the "new scene" card closes the wall
    let rows = total.div_ceil(cols);
    let (whole, _) = ui.allocate_exact_size(
        Vec2::new(avail, 24.0 + rows as f32 * (CARD_H + GAP) + 60.0),
        Sense::hover(),
    );
    let _ = whole;

    if scenes.is_empty() {
        ui.painter().text(
            Pos2::new(ui.min_rect().center().x, top - 6.0),
            egui::Align2::CENTER_BOTTOM,
            "No scenes yet — every scene heading becomes a card.",
            theme::font(theme::T_BODY),
            p.text_faint,
        );
    }

    st.last_rects.clear();
    let pointer = ui.ctx().pointer_latest_pos();
    let mut drag_started: Option<usize> = None;

    for (k, sc) in scenes.iter().enumerate() {
        let rect = slot(k);
        st.last_rects.push(rect);
        let being_dragged = st.dragging == Some(k);
        let head = Rect::from_min_size(rect.min, Vec2::new(rect.width(), HEAD_H));
        let body_resp = ui.interact(rect, egui::Id::new(("ns-card", sc.id)), Sense::click());
        let head_resp = ui.interact(head, egui::Id::new(("ns-card-head", sc.id)), Sense::click_and_drag());

        let hovered = body_resp.hovered() || head_resp.hovered();
        let selected = live == Some(sc.id);
        let lift = anim::ease(ui.ctx(), ("ns-card-lift", sc.id), hovered || selected, 0.20);
        let r = rect.expand(1.2 * lift);
        let tint = sc.tint.map(|t| p.group(t));
        let accent = tint.unwrap_or(p.primary);
        let fade = if being_dragged { 0.35 } else { 1.0 };

        draw_card(ui, r, sc, accent, tint, lift, selected, fade, lengths);

        // the synopsis is written straight onto the card
        let syn_rect = Rect::from_min_max(
            Pos2::new(r.left() + 14.0, r.top() + HEAD_H + 2.0),
            Pos2::new(r.right() - 14.0, r.bottom() - 30.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(syn_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(syn_rect.intersect(ui.clip_rect()));
        if let Some(i) = doc.index_of(sc.id) {
            let resp = child.add(
                egui::TextEdit::multiline(&mut doc.blocks[i].note)
                    .id(egui::Id::new(("ns-card-synopsis", sc.id)))
                    .frame(false)
                    .font(theme::font(theme::T_CAP + 1.0))
                    .text_color(theme::wash(p.text_dim, (255.0 * fade) as u8))
                    .desired_width(syn_rect.width())
                    .desired_rows(4)
                    .margin(egui::Margin::ZERO)
                    .hint_text(
                        egui::RichText::new("What happens in this scene?")
                            .color(theme::wash(p.text_faint, 150)),
                    ),
            );
            if resp.changed() {
                // a synopsis is one paragraph; the file keeps it on one line
                let b = &mut doc.blocks[i];
                if b.note.contains('\n') {
                    b.note = b.note.replace('\n', " ");
                }
                out.changed = true;
            }
        }

        if head_resp.drag_started() {
            drag_started = Some(k);
        }
        if head_resp.clicked() {
            out.act = Some(Act::Open(sc.id));
        }
        let heading_tip = if sc.heading.trim().is_empty() {
            "Untitled scene".to_string()
        } else {
            sc.heading.clone()
        };
        let _ = head_resp.on_hover_text(format!("{heading_tip}\nClick to open · drag to move"));
        if body_resp.secondary_clicked() {
            if let Some(q) = pointer {
                st.menu = Some((k, q, frame_no));
            }
        }
    }

    // ---- the card that makes a new scene ----
    let nk = scenes.len();
    let nr = slot(nk);
    st.new_card_rect = Some(nr);
    let resp = ui.interact(nr, egui::Id::new("ns-card-new"), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), 0.16);
    ui.painter().rect_filled(
        nr,
        egui::Rounding::same(theme::R_CARD),
        theme::wash(p.text_faint, (18.0 + 10.0 * (1.0 - hot)) as u8),
    );
    if hot > 0.01 {
        theme::glow_rect(ui.painter(), nr, theme::R_CARD, p.sec, 18.0, hot * 0.30);
    }
    theme::hover_surface(ui.painter(), nr, theme::R_CARD, p.sec, hot, false);
    let c = nr.center();
    theme::glow_star(ui.painter(), c - Vec2::new(0.0, 12.0), 7.0, p.sec_grad.1, 18.0, 0.16 + 0.24 * hot);
    theme::grad_star(
        ui.painter(),
        c - Vec2::new(0.0, 12.0),
        8.0 + 1.0 * hot,
        theme::mix(p.sec_grad.0, theme::lighten(p.sec_grad.0, 0.3), hot),
        theme::mix(p.sec_grad.1, theme::lighten(p.sec_grad.1, 0.3), hot),
    );
    ui.painter().text(
        c + Vec2::new(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        "New scene",
        theme::font_semi(theme::T_SM),
        theme::mix(p.text_dim, p.text, hot),
    );
    if resp.on_hover_text("A new scene at the end · Ctrl+Enter in the script").clicked() {
        out.act = Some(Act::NewScene);
    }

    // ---- dragging a card to a new place ----
    if let Some(k) = drag_started {
        st.dragging = Some(k);
    }
    let still_down = ui.input(|i| i.pointer.primary_down());
    if let Some(from) = st.dragging {
        if let Some(q) = pointer {
            // the gap nearest the pointer: before card k, or after the last
            let mut best = (f32::MAX, from);
            for k in 0..=scenes.len() {
                let edge = if k < scenes.len() {
                    let r = slot(k);
                    Pos2::new(r.left() - GAP * 0.5, r.center().y)
                } else {
                    let r = slot(scenes.len().saturating_sub(1));
                    Pos2::new(r.right() + GAP * 0.5, r.center().y)
                };
                let d = edge.distance(q);
                if d < best.0 {
                    best = (d, k);
                }
            }
            st.drop_at = Some(best.1);

            // where it would land, lit
            let at = best.1;
            let marker_x = if at < scenes.len() {
                slot(at).left() - GAP * 0.5
            } else {
                slot(scenes.len().saturating_sub(1)).right() + GAP * 0.5
            };
            let marker_row = if at < scenes.len() { slot(at) } else { slot(scenes.len().saturating_sub(1)) };
            let bar = Rect::from_center_size(
                Pos2::new(marker_x, marker_row.center().y),
                Vec2::new(3.0, CARD_H - 24.0),
            );
            let moving = at != from && at != from + 1;
            if moving {
                theme::glow_rect(ui.painter(), bar, 1.5, p.sec_light, 12.0, 0.9);
                ui.painter().rect_filled(bar, egui::Rounding::same(1.5), p.sec_light);
            }

            // the card itself, following the pointer
            if let Some(sc) = scenes.get(from) {
                let ghost = Rect::from_min_size(q - Vec2::new(40.0, 18.0), Vec2::new(CARD_W, HEAD_H + 8.0));
                egui::Area::new(egui::Id::new("ns-card-ghost"))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(ghost.min)
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        let (r, _) = ui.allocate_exact_size(ghost.size(), Sense::hover());
                        theme::glow_rect(ui.painter(), r, r.height() * 0.5, p.sec, 16.0, 0.35);
                        theme::glass_surface(ui.painter(), r, r.height() * 0.5, 0.96);
                        theme::grad_star(ui.painter(), Pos2::new(r.left() + 20.0, r.center().y), 5.5, p.sec_grad.0, p.sec_grad.1);
                        ui.painter().text(
                            Pos2::new(r.left() + 34.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            ui::elide(&sc.heading, 26),
                            theme::font_med(theme::T_SM),
                            p.text,
                        );
                    });
            }
            anim::keep_going(ui.ctx());
        }
        if !still_down {
            if let Some(to) = st.drop_at.take() {
                if to != from && to != from + 1 {
                    out.act = Some(Act::Move(from, to));
                }
            }
            st.dragging = None;
        }
    }

    // ---- a card's menu ----
    if let Some((k, at, opened)) = st.menu {
        if let Some(sc) = scenes.get(k) {
            let mut close = false;
            let area = egui::Area::new(egui::Id::new("ns-card-menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(at)
                .show(ui.ctx(), |ui| {
                    ui::popover_frame().show(ui, |ui| {
                        ui.set_width(ui::menu_width(ui, &["Open in the script", "Delete this scene"]).max(176.0));
                        let mut rects = Vec::new();
                        if ui::menu_item(ui, &mut rects, "Open in the script", Icon::Pencil, false) {
                            out.act = Some(Act::Open(sc.id));
                            close = true;
                        }
                        ui::section(ui, "Card colour");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.add_space(6.0);
                            for t in 0..6 {
                                if ui::swatch(ui, p.group(t), "", sc.tint == Some(t), None) {
                                    out.act = Some(Act::Tint(sc.id, Some(t)));
                                    close = true;
                                }
                            }
                            if ui::icon_button(ui, Icon::Eraser, "No colour", 22.0) {
                                out.act = Some(Act::Tint(sc.id, None));
                                close = true;
                            }
                        });
                        ui::separator(ui);
                        if ui::menu_item(ui, &mut rects, "Delete this scene", Icon::Trash, true) {
                            out.act = Some(Act::Delete(k));
                            close = true;
                        }
                    });
                })
                .response;
            let d = ui::Dismisser { opened_on: opened };
            if close
                || d.should_close(ui.ctx(), frame_no, &[area.rect])
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                st.menu = None;
            }
        } else {
            st.menu = None;
        }
    }

    out
}

#[allow(clippy::too_many_arguments)]
fn draw_card(
    ui: &egui::Ui,
    r: Rect,
    sc: &Scene,
    accent: egui::Color32,
    tint: Option<egui::Color32>,
    lift: f32,
    selected: bool,
    fade: f32,
    lengths: &std::collections::HashMap<u64, (usize, usize)>,
) {
    let p = pal();
    let painter = ui.painter();
    let radius = theme::R_CARD;

    theme::glow_rect(painter, r, radius, accent, 22.0, (0.10 + 0.22 * lift) * fade);
    if selected {
        theme::glow_rect(painter, r, radius, p.primary, 26.0, 0.30 * fade);
    }
    if !p.dark {
        painter.rect_filled(
            r.translate(Vec2::new(0.0, 2.0)),
            egui::Rounding::same(radius),
            egui::Color32::from_black_alpha((14.0 * fade) as u8),
        );
    }
    theme::glass_surface(painter, r, radius, (if selected { 0.96 } else { 0.86 }) * fade);
    if let Some(t) = tint {
        // the scene's colour as a breath through the glass
        theme::fill_grad_poly(
            painter,
            &theme::rounded_poly(r, radius),
            theme::wash(t, ((if p.dark { 44.0 } else { 34.0 }) * fade) as u8),
            theme::wash(t, 0),
            Vec2::new(0.35, 1.0),
        );
    }
    if selected {
        painter.rect_stroke(
            r.shrink(0.5),
            egui::Rounding::same(radius),
            egui::Stroke::new(1.4_f32, theme::wash(p.primary_light, (220.0 * fade) as u8)),
        );
    }

    // head: the star, the number, the heading
    let cy = r.top() + HEAD_H * 0.5 + 2.0;
    let star = Pos2::new(r.left() + 20.0, cy);
    theme::glow_star(painter, star, 4.6, p.sec_grad.1, 13.0, (0.10 + 0.22 * lift) * fade);
    theme::grad_star(painter, star, 5.6, p.sec_grad.0, p.sec_grad.1);
    let num = format!("{}", sc.number);
    painter.text(
        Pos2::new(r.left() + 33.0, cy),
        egui::Align2::LEFT_CENTER,
        &num,
        theme::font_mono(theme::T_MICRO),
        theme::wash(p.text_faint, (255.0 * fade) as u8),
    );
    let heading = if sc.heading.trim().is_empty() {
        "Untitled scene".to_string()
    } else {
        sc.heading.clone()
    };
    let right_pad = if tint.is_some() { 26.0 } else { 14.0 };
    painter.text(
        Pos2::new(r.left() + 33.0 + 8.0 * num.len() as f32 + 6.0, cy),
        egui::Align2::LEFT_CENTER,
        ui::elide(&heading, ((r.width() - 56.0 - right_pad) / 7.8) as usize),
        theme::font_semi(theme::T_SM),
        theme::wash(p.text, (255.0 * fade) as u8),
    );
    if let Some(t) = tint {
        painter.circle_filled(Pos2::new(r.right() - 16.0, cy), 3.4, theme::wash(t, (255.0 * fade) as u8));
    }

    // foot: where it falls and who is in it
    let (page, eighths) = lengths.get(&sc.id).copied().unwrap_or((0, 0));
    let mut foot = if page > 0 {
        format!("p.{page} · {} pg", eighths_label(eighths))
    } else {
        "not on the page yet".to_string()
    };
    if !sc.cast.is_empty() {
        foot.push_str(" · ");
        foot.push_str(&sc.cast.join(", "));
    }
    painter.text(
        Pos2::new(r.left() + 16.0, r.bottom() - 16.0),
        egui::Align2::LEFT_CENTER,
        ui::elide(&foot, ((r.width() - 32.0) / 6.2) as usize),
        theme::font_mono(theme::T_MICRO),
        theme::wash(p.text_faint, (255.0 * fade) as u8),
    );
}
