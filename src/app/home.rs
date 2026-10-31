//! Home: the script picker the app opens on, with a library of scripts in it.
//!
//! A greeting, the script you were last on as one wide card, and every other
//! script as a card of its own. On a team, each card also says who saved it
//! last and who is in it now.

use std::path::PathBuf;

use eframe::egui::{self, Align, Layout, Pos2, Rect, Sense, Stroke, Vec2};
use web_time::Instant;

use super::{new_script_button, relative_time, youtrack_button, App, Popover};
use crate::anim;
use crate::icons::{self, Icon};
use crate::people;
use crate::storage::Entry;
use crate::theme::{self, pal};
use crate::ui;

/// How wide a card wants to be; the grid fills the row with as many as fit.
const CARD_W: f32 = 268.0;
const CARD_H: f32 = 132.0;
/// The continue card's title, in the display face.
const T_CONTINUE: f32 = 22.0;

impl App {
    /// Back to the picker, letting go of the script that was open.
    pub(super) fn go_home(&mut self) {
        if self.dirty {
            self.save(false);
        }
        if let Some(path) = self.path.clone() {
            self.store.close(&path);
        }
        self.home = true;
        self.home_born = Instant::now();
        self.popover = None;
        self.find.open = false;
        self.focus_mode = false;
        self.refresh_entries();
    }

