//! The application shell around the page.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers, Sense, Stroke};

use crate::editor::{self, Caret, EditorState};
use crate::export::{self, Format};
use crate::model::{Document, Element};
use crate::storage::{self, Entry};
use crate::theme;

const AUTOSAVE_IDLE: Duration = Duration::from_millis(1200);
const SNAPSHOT_IDLE: Duration = Duration::from_millis(700);
const STATUS_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct Snapshot {
    blocks: Vec<crate::model::Block>,
    title: String,
}

#[derive(Default, Clone, Copy)]
struct Stats {
    pages: usize,
    words: usize,
    scenes: usize,
}

pub struct App {
    doc: Document,
    path: Option<PathBuf>,
    entries: Vec<Entry>,
    ed: EditorState,

    search: String,
    show_library: bool,
    show_details: bool,

    dirty: bool,
    last_change: Option<Instant>,
    saved_at: Option<Instant>,

    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    baseline: Snapshot,
    snapshot_due: bool,

    stats: Stats,
    stats_at: Option<Instant>,
    stats_stale: bool,

    status: Option<(String, bool, Instant)>,
    confirm_delete: Option<PathBuf>,
    mono_font: Option<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mono_font = theme::install_fonts(&cc.egui_ctx);
        let _ = storage::ensure_dirs();

        let entries = storage::list_scripts();
        let (doc, path) = match entries.first() {
            Some(e) => match storage::load(&e.path) {
                Ok(d) => (d, Some(e.path.clone())),
                Err(_) => (starter_document(), None),
            },
            None => (starter_document(), None),
        };

        let baseline = Snapshot {
            blocks: doc.blocks.clone(),
            title: doc.meta.title.clone(),
        };

        let mut app = Self {
            doc,
            path,
            entries,
            ed: EditorState::default(),
            search: String::new(),
            show_library: true,
            show_details: false,
            dirty: false,
            last_change: None,
            saved_at: None,
            undo: Vec::new(),
            redo: Vec::new(),
            baseline,
            snapshot_due: false,
            stats: Stats::default(),
            stats_at: None,
            stats_stale: true,
            status: None,
            confirm_delete: None,
            mono_font,
        };

