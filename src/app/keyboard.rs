//! Settings → Keyboard: every shortcut, and a chord of your own for any of
//! them. They are kept with your settings, which on a team library are
//! your account's, so they are yours alone and follow you to any browser you
//! sign in on. Reset puts back the shortcuts Northstar ships with.

use eframe::egui::{self, Align, Key, Layout, Rect, Sense, Stroke, Vec2};

use super::App;
use crate::anim;
use crate::icons::Icon;
use crate::keys::{self, Command};
use crate::theme::{self, pal};
use crate::ui;

const GROUPS: [&str; 4] = ["Writing", "Export", "Views and panels", "Elements"];

impl App {
    /// While Settings → Keyboard waits for a chord, the next key pressed is
    /// the chord, and nothing else hears it. Esc on its own gives up.
    pub(super) fn capture_chord(&mut self, ctx: &egui::Context) {
        let Some(cmd) = self.key_capture else {
            return;
        };
        // Settings closed while it was listening: it listens no more
        if self.popover != Some(super::Popover::Settings) {
            self.key_capture = None;
            self.key_refusal = None;
            return;
        }
        let pressed = ctx.input_mut(|i| {
            let mut found = None;
            i.events.retain(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if found.is_none() {
                        found = Some((*key, *modifiers));
                    }
                    false
                }
                // the chord types nothing anywhere
                egui::Event::Key { .. } | egui::Event::Text(_) => false,
                _ => true,
            });
            found
        });
        let Some((key, mods)) = pressed else {
            return;
        };
        if key == Key::Escape && !(mods.ctrl || mods.command || mods.alt || mods.shift) {
            self.key_capture = None;
            self.key_refusal = None;
            return;
        }
        let chord = keys::chord(mods, key);
        if let Some(why) = keys::refused(&chord, crate::WEB) {
            self.key_refusal = Some((cmd, format!("{}: {why}", keys::text(&chord))));
            return;
        }
        let commands = self.commands();
        if let Some(other) = self.settings.keys.taken_by(&chord, cmd, &commands, crate::WEB) {
            self.key_refusal = Some((
                cmd,
                format!("{} is already {}. Give that one another shortcut first.", keys::text(&chord), other.label()),
            ));
            return;
        }
        self.settings.keys.set(cmd, chord, crate::WEB);
        self.key_capture = None;
        self.key_refusal = None;
        self.save_settings();
    }

    pub(super) fn keyboard_settings(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        ui.label(
            egui::RichText::new(
                "Click a shortcut, then press the keys you want it to be. Esc cancels. \
                 Your shortcuts are yours alone: they follow you to any browser you sign in on.",
            )
            .font(theme::font(theme::T_CAP))
            .color(p.text_dim),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let changed = !self.settings.keys.is_default();
            if changed && ui::button(ui, "Reset all to default", Some(Icon::Refresh), false) {
                self.settings.keys.reset_all();
                self.key_capture = None;
                self.key_refusal = None;
            }
            if !changed {
                ui.label(
                    egui::RichText::new("All shortcuts are Northstar's own.")
                        .font(theme::font(theme::T_CAP))
                        .color(p.text_faint),
                );
            }
        });
        ui.add_space(4.0);

        let commands = self.commands();
        for group in GROUPS {
            ui::section(ui, group);
            for cmd in commands.iter().copied().filter(|c| c.group() == group) {
                self.key_row(ui, cmd);
            }
            ui.add_space(4.0);
        }
        ui.label(
            egui::RichText::new(
                "Writing keys stay as they are: Enter, Tab and Shift+Tab, Backspace, and Alt+Up and Down to move a block.",
            )
            .font(theme::font(theme::T_CAP))
            .color(p.text_faint),
        );
    }

    fn key_row(&mut self, ui: &mut egui::Ui, cmd: Command) {
        let p = pal();
        let web = crate::WEB;
        let listening = self.key_capture == Some(cmd);
        let custom = self.settings.keys.is_custom(cmd);
        ui.horizontal(|ui| {
            ui.set_min_height(28.0);
            ui.label(egui::RichText::new(cmd.label()).font(theme::font(theme::T_SM)).color(p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if custom {
                    let back = cmd.defaults(web).first().map(keys::text).unwrap_or_default();
                    if ui::icon_button(ui, Icon::Refresh, &format!("Back to {back}"), 24.0) {
                        self.settings.keys.reset(cmd);
                        self.key_refusal = None;
                    }
                }
                let chords: Vec<String> = self.settings.keys.chords(cmd, web).iter().map(keys::text).collect();
                let text = if listening { "Press keys…".to_string() } else { chords.join("  or  ") };
                if keycap(ui, &text, listening, custom) {
                    self.key_capture = if listening { None } else { Some(cmd) };
                    self.key_refusal = None;
                }
            });
        });
        if let Some((c, why)) = &self.key_refusal {
            if *c == cmd {
                ui.label(egui::RichText::new(why).font(theme::font(theme::T_CAP)).color(p.warn));
            }
        }
    }
}

/// A shortcut, drawn as a key cap; click it to choose another. Lit while it
/// waits for keys, and tinted when it is one you chose.
fn keycap(ui: &mut egui::Ui, text: &str, listening: bool, custom: bool) -> bool {
    let p = pal();
    let font = theme::font_mono(theme::T_CAP);
    let g = ui.painter().layout_no_wrap(text.to_string(), font, p.text);
    let size = Vec2::new(g.rect.width() + 20.0, 24.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    let lit = anim::ease(ui.ctx(), resp.id.with("lit"), listening, 0.24);
    let fill = theme::mix(p.sunken, theme::mix(p.sunken, p.primary, 0.25), lit.max(0.4 * hot));
    ui.painter().rect_filled(rect, egui::Rounding::same(6.0), fill);
    // the key's lip, a shade under it
    ui.painter().line_segment(
        [
            rect.left_bottom() + Vec2::new(5.0, -0.5),
            rect.right_bottom() + Vec2::new(-5.0, -0.5),
        ],
        Stroke::new(1.0, theme::wash(p.line_strong, 160)),
    );
    let edge = if listening {
        theme::wash(p.primary_light, 200)
    } else if custom {
        theme::wash(p.primary_light, 110)
    } else {
        theme::wash(p.line_strong, 140)
    };
    ui.painter().rect_stroke(rect, egui::Rounding::same(6.0), Stroke::new(1.0, edge));
    let ink = if listening {
        p.primary_light
    } else if custom {
        theme::mix(p.text, p.primary_light, 0.35)
    } else {
        theme::mix(p.text_dim, p.text, hot)
    };
    let at = Rect::from_center_size(rect.center(), g.rect.size()).min;
    ui.painter().galley(at, g, ink);
    if listening {
        anim::keep_going(ui.ctx());
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.on_hover_text(if listening { "Press the keys · Esc cancels" } else { "Click, then press the keys you want" })
        .clicked()
}
