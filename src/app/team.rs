//! The team library's side of the app: who you are, who else is here, who is
//! editing what, and who wrote what when. None of it is drawn on the desktop:
//! everything here starts from `self.store.team()`, which only a team library
//! has.

use std::path::Path;

use eframe::egui::{self, Align, Layout, Pos2, Rect, Sense, Stroke, Vec2};
use web_time::{Duration, Instant, SystemTime};

use super::{relative_time, App, Mode, Popover};
use crate::alerts::Tone;
use crate::backend::{Access, Person, Role};
use crate::icons::{self, Icon};
use crate::people;
use crate::theme::{self, pal};
use crate::ui;

impl App {
    /// You, when signed in to a team.
    pub(super) fn me(&self) -> Option<Person> {
        self.store.team().map(|t| t.me.clone())
    }

    // ---------- the title bar ----------

    /// The right-hand end of the title bar on a team: the way home, then you.
    pub(super) fn title_extras(&mut self, ui: &mut egui::Ui) {
        let Some((me, role, team_name)) = self
            .store
            .team()
            .map(|t| (t.me.clone(), t.role, t.name.clone()))
        else {
            return;
        };
        let p = pal();
        ui.spacing_mut().item_spacing.x = 4.0;

        let (rect, resp) = ui.allocate_exact_size(Vec2::new(34.0, 28.0), Sense::click());
        self.account_button_rect = rect;
        let open = self.popover == Some(Popover::Account);
        ui::ghost_slot(ui, rect, resp.hovered() || open, open);
        people::avatar(ui, Rect::from_center_size(rect.center(), Vec2::splat(20.0)), &me);
        if resp
            .on_hover_text(format!("{} · {} · {team_name}", me.name, role.label()))
            .clicked()
        {
            self.toggle_popover(Popover::Account);
        }

        if !self.home && ui::icon_button(ui, Icon::Home, "All scripts", 28.0) {
            self.go_home();
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(team_name)
                .font(theme::font_med(theme::T_CAP))
                .color(p.text_faint),
        );
    }