        if app.path.is_none() {
            // first run: put the starter script in the library
            app.path = Some(storage::unique_path(&app.doc.meta.title, None));
            app.save(false);
            app.refresh_entries();
        }
        app
    }

    // ---------- document lifecycle ----------

    fn mark_changed(&mut self) {
        self.dirty = true;
        self.last_change = Some(Instant::now());
        self.snapshot_due = true;
        self.stats_stale = true;
    }

    fn snapshot_now(&mut self) {
        let current = Snapshot {
            blocks: self.doc.blocks.clone(),
            title: self.doc.meta.title.clone(),
        };
        self.undo.push(std::mem::replace(&mut self.baseline, current));
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.snapshot_due = false;
    }

    fn undo(&mut self) {
        if self.snapshot_due {
            self.snapshot_now();
        }
        if let Some(prev) = self.undo.pop() {
            let current = Snapshot {
                blocks: self.doc.blocks.clone(),
                title: self.doc.meta.title.clone(),
            };
            self.redo.push(current);
            self.doc.blocks = prev.blocks.clone();
            self.doc.meta.title = prev.title.clone();
            self.baseline = prev;
            self.doc.ensure_not_empty();
            self.doc.reseed_ids();
            self.dirty = true;
            self.last_change = Some(Instant::now());
            self.stats_stale = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let current = Snapshot {
                blocks: self.doc.blocks.clone(),
                title: self.doc.meta.title.clone(),
            };
            self.undo.push(current);
            self.doc.blocks = next.blocks.clone();
            self.doc.meta.title = next.title.clone();
            self.baseline = next;
            self.doc.ensure_not_empty();
            self.doc.reseed_ids();
            self.dirty = true;
            self.last_change = Some(Instant::now());
            self.stats_stale = true;
        }
    }

    fn save(&mut self, allow_rename: bool) {
        self.doc.normalize();
        let mut path = match self.path.clone() {
            Some(p) => p,
            None => storage::unique_path(&self.doc.meta.title, None),
        };

        if allow_rename {
            let wanted = storage::slugify(&self.doc.meta.title);
            let current_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let stem_matches = current_stem == wanted
                || current_stem
                    .rsplit_once('-')
                    .map(|(head, tail)| head == wanted && tail.parse::<u32>().is_ok())
                    .unwrap_or(false);
            if !stem_matches {
                let new_path = storage::unique_path(&self.doc.meta.title, Some(&path));
                if path.exists() {
                    if std::fs::rename(&path, &new_path).is_ok() {
                        path = new_path;
                    }
                } else {
                    path = new_path;
                }
            }
        }

        match storage::save(&path, &self.doc) {
            Ok(()) => {
                self.path = Some(path);
                self.dirty = false;
                self.saved_at = Some(Instant::now());
                self.refresh_entries();
            }
            Err(e) => self.toast(format!("Could not save: {e}"), true),
        }
    }

    fn open(&mut self, path: PathBuf) {
        if self.dirty {
            self.save(false);
        }
        match storage::load(&path) {
            Ok(doc) => {
                self.doc = doc;
                self.path = Some(path);
                self.baseline = Snapshot {
                    blocks: self.doc.blocks.clone(),
                    title: self.doc.meta.title.clone(),
                };
                self.undo.clear();
                self.redo.clear();
                self.dirty = false;
                self.snapshot_due = false;
                self.stats_stale = true;
                self.ed.focus_block = self.doc.blocks.first().map(|b| b.id);
                self.ed.pending_focus = self.doc.blocks.first().map(|b| (b.id, Caret::End));
            }
            Err(e) => self.toast(format!("Could not open: {e}"), true),
        }
    }

    fn new_script(&mut self) {
        if self.dirty {
            self.save(false);
        }
        let mut doc = Document::default();
        doc.meta.title = "Untitled Script".to_string();
        doc.meta.draft = today();
        self.doc = doc;
        self.path = Some(storage::unique_path("Untitled Script", None));
        self.undo.clear();
        self.redo.clear();
        self.baseline = Snapshot {
            blocks: self.doc.blocks.clone(),
            title: self.doc.meta.title.clone(),
        };
        self.save(false);
        self.refresh_entries();
        self.ed.pending_focus = self.doc.blocks.first().map(|b| (b.id, Caret::Start));
        self.show_details = true;
    }

    fn duplicate(&mut self, path: &PathBuf) {
        if let Ok(mut doc) = storage::load(path) {
            doc.meta.title = format!("{} (copy)", doc.meta.title);
            let new_path = storage::unique_path(&doc.meta.title, None);
            if storage::save(&new_path, &doc).is_ok() {
                self.refresh_entries();
                self.toast("Duplicated".to_string(), false);
            }
        }
    }

    fn delete(&mut self, path: &PathBuf) {
        if storage::delete(path).is_ok() {
            if self.path.as_deref() == Some(path.as_path()) {
                self.path = None;
                self.doc = Document::default();
                self.dirty = false;
            }
            self.refresh_entries();
            self.toast("Deleted".to_string(), false);
        }
    }

    fn refresh_entries(&mut self) {
        self.entries = storage::list_scripts();
    }

    fn toast(&mut self, msg: String, error: bool) {
        self.status = Some((msg, error, Instant::now()));
    }

    fn do_export(&mut self, format: Format) {
        self.doc.normalize();
        match export::export(&self.doc, format) {
            Ok(p) => {
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file")
                    .to_string();
                self.toast(format!("Exported {name} to the exports folder"), false);
                if format == Format::Pdf {
                    storage::open_with_desktop(&p);
                }
            }
            Err(e) => self.toast(format!("Export failed: {e}"), true),
        }
    }

    fn refresh_stats(&mut self) {
        let stale_enough = self
            .stats_at
            .map(|t| t.elapsed() > Duration::from_millis(350))
            .unwrap_or(true);
        if self.stats_stale && stale_enough {
            self.stats = Stats {
                pages: export::page_count(&self.doc),
                words: self.doc.word_count(),
                scenes: self.doc.scene_count(),
            };
            self.stats_at = Some(Instant::now());
            self.stats_stale = false;
        }
    }

    // ---------- shortcuts ----------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let hit = |ctx: &egui::Context, m: Modifiers, k: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)))
        };

        if hit(ctx, Modifiers::COMMAND, Key::S) {
            self.save(true);
            self.toast("Saved".to_string(), false);
        }
        if hit(ctx, Modifiers::COMMAND, Key::N) {
            self.new_script();
        }
        if hit(ctx, Modifiers::COMMAND, Key::Z) {
            self.undo();
        }
        if hit(ctx, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
            || hit(ctx, Modifiers::COMMAND, Key::Y)
        {
            self.redo();
        }
        if hit(ctx, Modifiers::COMMAND, Key::B) {
            self.show_library = !self.show_library;
        }
        if hit(ctx, Modifiers::COMMAND, Key::I) {
            self.show_details = !self.show_details;
        }
        if hit(ctx, Modifiers::COMMAND, Key::E) {
            self.do_export(Format::Pdf);
        }
        if hit(ctx, Modifiers::COMMAND, Key::Plus) || hit(ctx, Modifiers::COMMAND, Key::Equals) {
            self.ed.font_px = (self.ed.font_px + 1.0).min(26.0);
        }
        if hit(ctx, Modifiers::COMMAND, Key::Minus) {
            self.ed.font_px = (self.ed.font_px - 1.0).max(11.0);
        }

        const DIGITS: [Key; 7] = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
        ];
        for (i, k) in DIGITS.iter().enumerate() {
            if hit(ctx, Modifiers::COMMAND, *k) {
                if let Some(e) = Element::from_digit(i + 1) {
                    if editor::set_element(&mut self.doc, &mut self.ed, e) {
                        self.mark_changed();
                    }
                }
            }
        }
    }

    // ---------- panels ----------

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(58.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::RAIL)
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .stroke(Stroke::NONE),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ghost_button(ui, if self.show_library { "‹ Library" } else { "Library" })
                        .clicked()
                    {
                        self.show_library = !self.show_library;
                    }

                    ui.add_space(6.0);
                    let title = if self.doc.meta.title.trim().is_empty() {
                        "Untitled Script".to_string()
                    } else {
                        self.doc.meta.title.clone()
                    };
                    ui.label(
                        egui::RichText::new(title)
                            .size(15.0)
                            .color(theme::TEXT)
                            .strong(),
                    );
                    if self.dirty {
                        ui.label(egui::RichText::new("•").size(18.0).color(theme::RED));
                    }

                    ui.add_space(14.0);
                    let current = self.ed.current_element(&self.doc);
                    let mut wanted: Option<Element> = None;
                    for e in Element::ALL {
                        if element_pill(ui, e, current == Some(e)).clicked() {
                            wanted = Some(e);
                        }
                    }
                    if let Some(e) = wanted {
                        if editor::set_element(&mut self.doc, &mut self.ed, e) {
                            self.mark_changed();
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ghost_button(ui, if self.show_details { "Details ›" } else { "Details" })
                            .clicked()
                        {
                            self.show_details = !self.show_details;
                        }
                        ui.menu_button("Export", |ui| {
                            ui.set_min_width(150.0);
                            for f in [Format::Pdf, Format::Text, Format::Fountain] {
                                if ui.button(f.label()).clicked() {
                                    self.do_export(f);
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui.button("Open exports folder").clicked() {
                                storage::open_with_desktop(&storage::exports_dir());
                                ui.close_menu();
                            }
                        });
                        if accent_button(ui, "New").clicked() {
                            self.new_script();
                        }
                    });
                });
            });
    }

    fn library_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("library")
            .exact_width(276.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::RAIL)
                    .inner_margin(egui::Margin::symmetric(12.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("LIBRARY")
                            .size(10.5)
                            .color(theme::TEXT_FAINT),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("+").size(15.0).color(theme::RED_HOT),
                                )
                                .fill(theme::SURFACE)
                                .rounding(egui::Rounding::same(8.0))
                                .min_size(egui::vec2(28.0, 24.0)),
                            )
                            .on_hover_text("New script (Ctrl+N)")
                            .clicked()
                        {
                            self.new_script();
                        }
                    });
                });
                ui.add_space(8.0);

                let search = egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search")
                    .desired_width(f32::INFINITY)
                    .margin(egui::Margin::symmetric(10.0, 7.0));
                ui.add(search);
                ui.add_space(10.0);

                let needle = self.search.to_lowercase();
                let entries = self.entries.clone();
                let mut to_open: Option<PathBuf> = None;
                let mut to_duplicate: Option<PathBuf> = None;

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for entry in &entries {
                            if !needle.is_empty()
                                && !entry.title.to_lowercase().contains(&needle)
                                && !entry.preview.to_lowercase().contains(&needle)
                            {
                                continue;
                            }
                            let selected = self.path.as_deref() == Some(entry.path.as_path());
                            let resp = library_row(ui, entry, selected);
                            if resp.clicked() && !selected {
                                to_open = Some(entry.path.clone());
                            }
                            resp.context_menu(|ui| {
                                if ui.button("Duplicate").clicked() {
                                    to_duplicate = Some(entry.path.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Show file").clicked() {
                                    storage::open_with_desktop(&storage::scripts_dir());
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .button(egui::RichText::new("Delete").color(theme::RED_HOT))
                                    .clicked()
                                {
                                    self.confirm_delete = Some(entry.path.clone());
                                    ui.close_menu();
                                }
                            });
                            ui.add_space(6.0);
                        }
                    });

                if let Some(p) = to_open {
                    self.open(p);
                }
                if let Some(p) = to_duplicate {
                    self.duplicate(&p);
                }
            });
    }

    fn details_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("details")
            .exact_width(292.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::RAIL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("TITLE PAGE")
                                .size(10.5)
                                .color(theme::TEXT_FAINT),
                        );
                        ui.add_space(8.0);

                        let mut edited = false;
                        edited |= field(ui, "Title", &mut self.doc.meta.title);
                        edited |= field(ui, "Written by", &mut self.doc.meta.author);
                        edited |= field(ui, "Draft", &mut self.doc.meta.draft);
                        edited |= field(ui, "Contact", &mut self.doc.meta.contact);
                        if edited {
                            self.mark_changed();
                        }

                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("SCENES")
                                .size(10.5)
                                .color(theme::TEXT_FAINT),
                        );
                        ui.add_space(6.0);

                        let outline = self.doc.outline();
                        if outline.is_empty() {
                            ui.label(
                                egui::RichText::new("No scene headings yet.")
                                    .size(12.0)
                                    .color(theme::TEXT_FAINT),
                            );
                        }
                        let mut jump: Option<u64> = None;
                        for (n, (id, text)) in outline.iter().enumerate() {
                            let label = if text.trim().is_empty() {
                                format!("{}.  (untitled scene)", n + 1)
                            } else {
                                format!("{}.  {}", n + 1, text)
                            };
                            let resp = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(truncate(&label, 34))
                                        .size(12.0)
                                        .color(theme::TEXT_DIM),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .min_size(egui::vec2(ui.available_width(), 22.0)),
                            );
                            if resp.clicked() {
                                jump = Some(*id);
                            }
                        }
                        if let Some(id) = jump {
                            self.ed.jump_to(id);
                        }

                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("LIBRARY FOLDER")
                                .size(10.5)
                                .color(theme::TEXT_FAINT),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(storage::scripts_dir().display().to_string())
                                .size(11.0)
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(6.0);
                        if ghost_button(ui, "Open in file manager").clicked() {
                            storage::open_with_desktop(&storage::scripts_dir());
                        }

                        if let Some(font) = &self.mono_font {
                            ui.add_space(14.0);
                            ui.label(
                                egui::RichText::new(format!("Page font: {font}"))
                                    .size(10.5)
                                    .color(theme::TEXT_FAINT),
                            );
                        }
                    });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(30.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::RAIL)
                    .inner_margin(egui::Margin::symmetric(14.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let s = self.stats;
                    ui.label(
                        egui::RichText::new(format!(
                            "{} pg   ·   ~{} min   ·   {} scenes   ·   {} words",
                            s.pages, s.pages, s.scenes, s.words
                        ))
                        .size(11.5)
                        .color(theme::TEXT_DIM),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (label, color) = if self.dirty {
                            ("unsaved", theme::RED_HOT)
                        } else if self.saved_at.is_some() {
                            ("saved", theme::TEXT_FAINT)
                        } else {
                            ("", theme::TEXT_FAINT)
                        };
                        ui.label(egui::RichText::new(label).size(11.5).color(color));

                        if let Some((msg, error, at)) = self.status.clone() {
                            if at.elapsed() < STATUS_TTL {
                                ui.add_space(14.0);
                                ui.label(
                                    egui::RichText::new(msg)
                                        .size(11.5)
                                        .color(if error { theme::RED_HOT } else { theme::TEXT_DIM }),
                                );
                            } else {
                                self.status = None;
                            }
                        }
                    });
                });
            });
    }

    fn delete_dialog(&mut self, ctx: &egui::Context) {
        let Some(path) = self.confirm_delete.clone() else {
            return;
        };
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("this script")
            .to_string();

        let mut open = true;
        egui::Window::new("Delete script")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.label(
                    egui::RichText::new(format!("Delete {name}? This cannot be undone."))
                        .size(13.0),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Delete").color(egui::Color32::WHITE),
                            )
                            .fill(theme::RED)
                            .rounding(egui::Rounding::same(theme::R_CTRL)),
                        )
                        .clicked()
                    {
                        self.delete(&path);
                        self.confirm_delete = None;
                    }
                    if ghost_button(ui, "Cancel").clicked() {
                        self.confirm_delete = None;
                    }
                });
            });
        if !open {
            self.confirm_delete = None;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.shortcuts(ctx);
        self.refresh_stats();

        self.top_bar(ctx);
        self.status_bar(ctx);
        if self.show_library {
            self.library_panel(ctx);
        }
        if self.show_details {
            self.details_panel(ctx);
        }

        let mut changed = false;
        {
            let doc = &mut self.doc;
            let ed = &mut self.ed;
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(theme::BG))
                .show(ctx, |ui| {
                    changed = editor::show(ui, doc, ed);
                });
        }
        if changed {
            self.mark_changed();
        }

        self.delete_dialog(ctx);

        // undo checkpoints and autosave, both keyed off a pause in typing
        if let Some(last) = self.last_change {
            if self.snapshot_due && last.elapsed() > SNAPSHOT_IDLE {
                self.snapshot_now();
            }
            if self.dirty && last.elapsed() > AUTOSAVE_IDLE {
                self.save(false);
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save(true);
        }

        ctx.request_repaint_after(Duration::from_millis(400));
    }
}

