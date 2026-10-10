//! Character colours chosen by hand, for continuity: MARIA is always MARIA's
//! colour, in every script she is in and on every page she prints on. A chosen
//! colour is a hue kept in the script itself, so the whole team sees the same
//! one; it is drawn at the theme's own depth, so it reads in any theme and as
//! ink on paper. Choosing is the team edition's, for now: the desktop shows
//! chosen colours but does not offer to choose them.

use eframe::egui::{self, Align, Layout, Pos2, Rect, Sense, Stroke, Vec2};

use super::App;
use crate::alerts::Tone;
use crate::anim;
use crate::icons::Icon;
use crate::theme::{self, pal};
use crate::ui;

/// The colour picker, open for one character.
pub(super) struct Pick {
    pub name: String,
    /// Where it was opened from: the picker hangs just under it.
    pub at: Pos2,
    pub opened: u64,
    /// Set once this opening has been written to the undo history.
    pub checkpointed: bool,
}

/// The swatches every picker offers: twelve hues, a twelfth of the wheel apart.
const HUES: [u16; 12] = [0, 30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330];

impl App {
    /// The colours chosen in this script, when Custom is on; otherwise none,
    /// and everyone is dealt one.
    pub(super) fn chosen_voices(&self) -> &[(String, u16)] {
        if self.settings.custom_colors {
            &self.doc.meta.voices
        } else {
            &[]
        }
    }

    /// May character colours be chosen here? A team library, colours on.
    pub(super) fn can_choose_voices(&self) -> bool {
        self.team_edition() && self.settings.character_colors
    }

    pub(super) fn open_voice_picker(&mut self, name: &str, at: Pos2) {
        if !self.can_choose_voices() || !self.guard_edit() {
            return;
        }
        self.voice_pick = Some(Pick {
            name: name.to_string(),
            at,
            opened: self.frame_no,
            checkpointed: false,
        });
    }

    /// Give `name` a colour of its own (`Some(hue)`) or hand them back to
    /// chance. It is a change to the script, saved for everyone; choosing a
    /// colour turns Custom on, since that is what choosing one means.
    pub(super) fn set_voice(&mut self, name: &str, hue: Option<u16>) {
        if !self.guard_edit() {
            return;
        }
        if let Some(p) = self.voice_pick.as_mut() {
            if !p.checkpointed {
                p.checkpointed = true;
                self.checkpoint();
            }
        } else {
            self.checkpoint();
        }
        self.doc.meta.set_voice(name, hue);
        if hue.is_some() && !self.settings.custom_colors {
            self.settings.custom_colors = true;
            self.save_settings();
            self.deck.say(
                "Character colours: Custom",
                "Chosen colours are kept in the script, so the whole team sees them.",
                Tone::Info,
            );
        }
        self.mark_changed();
    }

