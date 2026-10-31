//! Cards: the script as a wall of index cards, one per scene.
//!
//! A card is one of the site's panels: opaque, hairlined, its scene number
//! set big in the display face and its details as HUD readouts, with the
//! scene's colour as a bar down its edge. Drag a card by its head to move the
//! whole scene; write the synopsis straight onto it. An act opens a new row
//! of the wall with a chapter heading across it.

use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

use crate::anim;
use crate::icons::Icon;
use crate::model::{act_title, eighths_label, ActSpan, Document, Scene};
use crate::theme::{self, pal};
use crate::ui;

pub const CARD_W: f32 = 236.0;
pub const CARD_H: f32 = 156.0;
const GAP: f32 = 18.0;
const HEAD_H: f32 = 36.0;
/// An act's ribbon across the wall, and the room under it.
const RIBBON_H: f32 = 40.0;
const RIBBON_GAP: f32 = 12.0;

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
    /// Go to an act in the script, by its block id.
    OpenAct(u64),
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
    act_notes: &std::collections::HashMap<u64, String>,
    live: Option<u64>,
    frame_no: u64,
) -> Out {
    let p = pal();
    let scenes = doc.scenes();
    let acts = doc.acts();
    let mut out = Out {
        changed: false,
        act: None,
    };

    let avail = ui.available_width();
    let cols = (((avail - 2.0 * GAP) + GAP) / (CARD_W + GAP)).floor().max(1.0) as usize;
    let grid_w = cols as f32 * CARD_W + (cols as f32 - 1.0) * GAP;
    let left = ui.min_rect().left() + ((avail - grid_w) * 0.5).max(GAP);
    let top = ui.cursor().top() + 24.0;

    // Where every card goes, row by row. An act starts a fresh row with its
    // ribbon over it; without acts this is the plain grid.
    let mut slots: Vec<Rect> = Vec::with_capacity(scenes.len() + 1);
    let mut ribbons: Vec<(Rect, &ActSpan)> = Vec::new();
    let (mut y, mut c) = (top, 0usize);
    let mut next_act = 0usize;
    let mut ribbon = |y: &mut f32, c: &mut usize, a| {
        if *c > 0 {
            *y += CARD_H + GAP;
            *c = 0;
        }
        ribbons.push((Rect::from_min_size(Pos2::new(left, *y), Vec2::new(grid_w, RIBBON_H)), a));
        *y += RIBBON_H + RIBBON_GAP;
    };
    for sc in &scenes {
        while let Some(a) = acts.get(next_act).filter(|a| a.start < sc.start) {
            ribbon(&mut y, &mut c, a);
            next_act += 1;
        }
        slots.push(Rect::from_min_size(
            Pos2::new(left + c as f32 * (CARD_W + GAP), y),
            Vec2::new(CARD_W, CARD_H),
        ));
        c += 1;
        if c == cols {
            c = 0;
            y += CARD_H + GAP;
        }
    }
    for a in &acts[next_act.min(acts.len())..] {
        ribbon(&mut y, &mut c, a);
    }
    // the "new scene" card closes the wall
    slots.push(Rect::from_min_size(
        Pos2::new(left + c as f32 * (CARD_W + GAP), y),
        Vec2::new(CARD_W, CARD_H),
    ));
    let slot = |k: usize| slots[k];
    let bottom = slots.last().map(|r| r.bottom()).unwrap_or(top) + GAP;
    let (whole, _) = ui.allocate_exact_size(
        Vec2::new(avail, 24.0 + (bottom - top) + 60.0),
        Sense::hover(),
    );
    let _ = whole;

    for (rect, a) in &ribbons {
        if act_ribbon(ui, *rect, a, act_notes.get(&a.id).map(String::as_str).unwrap_or("")) {
            out.act = Some(Act::OpenAct(a.id));
        }
    }

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
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    // an empty slot: the hairline only, a panel fill coming in from the
    // left as you reach for it, like the site's ghost button
    ui::wipe(ui.painter(), nr, hot, if p.dark { p.raised } else { p.sunken });
    ui.painter().rect_stroke(
        nr.shrink(0.5),
        egui::Rounding::same(theme::R_CARD),
        egui::Stroke::new(1.0_f32, theme::mix(p.line, p.text_faint, hot)),
    );
    let c = nr.center();
    crate::icons::draw(
        ui.painter(),
        Rect::from_center_size(c - Vec2::new(0.0, 12.0), Vec2::splat(16.0)),
        Icon::Plus,
        theme::mix(p.text_dim, p.text, hot),
    );
    let label = "NEW SCENE";
    let font = theme::font_hud(theme::T_CAP);
    let lw = theme::tracked_width(ui.ctx(), label, font.clone(), 0.9);
    theme::tracked_text(
        ui.painter(),
        Pos2::new(c.x - lw * 0.5, c.y + 14.0),
        label,
        font,
        theme::mix(p.text_dim, p.text, hot),
        0.9,
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
                // where it will land: the one light that has a job to do
                ui.painter().rect_filled(bar, egui::Rounding::ZERO, p.primary_light);
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
                        theme::glass_surface(ui.painter(), r, theme::R_CARD, 1.0);
                        ui.painter().rect_stroke(r.shrink(0.5), egui::Rounding::same(theme::R_CARD), egui::Stroke::new(1.0_f32, p.primary_light));
                        ui.painter().rect_filled(
                            Rect::from_center_size(Pos2::new(r.left() + 20.0, r.center().y), Vec2::splat(7.0)),
                            egui::Rounding::ZERO,
                            p.primary_light,
                        );
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

/// An act's ribbon across the wall: the script's chapter heading in small —
/// the signal square, the title in the display face, a HUD readout of what it
/// holds, and a hairline running out to the right. Returns true when clicked.
fn act_ribbon(ui: &mut egui::Ui, rect: Rect, a: &ActSpan, note: &str) -> bool {
    let p = pal();
    let resp = ui.interact(rect, egui::Id::new(("ns-act-ribbon", a.id)), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    let cy = rect.center().y;
    ui.painter().rect_filled(
        Rect::from_center_size(Pos2::new(rect.left() + 10.0, cy), Vec2::splat(7.0)),
        egui::Rounding::ZERO,
        theme::mix(p.primary_light, p.text, hot),
    );
    let title = if a.title.trim().is_empty() { act_title(a.number) } else { a.title.trim().to_string() };
    let g = ui
        .painter()
        .layout_no_wrap(title.to_uppercase(), theme::font_display(20.0), p.text);
    let w = g.rect.width();
    ui.painter().galley(Pos2::new(rect.left() + 24.0, cy - g.rect.height() * 0.5), g, p.text);
    let mut x = rect.left() + 24.0 + w + 16.0;
    if !note.is_empty() {
        let font = theme::font_hud(theme::T_MICRO + 0.5);
        let up = note.to_uppercase();
        let nw = theme::tracked_width(ui.ctx(), &up, font.clone(), 0.8);
        if x + nw + 40.0 < rect.right() {
            theme::tracked_text(ui.painter(), Pos2::new(x, cy), &up, font, p.text_faint, 0.8);
            x += nw + 16.0;
        }
    }
    let end = rect.right() - 8.0;
    if end - x > 24.0 {
        let y = cy.round() + 0.5;
        ui.painter().line_segment(
            [Pos2::new(x, y), Pos2::new(end, y)],
            egui::Stroke::new(1.0_f32, theme::mix(p.line, p.line_strong, hot)),
        );
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text("Go to this act in the script").clicked()
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

    // The site's panel: an opaque fill, a hairline that goes strong as you
    // reach for it, the inset top edge, and the light signal blue round the
    // scene you are in.
    let _ = accent;
    let w = |c: egui::Color32| theme::wash(c, (c.a() as f32 * fade) as u8);
    painter.rect_filled(r, egui::Rounding::same(radius), w(theme::mix(p.solid_hi, p.raised, 0.6 * lift)));
    painter.rect_stroke(
        r.shrink(0.5),
        egui::Rounding::same(radius),
        egui::Stroke::new(
            1.0_f32,
            w(if selected { p.primary_light } else { theme::mix(p.line, p.text_faint, lift) }),
        ),
    );
    theme::top_edge(painter, r);
    if let Some(t) = tint {
        // the scene's colour as a bar down the card's left edge
        painter.rect_filled(
            Rect::from_min_size(r.min + Vec2::new(1.0, 1.0), Vec2::new(3.0, r.height() - 2.0)),
            egui::Rounding::ZERO,
            w(t),
        );
    }

    // head: the number set big in the display face, then the heading
    let cy = r.top() + HEAD_H * 0.5 + 2.0;
    let num_col = theme::wash(p.primary_light, (255.0 * fade) as u8);
    let ng = painter.layout_no_wrap(format!("{:02}", sc.number), theme::font_display(18.0), num_col);
    let num_w = ng.rect.width();
    painter.galley(Pos2::new(r.left() + 14.0, cy - ng.rect.height() * 0.5), ng, num_col);
    let heading = if sc.heading.trim().is_empty() {
        "Untitled scene".to_string()
    } else {
        sc.heading.clone()
    };
    let head_x = r.left() + 14.0 + num_w + 10.0;
    painter.text(
        Pos2::new(head_x, cy),
        egui::Align2::LEFT_CENTER,
        ui::elide(&heading, ((r.right() - head_x - 14.0) / 7.8) as usize),
        theme::font_semi(theme::T_SM),
        theme::wash(p.text, (255.0 * fade) as u8),
    );

    // foot: where it falls and who is in it
    let (page, eighths) = lengths.get(&sc.id).copied().unwrap_or((0, 0));
    let mut foot = if page > 0 {
        format!("p.{page} / {} pg", eighths_label(eighths))
    } else {
        "not on the page yet".to_string()
    };
    if !sc.cast.is_empty() {
        foot.push_str(" / ");
        foot.push_str(&sc.cast.join(", "));
    }
    theme::tracked_text(
        painter,
        Pos2::new(r.left() + 14.0, r.bottom() - 16.0),
        &ui::elide(&foot, ((r.width() - 28.0) / 7.0) as usize).to_uppercase(),
        theme::font_hud(theme::T_MICRO),
        theme::wash(p.text_faint, (255.0 * fade) as u8),
        0.6,
    );
}