// ---------- small widgets ----------

fn ghost_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).size(12.5).color(theme::TEXT_DIM))
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::LINE))
            .rounding(egui::Rounding::same(theme::R_CTRL)),
    )
}

fn accent_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(text)
                .size(12.5)
                .color(egui::Color32::WHITE)
                .strong(),
        )
        .fill(theme::RED)
        .stroke(Stroke::new(1.0, theme::RED_HOT))
        .rounding(egui::Rounding::same(theme::R_CTRL)),
    )
}

fn element_pill(ui: &mut egui::Ui, e: Element, selected: bool) -> egui::Response {
    let color = theme::element_color(e);
    let (fill, stroke, text_color) = if selected {
        (theme::RED_WASH, Stroke::new(1.0, color), color)
    } else {
        (theme::SURFACE, Stroke::new(1.0, theme::LINE), theme::TEXT_DIM)
    };
    ui.add(
        egui::Button::new(egui::RichText::new(e.short()).size(10.5).color(text_color))
            .fill(fill)
            .stroke(stroke)
            .rounding(egui::Rounding::same(theme::R_PILL))
            .min_size(egui::vec2(0.0, 26.0)),
    )
    .on_hover_text(format!(
        "{}  (Ctrl+{})",
        e.label(),
        Element::ALL.iter().position(|x| *x == e).unwrap_or(0) + 1
    ))
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    ui.label(egui::RichText::new(label).size(11.0).color(theme::TEXT_FAINT));
    ui.add_space(2.0);
    let r = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(10.0, 7.0)),
    );
    ui.add_space(10.0);
    r.changed()
}