    /// Settings → Colour, on a team: Random or Custom, and in Custom, every
    /// speaking character with the colour they wear.
    pub(super) fn colour_mode_settings(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Colours")
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                // right to left: Custom is laid first
                for (label, custom) in [("Custom", true), ("Random", false)] {
                    let on = self.settings.custom_colors == custom;
                    if ui::button(ui, label, None, on) && !on {
                        self.settings.custom_colors = custom;
                    }
                }
            });
        });
        let hint = if self.settings.custom_colors {
            "Click a name to choose its colour. Chosen colours live in the script, for the whole team; anyone without one is dealt one."
        } else {
            "Everyone is dealt a colour from the wheel. Shuffle deals again."
        };
        ui.label(egui::RichText::new(hint).font(theme::font(theme::T_CAP)).color(p.text_faint));
        ui.add_space(4.0);
        if !self.settings.custom_colors {
            return;
        }
        if self.layout.speakers.is_empty() {
            ui.label(
                egui::RichText::new("Nobody speaks in this script yet.")
                    .font(theme::font(theme::T_CAP))
                    .color(p.text_faint),
            );
            return;
        }
        let voices = theme::voice_colors(&self.layout.speakers, self.settings.colour_seed, self.chosen_voices());
        let mut open: Option<(String, Pos2)> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for name in &self.layout.speakers {
                let Some(c) = voices.get(name).copied() else { continue };
                let chosen = self.doc.meta.voice(name).is_some();
                let label = ui::elide(name, 18);
                let g = ui.painter().layout_no_wrap(label.clone(), theme::font_med(theme::T_CAP), c);
                let size = Vec2::new(g.rect.width() + 34.0, 24.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(theme::R_PILL.min(12.0)),
                    theme::tint(c, (if p.dark { 0.10 } else { 0.12 }) + 0.06 * hot),
                );
                let dot = Pos2::new(rect.left() + 13.0, rect.center().y);
                ui.painter().circle_filled(dot, 5.0, c);
                if chosen {
                    // a ring: this colour was chosen, not dealt
                    ui.painter().circle_stroke(dot, 7.5, Stroke::new(1.2, theme::wash(c, 200)));
                }
                ui.painter().galley(Pos2::new(rect.left() + 25.0, rect.center().y - g.rect.height() * 0.5), g, c);
                let resp = resp.on_hover_text(if chosen { "Chosen · click to change" } else { "Dealt · click to choose" });
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    open = Some((name.clone(), rect.left_bottom()));
                }
            }
        });
        if let Some((name, at)) = open {
            self.open_voice_picker(&name, at);
        }
    }

    /// The picker: twelve swatches, a hue track for anything between, and a
    /// way back to a dealt colour. It closes with Esc or a click outside it.
    pub(super) fn voice_picker(&mut self, ctx: &egui::Context) {
        let Some(pick) = self.voice_pick.as_ref() else {
            return;
        };
        let (name, at, opened) = (pick.name.clone(), pick.at, pick.opened);
        if !self.access.can_edit() {
            self.voice_pick = None;
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.voice_pick = None;
            return;
        }
        let p = pal();
        let screen = ctx.screen_rect();
        const W: f32 = 236.0;
        let pos = Pos2::new(
            at.x.clamp(screen.left() + 8.0, screen.right() - W - 32.0),
            (at.y + 6.0).min(screen.bottom() - 190.0),
        );
        let chosen = self.doc.meta.voice(&name);
        let now = theme::voice_colors(&self.layout.speakers, self.settings.colour_seed, self.chosen_voices())
            .get(&name)
            .copied()
            .unwrap_or(p.text);
        let mut set: Option<Option<u16>> = None;
        let area = egui::Area::new(egui::Id::new("ns-voice-picker"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    ui.set_width(W);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(egui::RichText::new(ui::elide(&name, 20)).font(theme::font_semi(theme::T_SM)).color(now));
                        ui.label(egui::RichText::new("\u{2019}s colour").font(theme::font_med(theme::T_SM)).color(p.text_dim));
                    });
                    ui.add_space(6.0);
                    for row in HUES.chunks(6) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.add_space(2.0);
                            for &h in row {
                                if ui::swatch(ui, theme::voice_of_hue(h), "", chosen == Some(h), None) {
                                    set = Some(Some(h));
                                }
                            }
                        });
                    }
                    ui.add_space(6.0);
                    if let Some(h) = hue_track(ui, chosen) {
                        set = Some(Some(h));
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if chosen.is_some() && ui::button(ui, "Use random", Some(Icon::Refresh), false) {
                            set = Some(None);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(if chosen.is_some() { "chosen" } else { "dealt" })
                                    .font(theme::font_mono(theme::T_MICRO))
                                    .color(p.text_faint),
                            );
                        });
                    });
                });
            });
        if let Some(h) = set {
            self.set_voice(&name, h);
        }
        let d = ui::Dismisser { opened_on: opened };
        if d.should_close(ctx, self.frame_no, &[area.response.rect]) {
            self.voice_pick = None;
        }
    }
}

/// A slim track of every hue, drawn at the theme's character depth; click or
/// drag along it for a hue between the swatches. Returns the hue picked.
fn hue_track(ui: &mut egui::Ui, chosen: Option<u16>) -> Option<u16> {
    let p = pal();
    let w = ui.available_width() - 4.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 18.0), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(w - 12.0, 8.0));
    // one mesh, so the hues run into each other with no seams
    let steps = 36;
    let mut mesh = egui::Mesh::default();
    for k in 0..=steps {
        let x = track.left() + track.width() * k as f32 / steps as f32;
        let c = theme::voice_of_hue((k * 10 % 360) as u16);
        mesh.colored_vertex(Pos2::new(x, track.top()), c);
        mesh.colored_vertex(Pos2::new(x, track.bottom()), c);
        if k > 0 {
            let i = (k * 2) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    // round the ends off into the panel
    for x in [track.left(), track.right()] {
        let c = if x == track.left() { theme::voice_of_hue(0) } else { theme::voice_of_hue(359) };
        ui.painter().circle_filled(Pos2::new(x, track.center().y), track.height() * 0.5, c);
    }
    if let Some(h) = chosen {
        let x = track.left() + track.width() * h as f32 / 360.0;
        let knob = Pos2::new(x, track.center().y);
        ui.painter().circle_filled(knob, 7.0, p.solid);
        ui.painter().circle_filled(knob, 5.0, theme::voice_of_hue(h));
        ui.painter().circle_stroke(knob, 7.0, Stroke::new(1.0, theme::wash(p.text, 120)));
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() || resp.dragged() {
        let x = resp.interact_pointer_pos()?.x;
        let t = ((x - track.left()) / track.width()).clamp(0.0, 0.9999);
        return Some((t * 360.0) as u16);
    }
    None
}