    pub(super) fn home_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: theme::GAP,
                right: theme::GAP,
                top: theme::GAP * 0.5,
                bottom: theme::GAP,
            }))
            .show(ctx, |ui| {
                let full = ui.available_rect_before_wrap();
                let w = (full.width() - 2.0 * theme::GAP).clamp(300.0, 1100.0);
                let col = Rect::from_min_size(
                    Pos2::new(full.center().x - w * 0.5, full.top()),
                    Vec2::new(w, full.height()),
                );
                let mut inner = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(col)
                        .layout(Layout::top_down(Align::Min)),
                );
                // the picker settles in, the way the page does
                let t = (self.home_born.elapsed().as_secs_f32() / 0.32).clamp(0.0, 1.0);
                if t < 1.0 && anim::enabled() {
                    inner.set_opacity(0.25 + 0.75 * anim::smootherstep(t));
                    anim::keep_going(ctx);
                }
                egui::ScrollArea::vertical()
                    .id_salt("ns-home")
                    .auto_shrink([false, false])
                    .show(&mut inner, |ui| {
                        ui.set_width(w - 14.0);
                        self.home_body(ui);
                    });
            });
    }

    fn home_body(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let team = self.store.team().cloned();
        let w = ui.available_width();

        // ---- the greeting, straight on the glass ----
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let hello = match team.as_ref().map(|t| t.me.first_name().to_string()).or_else(local_name) {
                    Some(name) => format!("{}, {name}", greeting()),
                    None => greeting().to_string(),
                };
                let about = match &team {
                    Some(team) => {
                        let people = team.members.iter().filter(|m| !m.pending).count().max(1);
                        format!(
                            "{} · {} · {}",
                            team.name,
                            plural(self.entries.len(), "script", "scripts"),
                            plural(people, "person", "people"),
                        )
                    }
                    None => format!("Your library · {}", plural(self.entries.len(), "script", "scripts")),
                };
                // the site's hero: a HUD eyebrow, then the words set giant
                ui::hud(ui, &about.replace(" · ", " / "), None);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(hello.to_uppercase())
                        .font(theme::font_display(34.0))
                        .color(p.text),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.can_write() {
                    if new_script_button(ui, 150.0, &self.keys_for(crate::keys::Command::NewScript)) {
                        self.new_script();
                        self.home = false;
                    }
                    ui.add_space(4.0);
                    if ui::button(ui, "Import", Some(Icon::Upload), false) {
                        self.start_import();
                    }
                }
                if team.is_some() && ui::icon_button(ui, Icon::Trash, "Recently deleted", 30.0) {
                    self.toggle_popover(Popover::Trash);
                }
            });
        });
        ui.add_space(14.0);

        // ---- search ----
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.home_search)
                .id(egui::Id::new("ns-home-search"))
                .hint_text(if team.is_some() {
                    "Search the team's scripts  ·  Ctrl+F"
                } else {
                    "Search your scripts  ·  Ctrl+F"
                })
                .desired_width(w.min(380.0))
                .margin(egui::Margin::symmetric(12.0, 8.0)),
        );
        if self.focus_home_search {
            search.request_focus();
            self.focus_home_search = false;
        }
        ui.add_space(16.0);

        let needle = self.home_search.trim().to_lowercase();
        let mut list: Vec<Entry> = self
            .entries
            .iter()
            .filter(|e| {
                needle.is_empty()
                    || e.title.to_lowercase().contains(&needle)
                    || e.author.to_lowercase().contains(&needle)
                    || e.preview.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.modified));

        if self.entries.is_empty() {
            self.home_empty(ui);
            self.home_foot(ui);
            return;
        }

        // ---- where you left off ----
        let me = team.as_ref().map(|t| t.me.clone());
        if needle.is_empty() {
            let mine = list
                .iter()
                .find(|e| match (&me, &e.edited_by) {
                    (Some(me), Some(by)) => by.id == me.id,
                    _ => false,
                })
                .or_else(|| list.first())
                .cloned();
            if let Some(e) = mine {
                self.continue_card(ui, &e, w);
                ui.add_space(22.0);
            }
        }

        // ---- every script, as cards ----
        let (starred, rest): (Vec<Entry>, Vec<Entry>) = list.into_iter().partition(|e| e.starred);
        let mut order: Vec<PathBuf> = Vec::new();
        let mut open = None;
        let mut star = None;
        let mut menu = None;
        for (label, icon, items) in [
            ("Starred", Icon::StarFilled, &starred),
            (if team.is_some() { "Team scripts" } else { "Scripts" }, Icon::File, &rest),
        ] {
            if items.is_empty() {
                continue;
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                icons::show(ui, icon, 12.0, if label == "Starred" { p.warn } else { p.text_faint });
                ui::hud(ui, &format!("{label} / {:02}", items.len()), Some(p.text_dim));
            });
            ui.add_space(8.0);
            let gap = theme::GAP;
            let cols = (((w + gap) / (CARD_W + gap)).floor() as usize).max(1);
            let card_w = (w - gap * (cols - 1) as f32) / cols as f32;
            // the arrow keys move a row at a time
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("ns-home-cols"), cols));
            for row in items.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for e in row {
                        // the ring only once the arrow keys are in use
                        let selected = self.home_keyed && self.home_sel == order.len();
                        order.push(e.path.clone());
                        match self.script_card(ui, e, card_w, selected) {
                            CardAction::Open => open = Some(e.path.clone()),
                            CardAction::Star => star = Some(e.path.clone()),
                            CardAction::Menu(at) => menu = Some((e.path.clone(), at)),
                            CardAction::None => {}
                        }
                    }
                });
                ui.add_space(gap);
            }
            ui.add_space(10.0);
        }
        if order.is_empty() {
            ui.label(
                egui::RichText::new("Nothing matches.")
                    .font(theme::font(theme::T_SM))
                    .color(p.text_faint),
            );
        }
        self.home_order = order;

        self.home_foot(ui);

        if let Some(path) = star {
            self.toggle_star(&path);
        }
        if let Some((path, at)) = menu {
            self.rail.menu = Some((path, at, self.frame_no));
        }
        if let Some(path) = open {
            self.open_from_home(path);
        }
    }

    /// Open a script from the picker. The picker stays up, with the card
    /// saying so, until the script arrives.
    pub(super) fn open_from_home(&mut self, path: PathBuf) {
        self.mode = super::Mode::Write;
        self.open(path);
    }

    fn continue_card(&mut self, ui: &mut egui::Ui, e: &Entry, w: f32) {
        let p = pal();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 98.0), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        // the site's chosen panel: the light signal blue round it
        ui.painter().rect_filled(rect, egui::Rounding::same(theme::R_SM), theme::mix(p.solid, p.solid_hi, hot));
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            egui::Rounding::same(theme::R_SM),
            Stroke::new(1.0_f32, p.primary_light),
        );
        theme::top_edge(ui.painter(), rect);

        let x = rect.left() + 26.0;
        ui.painter().rect_filled(
            Rect::from_center_size(Pos2::new(x + 3.0, rect.top() + 22.0), Vec2::splat(6.0)),
            egui::Rounding::ZERO,
            p.primary_light,
        );
        theme::tracked_text(
            ui.painter(),
            Pos2::new(x + 14.0, rect.top() + 22.0),
            "CONTINUE WRITING",
            theme::font_hud(theme::T_MICRO + 0.5),
            p.primary_light,
            0.8,
        );
        ui.painter().text(
            Pos2::new(x, rect.top() + 48.0),
            egui::Align2::LEFT_CENTER,
            ui::elide(&e.title.to_uppercase(), ((rect.width() - 220.0) / 15.0) as usize),
            theme::font_display(T_CONTINUE),
            p.text,
        );
        let me = self.me();
        let mut meta = format!(
            "{} / {}",
            plural(e.pages, "page", "pages"),
            plural(e.scenes, "scene", "scenes")
        );
        if let Some(by) = &e.edited_by {
            meta.push_str(&format!(" / saved by {}", people::by_line(by, me.as_ref(), &relative_time(e.modified))));
        }
        theme::tracked_text(
            ui.painter(),
            Pos2::new(x, rect.top() + 76.0),
            &meta.replace(" · ", " / ").to_uppercase(),
            theme::font_hud(theme::T_MICRO + 0.5),
            p.text_faint,
            0.8,
        );
        // on the right: who is in it now, or the way in
        let right = Pos2::new(rect.right() - 24.0, rect.center().y);
        if self.is_opening(&e.path) {
            ui.painter().text(right, egui::Align2::RIGHT_CENTER, "Opening…", theme::font_med(theme::T_SM), p.text_dim);
        } else if let Some(who) = &e.editing {
            editing_badge(ui, right, who);
        } else {
            let ink = theme::mix(p.text_dim, p.primary_light, hot);
            let arrow = Rect::from_center_size(Pos2::new(right.x - 7.0, right.y), Vec2::splat(14.0));
            icons::draw(ui.painter(), arrow, Icon::ArrowRight, ink);
            ui.painter().text(
                Pos2::new(arrow.left() - 6.0, right.y),
                egui::Align2::RIGHT_CENTER,
                "Open",
                theme::font_med(theme::T_SM),
                ink,
            );
        }
        if resp.clicked() {
            self.open_from_home(e.path.clone());
        }
    }

    fn script_card(&self, ui: &mut egui::Ui, e: &Entry, w: f32, selected: bool) -> CardAction {
        let p = pal();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, CARD_H), Sense::click());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
        ui.painter().rect_filled(rect, egui::Rounding::same(theme::R_SM), theme::mix(p.solid, p.solid_hi, hot));
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            egui::Rounding::same(theme::R_SM),
            Stroke::new(1.0_f32, theme::mix(p.line, p.text_faint, hot)),
        );
        theme::top_edge(ui.painter(), rect);
        if selected {
            // keyboard focus: the site's focus ring, 2px light signal blue
            // standing 2px off the card
            ui.painter().rect_stroke(
                rect.expand(3.0),
                egui::Rounding::same(theme::R_SM),
                Stroke::new(2.0_f32, p.primary_light),
            );
        }

        let pad = 16.0;
        let x = rect.left() + pad;
        let chars = ((w - pad * 2.0 - 28.0) / 8.4) as usize;
        ui.painter().text(
            Pos2::new(x, rect.top() + 24.0),
            egui::Align2::LEFT_CENTER,
            ui::elide(&e.title, chars),
            theme::font_bold(theme::T_BODY + 0.5),
            p.text,
        );
        let author = if e.author.trim().is_empty() {
            "No author yet".to_string()
        } else {
            format!("by {}", e.author.trim())
        };
        ui.painter().text(
            Pos2::new(x, rect.top() + 46.0),
            egui::Align2::LEFT_CENTER,
            ui::elide(&author, chars + 4),
            theme::font(theme::T_CAP),
            p.text_dim,
        );
        theme::tracked_text(
            ui.painter(),
            Pos2::new(x, rect.top() + 68.0),
            &format!("{:02} pg / {:02} sc", e.pages, e.scenes).to_uppercase(),
            theme::font_hud(theme::T_MICRO + 0.5),
            p.text_faint,
            0.8,
        );

        // the foot: who saved it last
        let foot_y = rect.bottom() - 24.0;
        let me = self.me();
        if self.is_opening(&e.path) {
            ui.painter().text(
                Pos2::new(x, foot_y),
                egui::Align2::LEFT_CENTER,
                "Opening…",
                theme::font_med(theme::T_CAP),
                p.text_dim,
            );
        } else if let Some(who) = &e.editing {
            editing_badge(ui, Pos2::new(rect.right() - pad, foot_y), who);
            if let Some(by) = &e.edited_by {
                people::avatar(ui, Rect::from_center_size(Pos2::new(x + 9.0, foot_y), Vec2::splat(18.0)), by);
            }
        } else if let Some(by) = &e.edited_by {
            people::avatar(ui, Rect::from_center_size(Pos2::new(x + 9.0, foot_y), Vec2::splat(18.0)), by);
            ui.painter().text(
                Pos2::new(x + 26.0, foot_y),
                egui::Align2::LEFT_CENTER,
                people::by_line(by, me.as_ref(), &relative_time(e.modified)),
                theme::font(theme::T_CAP),
                p.text_dim,
            );
        } else {
            ui.painter().text(
                Pos2::new(x, foot_y),
                egui::Align2::LEFT_CENTER,
                relative_time(e.modified),
                theme::font(theme::T_CAP),
                p.text_faint,
            );
        }

        // the star, top right: lit when starred, there when reached for
        let star_rect = Rect::from_center_size(Pos2::new(rect.right() - 22.0, rect.top() + 24.0), Vec2::splat(24.0));
        let star_resp = ui.interact(star_rect, resp.id.with("star"), Sense::click());
        let star_hot = anim::ease(ui.ctx(), star_resp.id, star_resp.hovered(), anim::HOVER);
        if e.starred || hot > 0.02 {
            icons::draw(
                ui.painter(),
                star_rect.shrink(6.0),
                if e.starred { Icon::StarFilled } else { Icon::Star },
                if e.starred {
                    theme::lighten(p.warn, 0.2 * star_hot)
                } else {
                    theme::mix(theme::wash(p.text_faint, (200.0 * hot) as u8), p.warn, star_hot)
                },
            );
        }
        let star_clicked = star_resp
            .on_hover_text(if e.starred { "Unstar — stars are yours alone" } else { "Star it, for you" })
            .clicked();

        let resp = if e.preview.is_empty() {
            resp
        } else {
            resp.on_hover_text(format!("{}\nright click for more", e.preview))
        };
        if star_clicked {
            CardAction::Star
        } else if resp.secondary_clicked() {
            ui.ctx()
                .pointer_latest_pos()
                .map(CardAction::Menu)
                .unwrap_or(CardAction::None)
        } else if resp.clicked() {
            CardAction::Open
        } else {
            CardAction::None
        }
    }

    fn home_empty(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        theme::island_frame(theme::R_ISLAND)
            .inner_margin(egui::Margin::symmetric(28.0, 26.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new("NO SCRIPTS YET")
                        .font(theme::font_display(26.0))
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(if self.can_write() {
                        "Start one, or bring one in: Fountain, Final Draft and Blackline's own files all import."
                    } else {
                        "When someone on the team starts a script, it shows up here."
                    })
                    .font(theme::font(theme::T_SM))
                    .color(p.text_dim),
                );
                if self.can_write() {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if new_script_button(ui, 150.0, &self.keys_for(crate::keys::Command::NewScript)) {
                            self.new_script();
                            self.home = false;
                        }
                        ui.add_space(4.0);
                        if ui::button(ui, "Import", Some(Icon::Upload), false) {
                            self.start_import();
                        }
                    });
                }
            });
        ui.add_space(18.0);
    }

    fn home_foot(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.set_width(260.0);
            if youtrack_button(ui, &self.settings.youtrack_url) {
                let url = self.settings.youtrack_url.trim().to_string();
                if url.starts_with("http://") || url.starts_with("https://") {
                    self.store.open_url(&url);
                } else {
                    self.deck.warn("That is not a web address", "Set the YouTrack link in Settings.");
                }
            }
            ui.add_space(10.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(260.0, 14.0), Sense::hover());
            let text = "BLACKLINE BY NATIVECOMPUTE";
            // tracked text is drawn from its left edge: centre it by its width
            let font = theme::font_hud(theme::T_MICRO);
            let width = theme::tracked_width(ui.ctx(), text, font.clone(), 0.8);
            theme::tracked_text(
                ui.painter(),
                Pos2::new(r.center().x - width * 0.5, r.center().y),
                text,
                font,
                p.text_faint,
                0.8,
            );
        });
        ui.add_space(12.0);
    }

    /// The picker's keys: arrows to move between cards, Enter to open.
    pub(super) fn home_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let typing = ctx.memory(|m| m.focused()).is_some();
        let n = self.home_order.len();
        let cols = ctx.data(|d| d.get_temp::<usize>(egui::Id::new("ns-home-cols"))).unwrap_or(3).max(1);
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::F))) {
            self.focus_home_search = true;
        }
        if typing || n == 0 {
            return;
        }
        let mut sel = self.home_sel.min(n - 1);
        let before = sel;
        let mut moved = false;
        ctx.input(|i| {
            moved = [Key::ArrowRight, Key::ArrowLeft, Key::ArrowDown, Key::ArrowUp]
                .iter()
                .any(|k| i.key_pressed(*k));
            if i.key_pressed(Key::ArrowRight) {
                sel = (sel + 1).min(n - 1);
            }
            if i.key_pressed(Key::ArrowLeft) {
                sel = sel.saturating_sub(1);
            }
            if i.key_pressed(Key::ArrowDown) {
                sel = (sel + cols).min(n - 1);
            }
            if i.key_pressed(Key::ArrowUp) {
                sel = sel.saturating_sub(cols);
            }
        });
        if moved && !self.home_keyed {
            // the first press shows where you are rather than moving off it
            self.home_keyed = true;
            sel = before;
        }
        self.home_sel = sel;
        if self.home_keyed && ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(path) = self.home_order.get(sel).cloned() {
                self.open_from_home(path);
            }
        }
    }
}