fn library_row(ui: &mut egui::Ui, entry: &Entry, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 58.0), Sense::click());

    let fill = if selected {
        theme::RED_WASH
    } else if resp.hovered() {
        theme::SURFACE_HI
    } else {
        theme::SURFACE
    };
    let stroke = if selected {
        Stroke::new(1.0, theme::RED)
    } else {
        Stroke::new(1.0, theme::LINE)
    };
    let painter = ui.painter();
    painter.rect(rect, egui::Rounding::same(12.0), fill, stroke);

    let pad = 12.0;
    painter.text(
        egui::pos2(rect.left() + pad, rect.top() + 14.0),
        egui::Align2::LEFT_CENTER,
        truncate(&entry.title, 26),
        egui::FontId::proportional(13.0),
        if selected { theme::TEXT } else { theme::TEXT },
    );
    painter.text(
        egui::pos2(rect.left() + pad, rect.top() + 33.0),
        egui::Align2::LEFT_CENTER,
        truncate(&entry.preview, 32),
        egui::FontId::proportional(11.0),
        theme::TEXT_FAINT,
    );
    painter.text(
        egui::pos2(rect.right() - pad, rect.bottom() - 12.0),
        egui::Align2::RIGHT_CENTER,
        relative_time(entry.modified),
        egui::FontId::proportional(10.5),
        theme::TEXT_FAINT,
    );
    resp
}

fn truncate(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{head}…")
    }
}

fn relative_time(t: SystemTime) -> String {
    let Ok(d) = SystemTime::now().duration_since(t) else {
        return "now".to_string();
    };
    let s = d.as_secs();
    if s < 90 {
        "just now".to_string()
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else if s < 86_400 {
        format!("{}h ago", s / 3600)
    } else {
        format!("{}d ago", s / 86_400)
    }
}

fn today() -> String {
    chrono::Local::now().format("%B %e, %Y").to_string().replace("  ", " ")
}

fn starter_document() -> Document {
    let mut doc = Document::default();
    doc.blocks.clear();
    doc.meta.title = "Untitled Script".to_string();
    doc.meta.draft = today();
    doc.push(Element::SceneHeading, "INT. EDIT BAY - NIGHT");
    doc.push(
        Element::Action,
        "Monitors glow. Coffee gone cold. A cursor blinks in an empty document.",
    );
    doc.push(Element::Character, "THE WRITER");
    doc.push(Element::Parenthetical, "(to no one)");
    doc.push(Element::Dialogue, "Alright. Page one.");
    doc.push(Element::Transition, "SMASH CUT TO:");
    doc.reseed_ids();
    doc
}