    /// Your account: who you are signed in as, Settings, and the way out.
    pub(super) fn account_menu(&mut self, ctx: &egui::Context) {
        let Some(team) = self.store.team().cloned() else {
            self.popover = None;
            return;
        };
        let p = pal();
        let screen = ctx.screen_rect();
        let anchor = self.account_button_rect;
        let area = egui::Area::new(egui::Id::new("ns-account"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(
                (anchor.right() - 248.0).max(screen.left() + 8.0),
                anchor.bottom() + 6.0,
            ))
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    ui.set_width(232.0);
                    ui.horizontal(|ui| {
                        people::face(ui, &team.me, 34.0);
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&team.me.name)
                                    .font(theme::font_semi(theme::T_SM))
                                    .color(p.text),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} · {}", team.role.label(), team.name))
                                    .font(theme::font(theme::T_CAP))
                                    .color(p.text_faint),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui::separator(ui);
                    let mut rects = Vec::new();
                    if ui::menu_item(ui, &mut rects, "Settings", Icon::Settings, false) {
                        self.popover = Some(Popover::Settings);
                        self.popover_frame = self.frame_no;
                    }
                    if ui::menu_item(ui, &mut rects, "The team", Icon::Users, false) {
                        self.settings_tab = super::TEAM_TAB;
                        self.popover = Some(Popover::Settings);
                        self.popover_frame = self.frame_no;
                    }
                    ui::separator(ui);
                    if ui::menu_item(ui, &mut rects, "Sign out", Icon::SignOut, true) {
                        self.popover = None;
                        if self.dirty {
                            self.save(false);
                        }
                        self.store.sign_out();
                    }
                });
            })
            .response;
        self.close_popover_if_outside(ctx, &[area.rect, self.account_button_rect]);
    }

    // ---------- read only ----------

    /// May the script on the page be changed? If not, say why — once in a
    /// while, not on every keystroke — and answer no.
    pub(super) fn guard_edit(&mut self) -> bool {
        if self.access.can_edit() {
            return true;
        }
        let quiet = self
            .read_only_said
            .map(|t| t.elapsed() < Duration::from_secs(4))
            .unwrap_or(false);
        if !quiet {
            self.read_only_said = Some(Instant::now());
            match &self.access {
                Access::ReadOnly { by: Some(who) } => self.deck.say(
                    &format!("{} is editing this script", who.first_name()),
                    "It is read only for you until they finish. You can still read it and export it.",
                    Tone::Info,
                ),
                _ => self.deck.say(
                    "View only",
                    "Your role on the team is Viewer: you can read and export scripts.",
                    Tone::Info,
                ),
            }
        }
        false
    }

    /// The chip in the ribbon that says this script is read only, and why —
    /// and, once whoever had it lets go, the way in.
    pub(super) fn lock_chip(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let Access::ReadOnly { by } = self.access.clone() else {
            return;
        };
        let text = match (&by, self.lock_free) {
            (Some(who), false) => format!("{} is editing · read only", who.first_name()),
            (Some(who), true) => format!("{} has finished", who.first_name()),
            (None, _) => "View only".to_string(),
        };
        let galley = ui
            .painter()
            .layout_no_wrap(text.clone(), theme::font_med(theme::T_CAP), p.text);
        let w = galley.rect.width() + 22.0 + if by.is_some() { 24.0 } else { 18.0 };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 26.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(13.0), theme::wash(p.text_faint, 30));
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            egui::Rounding::same(13.0),
            Stroke::new(1.0_f32, theme::wash(p.text_faint, 60)),
        );
        let mut x = rect.left() + 6.0;
        match &by {
            Some(who) => {
                people::avatar(ui, Rect::from_min_size(Pos2::new(x, rect.top() + 4.0), Vec2::splat(18.0)), who);
                x += 24.0;
            }
            None => {
                icons::draw(
                    ui.painter(),
                    Rect::from_min_size(Pos2::new(x + 2.0, rect.top() + 6.0), Vec2::splat(14.0)),
                    Icon::Lock,
                    p.text_dim,
                );
                x += 20.0;
            }
        }
        ui.painter().galley(
            Pos2::new(x, rect.center().y - galley.rect.height() * 0.5),
            galley,
            p.text,
        );
        let _ = resp.on_hover_text(match &by {
            Some(who) => format!("{} has this script open for editing. It is read only for you until they finish.", who.name),
            None => "Your role on the team is Viewer: you can read and export scripts.".to_string(),
        });
        if self.lock_free && by.is_some() {
            ui.add_space(4.0);
            if ui::button(ui, "Edit now", Some(Icon::Pencil), true) {
                if let Some(path) = self.path.clone() {
                    self.opening = Some(path.clone());
                    self.store.request_edit(&path);
                }
            }
        }
    }

    // ---------- history ----------

    pub(super) fn history_panel(&mut self, ctx: &egui::Context) {
        let p = pal();
        let screen = ctx.screen_rect();
        let anchor = self.menu_button_rect;
        let me = self.me();
        let path = self.path.clone();
        let sessions = path.as_ref().and_then(|x| self.store.history(x));
        let loading = sessions.is_none() && path.is_some();
        let sessions = sessions.unwrap_or_default();
        let area = egui::Area::new(egui::Id::new("ns-history"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(
                (anchor.right() - 320.0).max(screen.left() + 8.0),
                theme::TOPBAR_H + 4.0,
            ))
            .show(ctx, |ui| {
                ui::popover_frame()
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(296.0);
                        ui.horizontal(|ui| {
                            icons::show(ui, Icon::Timeline, 14.0, p.primary);
                            ui.label(
                                egui::RichText::new("History")
                                    .font(theme::font_semi(theme::T_H - 1.0))
                                    .color(p.text),
                            );
                        });
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Who worked on this script, and when. Whenever someone new picks it up, the version before is kept as a snapshot.")
                                .font(theme::font(theme::T_CAP))
                                .color(p.text_faint),
                        );
                        ui.add_space(8.0);
                        if sessions.is_empty() {
                            ui.label(
                                egui::RichText::new(if loading { "Fetching…" } else { "Nothing yet." })
                                    .font(theme::font(theme::T_SM))
                                    .color(p.text_dim),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for s in &sessions {
                                    ui.horizontal(|ui| {
                                        people::face(ui, &s.by, 24.0);
                                        ui.add_space(4.0);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(capitalised(&people::by_line(&s.by, me.as_ref(), "")))
                                                        .font(theme::font_med(theme::T_SM))
                                                        .color(p.text),
                                                );
                                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                    let (txt, col) = if s.words >= 0 {
                                                        (format!("+{} words", s.words), p.ok)
                                                    } else {
                                                        (format!("{} words", s.words), p.text_dim)
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(txt)
                                                            .font(theme::font_mono(theme::T_MICRO))
                                                            .color(col),
                                                    );
                                                });
                                            });
                                            ui.label(
                                                egui::RichText::new(span(s.from, s.to))
                                                    .font(theme::font(theme::T_CAP))
                                                    .color(p.text_faint),
                                            );
                                        });
                                    });
                                    ui.add_space(6.0);
                                }
                            });
                    });
            })
            .response;
        self.close_popover_if_outside(ctx, &[area.rect, self.menu_button_rect]);
    }

    // ---------- recently deleted ----------

    pub(super) fn trash_panel(&mut self, ctx: &egui::Context) {
        let p = pal();
        let screen = ctx.screen_rect();
        let anchor = if self.home { self.account_button_rect } else { self.menu_button_rect };
        let gone = self.store.trash();
        let loading = gone.is_none();
        let gone = gone.unwrap_or_default();
        let me = self.me();
        let mut restore = None;
        let area = egui::Area::new(egui::Id::new("ns-trash"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(
                (anchor.right() - 320.0).max(screen.left() + 8.0),
                anchor.bottom() + 6.0,
            ))
            .show(ctx, |ui| {
                ui::popover_frame()
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(296.0);
                        ui.horizontal(|ui| {
                            icons::show(ui, Icon::Trash, 14.0, p.danger_light);
                            ui.label(
                                egui::RichText::new("Recently deleted")
                                    .font(theme::font_semi(theme::T_H - 1.0))
                                    .color(p.text),
                            );
                        });
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("Deleted scripts wait here for 30 days. Anyone who can write can bring one back.")
                                .font(theme::font(theme::T_CAP))
                                .color(p.text_faint),
                        );
                        ui.add_space(8.0);
                        if gone.is_empty() {
                            ui.label(
                                egui::RichText::new(if loading { "Fetching…" } else { "Nothing has been deleted." })
                                    .font(theme::font(theme::T_SM))
                                    .color(p.text_dim),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for e in &gone {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 1.0;
                                            ui.label(
                                                egui::RichText::new(ui::elide(&e.title, 30))
                                                    .font(theme::font_med(theme::T_SM))
                                                    .color(p.text),
                                            );
                                            let by = e
                                                .edited_by
                                                .as_ref()
                                                .map(|b| people::by_line(b, me.as_ref(), &relative_time(e.modified)))
                                                .unwrap_or_else(|| relative_time(e.modified));
                                            ui.label(
                                                egui::RichText::new(format!("deleted by {by}"))
                                                    .font(theme::font(theme::T_CAP))
                                                    .color(p.text_faint),
                                            );
                                        });
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if self.can_write() && ui::button(ui, "Restore", None, false) {
                                                restore = Some(e.path.clone());
                                            }
                                        });
                                    });
                                    ui.add_space(6.0);
                                }
                            });
                    });
            })
            .response;
        if let Some(path) = restore {
            self.store.restore(&path);
            self.deck.ok("Restored", "It is back in the library.");
        }
        self.close_popover_if_outside(ctx, &[area.rect, anchor]);
    }

    // ---------- the Team tab in Settings ----------

    pub(super) fn team_settings(&mut self, ui: &mut egui::Ui) {
        let Some(team) = self.store.team().cloned() else {
            return;
        };
        let p = pal();
        let owner = team.role == Role::Owner;

        ui::section(ui, "You");
        ui.horizontal(|ui| {
            people::face(ui, &team.me, 40.0);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(&team.me.name)
                        .font(theme::font_semi(theme::T_BODY))
                        .color(p.text),
                );
                ui.label(
                    egui::RichText::new(format!("{} · Discord {}", team.role.label(), team.me.id))
                        .font(theme::font(theme::T_CAP))
                        .color(p.text_faint),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui::button(ui, "Sign out", Some(Icon::SignOut), false) {
                    if self.dirty {
                        self.save(false);
                    }
                    self.store.sign_out();
                }
            });
        });

        ui.add_space(6.0);
        ui::separator(ui);
        ui::section(ui, &format!("{} · {} people", team.name, team.members.len()));
        let mut change: Option<(String, Role)> = None;
        let mut remove: Option<String> = None;
        for m in &team.members {
            ui.horizontal(|ui| {
                people::face(ui, &m.person, 26.0);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let me = m.person.id == team.me.id;
                    let name = if m.pending {
                        "Not signed in yet".to_string()
                    } else if me {
                        format!("{} (you)", m.person.name)
                    } else {
                        m.person.name.clone()
                    };
                    ui.label(
                        egui::RichText::new(name)
                            .font(theme::font_med(theme::T_SM))
                            .color(if m.pending { p.text_dim } else { p.text }),
                    );
                    ui.label(
                        egui::RichText::new(format!("Discord {}", m.person.id))
                            .font(theme::font_mono(theme::T_MICRO))
                            .color(p.text_faint),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let editable = owner && !m.fixed && m.person.id != team.me.id;
                    if editable {
                        if ui::icon_button(ui, Icon::Trash, "Take them off the team", 24.0) {
                            remove = Some(m.person.id.clone());
                        }
                        for r in Role::ALL.iter().rev() {
                            let on = m.role == *r;
                            if ui::button(ui, r.label(), None, on) && !on {
                                change = Some((m.person.id.clone(), *r));
                            }
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(if m.fixed { format!("{} · set by the server", m.role.label()) } else { m.role.label().to_string() })
                                .font(theme::font(theme::T_CAP))
                                .color(p.text_dim),
                        );
                    }
                });
            });
            ui.add_space(4.0);
        }
        if let Some((id, role)) = change {
            self.store.set_role(&id, role);
        }
        if let Some(id) = remove {
            self.store.remove_member(&id);
            self.deck.say("Taken off the team", "They can no longer sign in.", Tone::Warn);
        }

        if owner {
            ui.add_space(6.0);
            ui::separator(ui);
            ui::section(ui, "Add someone");
            ui.label(
                egui::RichText::new("Paste their Discord user ID. In Discord: Settings → Advanced → Developer Mode on, then right-click their name → Copy User ID.")
                    .font(theme::font(theme::T_CAP))
                    .color(p.text_faint),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_add_id)
                        .id(egui::Id::new("ns-team-add"))
                        .hint_text("Discord user ID")
                        .font(theme::font_mono(theme::T_SM))
                        .desired_width(190.0)
                        .margin(egui::Margin::symmetric(10.0, 6.0)),
                );
                for r in [Role::Editor, Role::Viewer] {
                    let on = self.team_add_role == r;
                    if ui::button(ui, r.label(), None, on) && !on {
                        self.team_add_role = r;
                    }
                }
            });
            ui.add_space(4.0);
            let id: String = self.team_add_id.chars().filter(|c| c.is_ascii_digit()).collect();
            let valid = (15..=21).contains(&id.len());
            ui.horizontal(|ui| {
                if ui::button(ui, "Add to the team", Some(Icon::Plus), valid) {
                    if valid {
                        self.store.add_member(&id, self.team_add_role);
                        self.deck.ok(
                            "Added",
                            &format!("{id} can sign in now, as {}.", self.team_add_role.label().to_lowercase()),
                        );
                        self.team_add_id.clear();
                    } else {
                        self.deck.warn("That is not a Discord ID", "A Discord user ID is a long number, 17 to 20 digits.");
                    }
                }
            });
        }
    }

    // ---------- the Details panel ----------

    /// Who saved the script last and everyone who has worked on it.
    pub(super) fn details_people(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.path.clone() else { return };
        if self.store.team().is_none() {
            return;
        }
        let p = pal();
        let me = self.me();
        let entry = self.entries.iter().find(|e| e.path == path).cloned();
        let mut people_seen: Vec<Person> = Vec::new();
        if let Some(sessions) = self.store.history(&path) {
            for s in sessions {
                if !people_seen.contains(&s.by) {
                    people_seen.push(s.by.clone());
                }
            }
        }
        ui.add_space(6.0);
        ui::separator(ui);
        ui::section(ui, "People");
        if let Some(e) = &entry {
            if let Some(by) = &e.edited_by {
                ui.horizontal(|ui| {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("Last saved by")
                            .font(theme::font(theme::T_SM))
                            .color(p.text_dim),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(people::by_line(by, me.as_ref(), &relative_time(e.modified)))
                                .font(theme::font(theme::T_CAP))
                                .color(p.text),
                        );
                        people::face(ui, by, 18.0);
                    });
                });
            }
        }
        if !people_seen.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Worked on by")
                        .font(theme::font(theme::T_SM))
                        .color(p.text_dim),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(2.0);
                    people::faces(ui, &people_seen, 20.0, 6);
                });
            });
        }
        if ui::chip_button(ui, "history…") {
            self.popover = Some(Popover::History);
            self.popover_frame = self.frame_no;
        }
    }

    /// A library row's second line on a team: who has it, or who saved it.
    pub(super) fn team_row_line(&self, e: &crate::storage::Entry, pages: usize) -> Option<(String, bool)> {
        self.store.team()?;
        if let Some(who) = &e.editing {
            return Some((format!("{} is editing", who.first_name()), true));
        }
        let me = self.me();
        let when = relative_time(e.modified);
        Some(match &e.edited_by {
            Some(by) => (format!("{pages} pg · {}", people::by_line(by, me.as_ref(), &when)), false),
            None => (format!("{pages} pg · {when}"), false),
        })
    }

    /// Leave Write and Cards for a script you may only read — and if
    /// anything has changed it all the same, put it back.
    pub(super) fn keep_read_only(&mut self) {
        if !self.access.can_edit() {
            if self.mode != Mode::Read {
                self.mode = Mode::Read;
            }
            self.find.open = false;
            if self.doc != self.baseline {
                self.doc = self.baseline.clone();
                self.ed.fresh = true;
                self.layout_stale = true;
                self.dirty = false;
            }
        }
    }

    /// Is this the script that has just been asked for?
    pub(super) fn is_opening(&self, path: &Path) -> bool {
        self.opening.as_deref() == Some(path)
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// "Today 14:05–14:40", "Yesterday 09:12–09:30", "Mon 3 Mar 14:05–16:10".
fn span(from: SystemTime, to: SystemTime) -> String {
    use chrono::{Datelike, Local, TimeZone};
    let local = |t: SystemTime| {
        let secs = t
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Local.timestamp_opt(secs, 0).single()
    };
    let (Some(a), Some(b)) = (local(from), local(to)) else {
        return String::new();
    };
    let today = Local::now().date_naive();
    let day = if a.date_naive() == today {
        "Today".to_string()
    } else if Some(a.date_naive()) == today.pred_opt() {
        "Yesterday".to_string()
    } else if a.year() == today.year() {
        a.format("%a %-d %b").to_string()
    } else {
        a.format("%-d %b %Y").to_string()
    };
    if b.signed_duration_since(a).num_minutes() < 1 {
        format!("{day} {}", a.format("%H:%M"))
    } else {
        format!("{day} {}–{}", a.format("%H:%M"), b.format("%H:%M"))
    }
}