enum CardAction {
    None,
    Open,
    Star,
    Menu(Pos2),
}

/// "Alex is editing", with the quiet dot of someone being there.
fn editing_badge(ui: &egui::Ui, right: Pos2, who: &crate::backend::Person) {
    let p = pal();
    let text = format!("{} is editing", who.first_name());
    let galley = ui
        .painter()
        .layout_no_wrap(text, theme::font_med(theme::T_CAP), p.text);
    let left = right.x - galley.rect.width();
    ui.painter().circle_filled(Pos2::new(left - 9.0, right.y), 3.2, p.ok);
    ui.painter()
        .galley(Pos2::new(left, right.y - galley.rect.height() * 0.5), galley, p.text);
}

fn greeting() -> &'static str {
    use chrono::Timelike;
    match chrono::Local::now().hour() {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=22 => "Good evening",
        _ => "Up late",
    }
}

/// The name to greet you by on the desktop: your account's, first name only.
fn local_name() -> Option<String> {
    if crate::WEB {
        return None;
    }
    let raw = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).ok()?;
    let first = raw.split(['.', '_', '-', ' ']).next()?.trim();
    if first.is_empty() || first == "root" || !first.chars().all(|c| c.is_alphabetic()) {
        return None;
    }
    let mut c = first.chars();
    Some(c.next()?.to_uppercase().collect::<String>() + &c.as_str().to_lowercase())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
