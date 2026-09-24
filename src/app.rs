//! The shell around the page.
//!
//! Islands on a blurred backdrop, exactly as Tesseract lays them out: the
//! title bar and ribbon float on the glass, and the library rail, the script
//! page and the scene navigator are solid panes with a gap of backdrop between
//! them. Everything the user can destroy is asked about first, and everything
//! says so afterwards through the deck at the bottom of the window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{self, Align, Color32, Layout, Pos2, Rect, Sense, Stroke, Vec2};

use crate::alerts::{self, Tone};
use crate::anim;
use crate::blur;
use crate::caret;
use crate::cards::{self, CardsState};
use crate::chrome;
use crate::editor::{self, Caret, EditorState};
use crate::export::{self, Format};
use crate::icons::{self, Icon};
use crate::logo;
use crate::model::{eighths_label, CastMember, Document, Element};
use crate::pages::{self, PagesState};
use crate::settings::{AfterExport, BlurMode, Settings, SortBy};
use crate::splash::Splash;
use crate::storage::{self, Entry};
use crate::theme::{self, pal};
use crate::ui;

const SNAPSHOT_IDLE: Duration = Duration::from_millis(700);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Write,
    Cards,
    /// Reading mode: the script as it will print, and nothing else.
    Read,
}

impl Mode {
    fn index(self) -> usize {
        match self {
            Mode::Write => 0,
            Mode::Cards => 1,
            Mode::Read => 2,
        }
    }
    fn from_index(i: usize) -> Mode {
        match i {
            1 => Mode::Cards,
            2 => Mode::Read,
            _ => Mode::Write,
        }
    }
}

/// Everything the app will only do once you have said yes.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    DeleteScript(PathBuf),
    DeleteScene(usize),
    RenameScript,
    RestoreSnapshot(PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Popover {
    Menu,
    Settings,
    Snapshots,
    Elements,
}

/// What the printed layout says about the script, worked out once whenever it
/// changes rather than every frame.
#[derive(Default)]
struct Layout2 {
    /// Block id → the page that starts there.
    page_starts: HashMap<u64, usize>,
    /// Heading id → (page it starts on, length in eighths).
    lengths: HashMap<u64, (usize, usize)>,
    pages: usize,
    words: usize,
    scenes: usize,
    cast: Vec<CastMember>,
    /// Everyone who speaks, in order of first appearance — the order their
    /// colours are dealt out in.
    speakers: Vec<String>,
    dialogue: f32,
}

#[derive(Default)]
struct Rail {
    /// Where each row was last drawn, so a deleted one can be seen to go.
    last_rects: HashMap<PathBuf, Rect>,
    /// A script on its way out: what it said, where it was, and when it went.
    ghost: Option<(String, Rect, Instant)>,
    starred_collapsed: bool,
    all_collapsed: bool,
    /// A row's right-click menu: which script, where, and the frame it opened.
    menu: Option<(PathBuf, Pos2, u64)>,
}

#[derive(Default)]
struct Find {
    open: bool,
    needle: String,
    with: String,
    /// Which match is current, as an index into this frame's matches.
    current: usize,
    focus: bool,
    rect: Option<Rect>,
}

pub struct App {
    doc: Document,
    path: Option<PathBuf>,
    entries: Vec<Entry>,
    ed: EditorState,
    cards: CardsState,
    pages: PagesState,
    mode: Mode,

    search: String,
    focus_search: bool,
    focus_title: bool,
    focus_mode: bool,

    settings: Settings,
    theme_dirty: bool,
    blur: blur::Blur,
    /// When Tesseract's settings were last seen to change, and when we looked.
    tess_seen: Option<SystemTime>,
    tess_checked: Option<Instant>,

    dirty: bool,
    last_change: Option<Instant>,
    saved_at: Option<Instant>,

    undo: Vec<Document>,
    redo: Vec<Document>,
    baseline: Document,
    snapshot_due: bool,

    layout: Layout2,
    layout_at: Option<Instant>,
    layout_stale: bool,
    session_words: i64,
    words_seen: Option<(PathBuf, usize)>,
    /// Which of each character's cues was jumped to last.
    cast_cursor: HashMap<String, usize>,

    deck: alerts::Deck<Ask>,
    popover: Option<Popover>,
    popover_frame: u64,
    /// Which category the Settings window is showing.
    settings_tab: usize,
    menu_button_rect: Rect,
    settings_button_rect: Rect,
    element_button_rect: Rect,
    menu_item_rects: Vec<Rect>,
    page_rect: Rect,
    foot_rect: Rect,
    aside_rect: Rect,
    /// Where each island down the right was actually drawn this frame.
    aside_islands: Vec<Rect>,

    rail: Rail,
    find: Find,

    frame_no: u64,
    page_key: String,
    page_born: Instant,
    splash: Option<Splash>,
    importing: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    /// The scripts folder's own modification time, so a script dropped in
    /// from outside shows up in the library without waiting for a save.
    library_seen: Option<(SystemTime, Instant)>,
    /// Files named on the command line — "Open with Northstar" — handled on
    /// the first frame.
    pub arrivals: Vec<PathBuf>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App::with_context(&cc.egui_ctx);
        app.blur = blur::Blur::install(cc);
        // Worth one line on stderr: when the islands look wrong, this is why.
        eprintln!(
            "northstar: {} — window blur {}",
            blur::session_name(),
            app.blur.state.describe()
        );
        app.sync_glass();
        app
    }

    /// The real constructor; the headless UI tests build an App from here.
    pub fn with_context(ctx: &egui::Context) -> Self {
        let _ = storage::ensure_dirs();

        let mut settings = storage::read_settings();
        let mut tess_seen = None;
        if settings.match_tesseract {
            if let Some((t, when)) = storage::read_tesseract_settings() {
                settings.adopt_look(&t);
                tess_seen = Some(when);
            }
        }
        theme::set_palette(settings.theme, !settings.light_mode);
        theme::set_glass_opacity(settings.glass_opacity);
        anim::set_enabled(settings.animations);
        theme::apply(ctx);
        theme::install_fonts(ctx);
        ctx.set_zoom_factor(settings.font_scale);

        let entries = storage::list_scripts();
        let (doc, path) = match entries.first() {
            Some(e) => match storage::load(&e.path) {
                Ok(d) => (d, Some(e.path.clone())),
                Err(_) => (starter_document(), None),
            },
            None => (starter_document(), None),
        };

        let splash = if settings.splash {
            Some(Splash::default())
        } else {
            None
        };
        let mut ed = EditorState::default();
        ed.font_px = settings.page_px;

        let mut app = App {
            baseline: doc.clone(),
            doc,
            path,
            entries,
            ed,
            cards: CardsState::default(),
            pages: PagesState::default(),
            mode: Mode::Write,
            search: String::new(),
            focus_search: false,
            focus_title: false,
            focus_mode: false,
            settings,
            theme_dirty: false,
            blur: blur::Blur::default(),
            tess_seen,
            tess_checked: None,
            dirty: false,
            last_change: None,
            saved_at: None,
            undo: Vec::new(),
            redo: Vec::new(),
            snapshot_due: false,
            layout: Layout2::default(),
            layout_at: None,
            layout_stale: true,
            session_words: 0,
            words_seen: None,
            cast_cursor: HashMap::new(),
            deck: alerts::Deck::default(),
            popover: None,
            popover_frame: 0,
            settings_tab: 0,
            menu_button_rect: Rect::NOTHING,
            settings_button_rect: Rect::NOTHING,
            element_button_rect: Rect::NOTHING,
            menu_item_rects: Vec::new(),
            page_rect: Rect::NOTHING,
            foot_rect: Rect::NOTHING,
            aside_rect: Rect::NOTHING,
            aside_islands: Vec::new(),
            rail: Rail::default(),
            find: Find::default(),
            frame_no: 0,
            page_key: String::new(),
            page_born: Instant::now(),
            splash,
            importing: None,
            library_seen: None,
            arrivals: Vec::new(),
        };

        if app.path.is_none() {
            // first run: put the starter script in the library
            app.path = Some(storage::unique_path(&app.doc.meta.title, None));
            app.save(false);
            app.refresh_entries();
        }
        // the caret starts where the script does, as it does on opening one
        app.ed.focus_block = app.doc.blocks.first().map(|b| b.id);
        app.ed.pending_focus = app.doc.blocks.first().map(|b| (b.id, Caret::End));
        app.refresh_layout(true);
        app
    }

    // ---------- document lifecycle ----------

    fn mark_changed(&mut self) {
        self.dirty = true;
        self.last_change = Some(Instant::now());
        self.snapshot_due = true;
        self.layout_stale = true;
    }

    fn snapshot_now(&mut self) {
        let current = self.doc.clone();
        self.undo.push(std::mem::replace(&mut self.baseline, current));
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.snapshot_due = false;
    }

    /// A checkpoint before a change made in one go (a scene moved, a replace).
    fn checkpoint(&mut self) {
        if self.snapshot_due {
            self.snapshot_now();
        }
        self.undo.push(self.doc.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn after_history(&mut self) {
        self.doc.ensure_not_empty();
        self.doc.reseed_ids();
        self.dirty = true;
        self.last_change = Some(Instant::now());
        self.layout_stale = true;
        self.ed.fresh = true;
    }

    fn undo(&mut self) {
        if self.snapshot_due {
            self.snapshot_now();
        }
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(self.doc.clone());
                self.doc = prev.clone();
                self.baseline = prev;
                self.after_history();
            }
            None => self.deck.say("Nothing to undo", "", Tone::Info),
        }
    }

    fn redo(&mut self) {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(self.doc.clone());
                self.doc = next.clone();
                self.baseline = next;
                self.after_history();
            }
            None => self.deck.say("Nothing to redo", "", Tone::Info),
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
                        storage::follow_rename(&path, &new_path);
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
            Err(e) => self.deck.say("Could not save", &e.to_string(), Tone::Danger),
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
                self.baseline = self.doc.clone();
                self.undo.clear();
                self.redo.clear();
                self.dirty = false;
                self.snapshot_due = false;
                self.layout_stale = true;
                self.words_seen = None;
                self.find.current = 0;
                self.ed.fresh = true;
                self.ed.focus_block = self.doc.blocks.first().map(|b| b.id);
                if self.mode == Mode::Write {
                    self.ed.pending_focus = self.doc.blocks.first().map(|b| (b.id, Caret::End));
                }
                self.refresh_layout(true);
            }
            Err(e) => self.deck.say("Could not open", &e.to_string(), Tone::Danger),
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
        self.baseline = self.doc.clone();
        self.save(false);
        self.refresh_entries();
        self.ed.fresh = true;
        self.words_seen = None;
        self.mode = Mode::Write;
        // name it first: the title is selected, ready to be typed over
        self.focus_title = true;
        self.refresh_layout(true);
        self.deck.ok("Script created", "Name it, then press Enter to start writing.");
    }

    fn duplicate(&mut self, path: &Path) {
        if self.path.as_deref() == Some(path) && self.dirty {
            self.save(false);
        }
        match storage::load(path) {
            Ok(mut doc) => {
                doc.meta.title = format!("{} (copy)", doc.meta.title);
                doc.meta.starred = false;
                let new_path = storage::unique_path(&doc.meta.title, None);
                match storage::save(&new_path, &doc) {
                    Ok(()) => {
                        self.refresh_entries();
                        self.deck.ok("Duplicated", &doc.meta.title);
                    }
                    Err(e) => self.deck.say("Could not duplicate", &e.to_string(), Tone::Danger),
                }
            }
            Err(e) => self.deck.say("Could not duplicate", &e.to_string(), Tone::Danger),
        }
    }

    fn delete(&mut self, path: &Path) {
        let title = self
            .entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.title.clone())
            .unwrap_or_default();
        // leave something behind to watch go
        if let Some(r) = self.rail.last_rects.get(path).copied() {
            self.rail.ghost = Some((title.clone(), r, Instant::now()));
        }
        match storage::delete(path) {
            Ok(()) => {
                if self.path.as_deref() == Some(path) {
                    self.dirty = false;
                    self.refresh_entries();
                    match self.entries.first().map(|e| e.path.clone()) {
                        Some(next) => self.open(next),
                        None => {
                            self.path = None;
                            self.doc = starter_document();
                            self.path = Some(storage::unique_path(&self.doc.meta.title, None));
                            self.save(false);
                            self.ed.fresh = true;
                        }
                    }
                }
                self.refresh_entries();
                self.deck.say("Script deleted", &title, Tone::Warn);
            }
            Err(e) => self.deck.say("Could not delete", &e.to_string(), Tone::Danger),
        }
    }

    fn toggle_star(&mut self, path: &Path) {
        if self.path.as_deref() == Some(path) {
            self.doc.meta.starred = !self.doc.meta.starred;
            self.save(false);
            return;
        }
        if let Ok(mut doc) = storage::load(path) {
            doc.meta.starred = !doc.meta.starred;
            if storage::save(path, &doc).is_ok() {
                self.refresh_entries();
            }
        }
    }

    fn refresh_entries(&mut self) {
        self.entries = storage::list_scripts();
        if let Ok(m) = std::fs::metadata(storage::scripts_dir()).and_then(|m| m.modified()) {
            self.library_seen = Some((m, Instant::now()));
        }
    }

    /// Look at the scripts folder now and then; if anything was added,
    /// removed or renamed there from outside, list it again.
    fn watch_library(&mut self) {
        if let Some((_, at)) = self.library_seen {
            if at.elapsed() < Duration::from_millis(1500) {
                return;
            }
        }
        let now = std::fs::metadata(storage::scripts_dir()).and_then(|m| m.modified()).ok();
        match (now, self.library_seen) {
            (Some(m), Some((seen, _))) if m == seen => {
                self.library_seen = Some((m, Instant::now()));
            }
            _ => self.refresh_entries(),
        }
    }

    /// Work the printed layout out again, if anything has changed since.
    fn refresh_layout(&mut self, now: bool) {
        let stale_enough = self
            .layout_at
            .map(|t| t.elapsed() > Duration::from_millis(350))
            .unwrap_or(true);
        if !(self.layout_stale && (stale_enough || now)) {
            return;
        }
        let starts = export::page_starts(&self.doc);
        let lengths = export::scene_lengths(&self.doc);
        self.layout = Layout2 {
            page_starts: starts.into_iter().map(|(n, id)| (id, n)).collect(),
            pages: export::page_count(&self.doc),
            lengths: lengths.into_iter().map(|(id, pg, e)| (id, (pg, e))).collect(),
            words: self.doc.word_count(),
            scenes: self.doc.scene_count(),
            cast: self.doc.cast(),
            speakers: self.doc.speakers(),
            dialogue: self.doc.dialogue_share(),
        };
        // the session counts what was written, in whichever script
        if let Some(path) = self.path.clone() {
            match &self.words_seen {
                Some((p, before)) if *p == path => {
                    self.session_words += self.layout.words as i64 - *before as i64;
                }
                _ => {}
            }
            self.words_seen = Some((path, self.layout.words));
        }
        self.layout_at = Some(Instant::now());
        self.layout_stale = false;
    }

    fn export(&mut self, format: Format, quick: bool) {
        self.doc.normalize();
        match export::export_with(&self.doc, format, self.settings.scene_numbers) {
            Ok(p) => {
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file")
                    .to_string();
                if format == Format::Pdf && quick {
                    match self.settings.after_export {
                        AfterExport::Open => storage::open_with_desktop(&p),
                        AfterExport::Reveal => storage::reveal_in_file_manager(&p),
                        AfterExport::Nothing => {}
                    }
                }
                self.deck.ok("Exported", &name);
            }
            Err(e) => self.deck.say("Export failed", &e, Tone::Danger),
        }
    }

    fn import_path(&mut self, path: &Path) {
        match storage::import_file(path) {
            Ok(doc) => {
                if self.dirty {
                    self.save(false);
                }
                let new_path = storage::unique_path(&doc.meta.title, None);
                match storage::save(&new_path, &doc) {
                    Ok(()) => {
                        self.refresh_entries();
                        let title = doc.meta.title.clone();
                        self.open(new_path);
                        self.mode = Mode::Write;
                        self.deck.ok(
                            "Imported",
                            &format!("{title} — {} scenes", self.layout.scenes),
                        );
                    }
                    Err(e) => self.deck.say("Could not import", &e.to_string(), Tone::Danger),
                }
            }
            Err(e) => self.deck.say("Could not import", &e, Tone::Danger),
        }
    }

    fn start_import(&mut self) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(storage::pick_file_to_import());
        });
        self.importing = Some(rx);
    }

    fn take_snapshot(&mut self) {
        let Some(path) = self.path.clone() else { return };
        self.doc.normalize();
        match storage::take_snapshot(&path, &self.doc) {
            Ok(_) => self.deck.ok("Snapshot taken", "Find it under More → Snapshots."),
            Err(e) => self.deck.say("Could not take a snapshot", &e.to_string(), Tone::Danger),
        }
    }

    fn restore_snapshot(&mut self, snap: &Path) {
        let Some(path) = self.path.clone() else { return };
        match storage::load(snap) {
            Ok(old) => {
                // what is there now is kept first, so a restore loses nothing
                self.doc.normalize();
                let _ = storage::take_snapshot(&path, &self.doc);
                self.checkpoint();
                let title = self.doc.meta.title.clone();
                self.doc = old;
                self.doc.meta.title = title;
                self.after_history();
                self.save(false);
                self.refresh_layout(true);
                self.deck.ok("Snapshot restored", "The version before it was kept too.");
            }
            Err(e) => self.deck.say("Could not restore", &e.to_string(), Tone::Danger),
        }
    }

    /// A new scene heading straight after the scene the caret is in.
    fn new_scene(&mut self) {
        self.checkpoint();
        let at = self
            .ed
            .focus_block
            .and_then(|id| self.doc.scene_of(id))
            .and_then(|k| self.doc.scenes().get(k).map(|s| s.end))
            .unwrap_or(self.doc.blocks.len());
        let b = self.doc.new_block(Element::SceneHeading, "");
        let id = b.id;
        self.doc.blocks.insert(at, b);
        self.mode = Mode::Write;
        self.ed.focus(id, Caret::Start);
        self.ed.scroll_to_focus = true;
        self.mark_changed();
        self.snapshot_due = false;
        self.baseline = self.doc.clone();
    }

    fn sync_glass(&mut self) {
        // If the compositor could not blur, translucency would show the desktop
        // rather than a soft version of it, so the glass goes nearly solid.
        let want = match self.settings.blur {
            BlurMode::Off => 1.0,
            BlurMode::Always => self.settings.glass_opacity,
            BlurMode::Auto => {
                if self.blur.state.is_on() {
                    self.settings.glass_opacity
                } else {
                    0.97
                }
            }
        };
        theme::set_glass_opacity(want);
    }

    /// What is behind the islands: the backdrop, at whatever alpha the window
    /// is being painted with.
    fn window_ground(&self) -> Color32 {
        let p = pal();
        if !self.transparent_window() {
            return p.backdrop;
        }
        // On a light page the desk has to stay a shade darker than the cards
        // laid on it, so the glass is denser there than it is on a dark one.
        let g = theme::glass_opacity();
        let a = if p.dark { g * 235.0 } else { (0.78 + 0.22 * g) * 252.0 };
        theme::wash(p.backdrop, a as u8)
    }

    fn transparent_window(&self) -> bool {
        match self.settings.blur {
            BlurMode::Off => false,
            BlurMode::Always => true,
            BlurMode::Auto => self.blur.state.is_on(),
        }
    }

    fn save_settings(&mut self) {
        if let Err(e) = storage::write_settings(&self.settings) {
            self.deck
                .say("Could not save settings", &e.to_string(), Tone::Danger);
        }
    }

    /// Follow Tesseract's look, when asked to, whenever its settings change.
    fn follow_tesseract(&mut self) {
        if !self.settings.match_tesseract {
            return;
        }
        if self
            .tess_checked
            .map(|t| t.elapsed() < Duration::from_millis(1500))
            .unwrap_or(false)
        {
            return;
        }
        self.tess_checked = Some(Instant::now());
        if let Some((t, when)) = storage::read_tesseract_settings() {
            if self.tess_seen != Some(when) {
                self.tess_seen = Some(when);
                if self.settings.adopt_look(&t) {
                    self.theme_dirty = true;
                }
            }
        }
    }

    fn answer(&mut self, a: alerts::Answer<Ask>) {
        match a.action {
            Ask::DeleteScript(path) => self.delete(&path),
            Ask::DeleteScene(n) => {
                let heading = self
                    .doc
                    .scenes()
                    .get(n)
                    .map(|s| s.heading.clone())
                    .unwrap_or_default();
                self.checkpoint();
                if self.doc.remove_scene(n) {
                    self.mark_changed();
                    self.snapshot_due = false;
                    self.baseline = self.doc.clone();
                    self.deck.say("Scene deleted", &format!("{heading} · Ctrl+Z brings it back"), Tone::Warn);
                }
            }
            Ask::RenameScript => {
                if let Some(t) = a.text {
                    if !t.trim().is_empty() {
                        self.doc.meta.title = t.trim().to_string();
                        self.mark_changed();
                        self.save(true);
                        self.deck.ok("Renamed", t.trim());
                    }
                }
            }
            Ask::RestoreSnapshot(p) => self.restore_snapshot(&p),
        }
    }

    fn ask_delete(&mut self, path: PathBuf) {
        let title = self
            .entries
            .iter()
            .find(|e| e.path == path)
            .map(|e| e.title.clone())
            .unwrap_or_else(|| "this script".into());
        self.deck.ask(
            "Delete this script?",
            &format!("“{title}” goes for good. Its snapshots stay in the library folder."),
            "Delete",
            Tone::Danger,
            Ask::DeleteScript(path),
        );
    }

    fn ask_delete_scene(&mut self, n: usize) {
        let heading = self.doc.scenes().get(n).map(|s| s.heading.clone()).unwrap_or_default();
        self.deck.ask(
            "Delete this scene?",
            &format!(
                "“{}” and everything in it. Ctrl+Z brings it back.",
                if heading.trim().is_empty() { "Untitled scene" } else { heading.trim() }
            ),
            "Delete",
            Tone::Danger,
            Ask::DeleteScene(n),
        );
    }

    fn ask_restore(&mut self, s: &storage::Snapshot) {
        self.deck.ask(
            "Restore this snapshot?",
            &format!(
                "The script as it was at {}. What is there now is kept as a snapshot first, so nothing is lost.",
                s.when
            ),
            "Restore",
            Tone::Warn,
            Ask::RestoreSnapshot(s.path.clone()),
        );
    }

    fn toggle_popover(&mut self, which: Popover) {
        if self.popover == Some(which) {
            self.popover = None;
        } else {
            self.popover = Some(which);
            self.popover_frame = self.frame_no;
        }
    }

    fn close_popover_if_outside(&mut self, ctx: &egui::Context, areas: &[Rect]) {
        let d = ui::Dismisser {
            opened_on: self.popover_frame,
        };
        if d.should_close(ctx, self.frame_no, areas) {
            self.popover = None;
        }
    }

    /// Each speaker's colour, if characters are wearing colours at all.
    fn voices(&self) -> Option<HashMap<String, Color32>> {
        if self.settings.character_colors {
            Some(theme::character_colors(&self.layout.speakers, self.settings.colour_seed))
        } else {
            None
        }
    }

    fn title_or_untitled(&self) -> String {
        if self.doc.meta.title.trim().is_empty() {
            "Untitled Script".to_string()
        } else {
            self.doc.meta.title.clone()
        }
    }

    // ---------- the ribbon ----------

    fn top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("ns-ribbon")
            .exact_height(theme::TOPBAR_H)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: theme::GAP,
                right: theme::GAP,
                top: 5.0,
                bottom: 0.0,
            }))
            .show_separator_line(false)
            .show(ctx, |ui| {
                chrome::title_bar(ui, ctx);
                ui.add_space(6.0);
                self.ribbon_row(ui);
            });
    }

    /// The ribbon sits straight on the window's glass — no plates, no boxes.
    /// A control is only visible once you reach for it.
    fn ribbon_row(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if let Some(picked) = ui::segmented(
                ui,
                &[
                    ("Write", Icon::Pencil),
                    ("Cards", Icon::Grid),
                    ("Read", Icon::Book),
                ],
                self.mode.index(),
            ) {
                self.mode = Mode::from_index(picked);
            }

            ui.add_space(6.0);
            ui::rule(ui, 22.0);
            ui.add_space(6.0);

            // what is on the right is laid out first, so the tools know their room
            let right_w = 247.0 + if self.saved_label().is_some() { 96.0 } else { 0.0 };
            let room = (ui.available_width() - right_w).max(0.0);
            let tools = Rect::from_min_size(ui.cursor().min, Vec2::new(room, 30.0));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(tools)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            child.set_clip_rect(tools.expand2(Vec2::new(0.0, 20.0)).intersect(ui.clip_rect()));
            match self.mode {
                Mode::Write => self.write_tools(&mut child, room),
                Mode::Cards => self.cards_tools(&mut child, room),
                Mode::Read => self.read_tools(&mut child, room),
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 3.0;

                let (rect, menu) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::click());
                self.menu_button_rect = rect;
                let open = matches!(self.popover, Some(Popover::Menu) | Some(Popover::Snapshots));
                ui::ghost_slot(ui, rect, menu.hovered() || open, open);
                icons::draw(
                    ui.painter(),
                    rect.shrink(9.0),
                    Icon::More,
                    if open { p.primary_light } else { p.text_dim },
                );
                if menu.on_hover_text("More actions").clicked() {
                    self.toggle_popover(Popover::Menu);
                }

                let (srect, sresp) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::click());
                self.settings_button_rect = srect;
                let sopen = self.popover == Some(Popover::Settings);
                ui::ghost_slot(ui, srect, sresp.hovered() || sopen, sopen);
                icons::draw(
                    ui.painter(),
                    srect.shrink(8.0),
                    Icon::Settings,
                    if sopen { p.primary_light } else { p.text_dim },
                );
                if sresp
                    .on_hover_text(format!("Settings · {} · Ctrl+,", pal().id.name()))
                    .clicked()
                {
                    self.toggle_popover(Popover::Settings);
                }

                ui.add_space(4.0);
                ui::rule(ui, 20.0);
                ui.add_space(4.0);

                if ui::ribbon_button(ui, Icon::Eye, "", "Focus — just the scene you are in · Ctrl+.", self.focus_mode) {
                    self.focus_mode = !self.focus_mode;
                }
                if ui::ribbon_button(ui, Icon::Info, "", "Details — title page and figures · Ctrl+I", self.settings.show_details) {
                    self.settings.show_details = !self.settings.show_details;
                    self.save_settings();
                }
                if ui::ribbon_button(ui, Icon::Users, "", "Cast", self.settings.show_cast) {
                    self.settings.show_cast = !self.settings.show_cast;
                    self.save_settings();
                }
                if ui::ribbon_button(ui, Icon::List, "", "Scenes · Ctrl+Shift+I", self.settings.show_scenes) {
                    self.settings.show_scenes = !self.settings.show_scenes;
                    self.save_settings();
                }
                if ui::ribbon_button(ui, Icon::Sidebar, "", "Library · Ctrl+B", self.settings.show_library) {
                    self.settings.show_library = !self.settings.show_library;
                    self.save_settings();
                }

                if let Some((text, col)) = self.saved_label() {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(text)
                            .font(theme::font_mono(theme::T_MICRO))
                            .color(col),
                    );
                }
            });
        });
    }

    fn saved_label(&self) -> Option<(String, Color32)> {
        let p = pal();
        if self.dirty {
            Some(("unsaved".to_string(), p.sec_light))
        } else {
            self.saved_at
                .map(|t| (format!("saved {}", ago_instant(t)), p.text_faint))
        }
    }

    fn stats_line(&self, ui: &mut egui::Ui, room: f32, with_scenes: bool) {
        let p = pal();
        let l = &self.layout;
        let s = |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let mut long = format!(
            "{} · ~{} min · {} · {} words",
            s(l.pages, "page", "pages"),
            l.pages,
            s(l.scenes, "scene", "scenes"),
            l.words
        );
        if !with_scenes {
            long = format!("{} · ~{} min", s(l.pages, "page", "pages"), l.pages);
        }
        let session = if self.session_words > 0 {
            format!(" · +{} this session", self.session_words)
        } else {
            String::new()
        };
        let longer = format!("{long}{session}");
        let mid = format!("{} pg · {} sc · {} w", l.pages, l.scenes, l.words);
        let short = format!("{} pg", l.pages);
        let goal = self.settings.session_goal;
        let bar_w = if goal > 0 { 52.0 } else { 0.0 };
        let text = ui::pick_that_fits(ui, &[&longer, &long, &mid, &short], room - bar_w - 8.0);
        if !text.is_empty() {
            ui.label(
                egui::RichText::new(text)
                    .font(theme::font_mono(theme::T_CAP))
                    .color(p.text_faint),
            );
        }
        if goal > 0 && room > bar_w + 20.0 {
            // the session's goal as a thin track filling with the primary
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(46.0, 14.0), Sense::hover());
            let t = (self.session_words.max(0) as f32 / goal as f32).clamp(0.0, 1.0);
            let track = Rect::from_center_size(rect.center(), Vec2::new(46.0, 5.0));
            ui.painter()
                .rect_filled(track, egui::Rounding::same(3.0), theme::wash(p.text_faint, 55));
            if t > 0.0 {
                let fill = Rect::from_min_max(track.min, Pos2::new(track.left() + track.width() * t, track.bottom()));
                theme::grad_rect(ui.painter(), fill, 3.0, p.prim_grad.0, p.prim_grad.1, Vec2::new(1.0, 0.0));
            }
            resp.on_hover_text(format!(
                "Session goal: {} of {goal} words",
                self.session_words.max(0)
            ));
        }
    }

    fn write_tools(&mut self, ui: &mut egui::Ui, room: f32) {
        let p = pal();
        ui.spacing_mut().item_spacing.x = 3.0;
        let current = self.ed.current_element(&self.doc);
        let wide = room > 400.0 + 110.0 + 120.0;
        let mut wanted: Option<Element> = None;
        if wide {
            for e in Element::ALL {
                if element_chip(ui, e, current == Some(e)) {
                    wanted = Some(e);
                }
            }
        } else {
            let open = self.popover == Some(Popover::Elements);
            let label = current.map(|e| e.label()).unwrap_or("Element");
            let resp = ui::dropdown(ui, Icon::Pencil, label, open, 150.0);
            self.element_button_rect = resp.rect;
            if resp.on_hover_text("The element the caret is in · Tab cycles").clicked() {
                self.toggle_popover(Popover::Elements);
            }
        }
        if let Some(e) = wanted {
            if editor::set_element(&mut self.doc, &mut self.ed, e) {
                self.mark_changed();
            }
        }
        ui.add_space(6.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        if ui::ribbon_button(
            ui,
            Icon::Print,
            "Quick Export",
            "Export this script as a PDF  ·  Ctrl+E",
            false,
        ) {
            self.export(Format::Pdf, true);
        }
        let left = (room - (ui.min_rect().width())).max(0.0);
        let _ = p;
        self.stats_line(ui, left, true);
    }

    fn cards_tools(&mut self, ui: &mut egui::Ui, room: f32) {
        ui.spacing_mut().item_spacing.x = 6.0;
        if ui::ribbon_button(ui, Icon::Plus, "New scene", "A new scene after the one you are in  ·  Ctrl+Enter", false) {
            self.new_scene();
            self.mode = Mode::Cards;
        }
        let left = (room - ui.min_rect().width()).max(0.0);
        let text = ui::pick_that_fits(
            ui,
            &[
                "·  drag a card to move its scene · right click for colour",
                "·  drag to move · right click for more",
                "·  right click for more",
            ],
            left - 170.0,
        );
        self.stats_line(ui, 170.0_f32.min(left), true);
        if !text.is_empty() {
            ui.label(
                egui::RichText::new(text)
                    .font(theme::font_mono(theme::T_CAP))
                    .color(pal().text_faint),
            );
        }
    }

    fn read_tools(&mut self, ui: &mut egui::Ui, room: f32) {
        ui.spacing_mut().item_spacing.x = 6.0;
        if ui::ribbon_button(ui, Icon::Print, "Quick Export", "Export this script as a PDF  ·  Ctrl+E", false) {
            self.export(Format::Pdf, true);
        }
        let left = (room - ui.min_rect().width()).max(0.0);
        self.stats_line(ui, left.min(200.0), false);
        let hint = ui::pick_that_fits(ui, &["·  click a line to go and write it", "·  click a line to edit"], left - 200.0);
        if !hint.is_empty() {
            ui.label(
                egui::RichText::new(hint)
                    .font(theme::font_mono(theme::T_CAP))
                    .color(pal().text_faint),
            );
        }
    }

    // ---------- the library rail ----------

    fn browser(&mut self, ctx: &egui::Context) {
        if !self.settings.show_library || self.focus_mode {
            return;
        }
        let p = pal();
        egui::SidePanel::left("ns-rail")
            .exact_width(theme::RAIL_W + theme::GAP)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: theme::GAP,
                right: 0.0,
                top: theme::GAP,
                bottom: theme::GAP,
            }))
            .show(ctx, |ui| {
                theme::island_frame(theme::R_ISLAND)
                    .inner_margin(egui::Margin::symmetric(12.0, 14.0))
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());

                        // ---- make things ----
                        ui.horizontal(|ui| {
                            let w = ui.available_width() - 38.0;
                            if new_script_button(ui, w) {
                                self.new_script();
                            }
                            if ui::icon_button(ui, Icon::Upload, "Import Fountain, Final Draft or markdown  ·  or drop a file on the window", 32.0) {
                                self.start_import();
                            }
                        });
                        ui.add_space(9.0);

                        // ---- filter ----
                        let search = ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .id(egui::Id::new("ns-search"))
                                .desired_width(f32::INFINITY)
                                .margin(egui::Margin::symmetric(10.0, 7.0))
                                .hint_text("Filter scripts"),
                        );
                        if self.focus_search {
                            search.request_focus();
                            self.focus_search = false;
                        }
                        ui.add_space(9.0);

                        let mut open: Option<PathBuf> = None;
                        let mut ask: Option<PathBuf> = None;
                        let mut star: Option<PathBuf> = None;
                        let mut menu: Option<(PathBuf, Pos2)> = None;

                        // Room kept at the foot of the rail, so the way to the
                        // tracker and the maker's name are on screen wherever
                        // you are in the list. The gaps egui puts between the
                        // list, the button and the name are counted too —
                        // leave them out and the island runs past the others.
                        const TRACKER: f32 = 34.0;
                        const FOOT: f32 = 24.0;
                        let gaps = ui.spacing().item_spacing.y * 2.0;
                        let list_h = (ui.available_height() - TRACKER - FOOT - gaps).max(60.0);
                        let (list, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), list_h),
                            Sense::hover(),
                        );
                        let mut list_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(list)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("ns-rail-list")
                            .auto_shrink([false; 2])
                            .max_height(list_h)
                            .show(&mut list_ui, |ui| {
                                self.rail_body(ui, &mut open, &mut ask, &mut star, &mut menu);
                            });

                        if youtrack_button(ui, &self.settings.youtrack_url) {
                            let url = self.settings.youtrack_url.trim().to_string();
                            if url.starts_with("http://") || url.starts_with("https://") {
                                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                            } else {
                                self.deck.warn("That is not a web address", "Set the YouTrack link in Settings.");
                            }
                        }

                        let (foot, _) =
                            ui.allocate_exact_size(Vec2::new(ui.available_width(), FOOT), Sense::hover());
                        self.foot_rect = foot;
                        theme::tracked_text(
                            ui.painter(),
                            Pos2::new(foot.left() + 3.0, foot.center().y + 2.0),
                            "MARKEDEXILED SOFTWARE",
                            theme::font_semi(theme::T_MICRO - 0.5),
                            theme::wash(p.text_faint, 150),
                            1.4,
                        );

                        if let Some(path) = open {
                            self.open(path);
                        }
                        if let Some(path) = star {
                            self.toggle_star(&path);
                        }
                        if let Some(path) = ask {
                            self.ask_delete(path);
                        }
                        if let Some((path, at)) = menu {
                            self.rail.menu = Some((path, at, self.frame_no));
                        }
                    });
            });
    }

    fn rail_body(
        &mut self,
        ui: &mut egui::Ui,
        open: &mut Option<PathBuf>,
        ask: &mut Option<PathBuf>,
        star: &mut Option<PathBuf>,
        menu: &mut Option<(PathBuf, Pos2)>,
    ) {
        let p = pal();
        let needle = self.search.trim().to_lowercase();
        let mut list: Vec<Entry> = self
            .entries
            .iter()
            .filter(|e| {
                needle.is_empty()
                    || e.title.to_lowercase().contains(&needle)
                    || e.preview.to_lowercase().contains(&needle)
            })
            .cloned()
            .collect();
        match self.settings.sort_by {
            SortBy::Recent => list.sort_by(|a, b| b.modified.cmp(&a.modified)),
            SortBy::Title => list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase())),
            SortBy::Length => list.sort_by(|a, b| b.pages.cmp(&a.pages)),
        }

        let mut selected_row: Option<Rect> = None;
        let mut seen_rects: HashMap<PathBuf, Rect> = HashMap::new();
        let starred_first = self.settings.starred_first;
        let (starred, rest): (Vec<Entry>, Vec<Entry>) = if starred_first {
            list.into_iter().partition(|e| e.starred)
        } else {
            (Vec::new(), list)
        };
        let mut shown = 0usize;

        for (label, icon, tint, items, is_starred) in [
            ("Starred", Icon::StarFilled, Some(p.warn), &starred, true),
            ("Scripts", Icon::File, None, &rest, false),
        ] {
            if items.is_empty() && (is_starred || shown > 0) {
                continue;
            }
            let mut collapsed = if is_starred {
                self.rail.starred_collapsed
            } else {
                self.rail.all_collapsed
            };
            let head = section_header(ui, label, icon, items.len(), &mut collapsed, tint);
            if head.clicked {
                if is_starred {
                    self.rail.starred_collapsed = collapsed;
                } else {
                    self.rail.all_collapsed = collapsed;
                }
            }
            if !collapsed {
                let branch = ui.painter().add(egui::Shape::Noop);
                let mut kid_rects = Vec::new();
                for e in items {
                    shown += 1;
                    let r = self.script_row(ui, e, open, ask, star, menu);
                    seen_rects.insert(e.path.clone(), r);
                    if self.path.as_deref() == Some(e.path.as_path()) {
                        selected_row = Some(r);
                    }
                    kid_rects.push(r);
                }
                draw_branch(ui, branch, head.rect, &kid_rects, tint.unwrap_or(p.text_faint));
            }
            ui.add_space(8.0);
        }

        self.rail.last_rects = seen_rects;

        // A deleted script is watched out rather than simply ceasing to be: the
        // row it left behind lifts, narrows and burns off.
        if let Some((title, rect, at)) = self.rail.ghost.clone() {
            let t = (at.elapsed().as_secs_f32() / 0.5).clamp(0.0, 1.0);
            if t >= 1.0 || !anim::enabled() {
                self.rail.ghost = None;
            } else {
                let fade = 1.0 - anim::smootherstep(t);
                let r = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.center().y - 14.0 * (1.0 - fade)),
                    Vec2::new(rect.width() * (0.7 + 0.3 * fade), rect.height() * fade.max(0.05)),
                );
                ui.painter().rect_filled(
                    r,
                    egui::Rounding::same(theme::R_MD),
                    theme::wash(p.danger, (60.0 * fade) as u8),
                );
                ui.painter().text(
                    Pos2::new(r.left() + 16.0, r.center().y),
                    egui::Align2::LEFT_CENTER,
                    ui::elide(&title, 22),
                    theme::font(theme::T_SM),
                    theme::wash(p.danger_light, (200.0 * fade) as u8),
                );
                anim::keep_going(ui.ctx());
            }
        }

        // One marker for the whole list, which springs from the script you were
        // on to the one you just opened rather than blinking between them.
        if let Some(r) = selected_row {
            let y = anim::spring_to(ui.ctx(), "ns-rail-marker", r.center().y, 0.45);
            let x = anim::glide(ui.ctx(), "ns-rail-marker-x", r.left() + 7.5, 0.30);
            let bar = Rect::from_center_size(Pos2::new(x, y), Vec2::new(3.0, r.height() - 20.0));
            ui.painter()
                .rect_filled(bar, egui::Rounding::same(2.0), p.primary_light);
            if (y - r.center().y).abs() > 0.4 {
                anim::keep_going(ui.ctx());
            }
        }

        if shown == 0 {
            ui.add_space(14.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(if needle.is_empty() {
                        "No scripts yet."
                    } else {
                        "Nothing matches."
                    })
                    .font(theme::font(theme::T_SM))
                    .color(p.text_faint),
                );
            });
        }
    }

    fn script_row(
        &self,
        ui: &mut egui::Ui,
        e: &Entry,
        open: &mut Option<PathBuf>,
        ask: &mut Option<PathBuf>,
        star: &mut Option<PathBuf>,
        menu: &mut Option<(PathBuf, Pos2)>,
    ) -> Rect {
        let p = pal();
        let selected = self.path.as_deref() == Some(e.path.as_path());
        let w = ui.available_width();
        let (slot, resp) = ui.allocate_exact_size(Vec2::new(w, 40.0), Sense::click());
        // filed under its section's header, drawn in from its edge
        let rect = Rect::from_min_max(Pos2::new(slot.left() + INDENT, slot.top()), slot.max);
        let _ = ui.interact(rect, row_id(&e.path), Sense::hover());
        let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() || selected, anim::HOVER);

        let fill = if selected {
            p.primary_quiet
        } else {
            theme::mix(p.solid_hi, p.raised, hot)
        };
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(theme::R_MD), fill);

        // the open script's title is the live one, not the one on disk
        let title = if selected { self.title_or_untitled() } else { e.title.clone() };
        let right_pad = 4.0 + 26.0 * (hot.max(if e.starred { 1.0 } else { 0.0 }));
        ui.painter().text(
            Pos2::new(rect.left() + 16.0, rect.top() + 14.0),
            egui::Align2::LEFT_CENTER,
            ui::elide(&title, ((rect.width() - 34.0 - right_pad) / 6.6) as usize),
            if selected {
                theme::font_med(theme::T_SM)
            } else {
                theme::font(theme::T_SM)
            },
            if selected { p.primary_light } else { p.text },
        );
        let pages = if selected { self.layout.pages } else { e.pages };
        ui.painter().text(
            Pos2::new(rect.left() + 16.0, rect.bottom() - 13.0),
            egui::Align2::LEFT_CENTER,
            format!("{pages} pg · {}", relative_time(e.modified)),
            theme::font_mono(theme::T_MICRO),
            p.text_faint,
        );

        let star_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 44.0, rect.center().y),
            Vec2::splat(24.0),
        );
        let del_rect = Rect::from_center_size(
            Pos2::new(rect.right() - 20.0, rect.center().y),
            Vec2::splat(24.0),
        );
        let star_resp = ui.interact(star_rect, resp.id.with("star"), Sense::click());
        let del_resp = ui.interact(del_rect, resp.id.with("del"), Sense::click());

        // the star and the bin light *themselves* up rather than sitting on a
        // lozenge
        let star_hot = anim::ease(ui.ctx(), star_resp.id, star_resp.hovered(), anim::HOVER);
        if e.starred || hot > 0.02 {
            let lit = if e.starred { 1.0 } else { hot };
            icons::draw(
                ui.painter(),
                star_rect.shrink(6.0),
                if e.starred { Icon::StarFilled } else { Icon::Star },
                if e.starred {
                    theme::lighten(p.warn, 0.2 * star_hot)
                } else {
                    theme::mix(theme::wash(p.text_faint, (200.0 * lit) as u8), p.warn, star_hot)
                },
            );
        }
        let del_hot = anim::ease(ui.ctx(), del_resp.id, del_resp.hovered(), anim::HOVER);
        if hot > 0.02 {
            icons::draw(
                ui.painter(),
                del_rect.shrink(6.0),
                Icon::Trash,
                theme::mix(theme::wash(p.text_faint, (200.0 * hot) as u8), p.danger_light, del_hot),
            );
        }

        if star_resp.clicked() {
            *star = Some(e.path.clone());
        } else if del_resp.clicked() {
            *ask = Some(e.path.clone());
        } else if resp.clicked() && !selected {
            *open = Some(e.path.clone());
        }
        if resp.secondary_clicked() {
            if let Some(q) = ui.ctx().pointer_latest_pos() {
                *menu = Some((e.path.clone(), q));
            }
        }
        let _ = star_resp.on_hover_text(if e.starred { "Unstar" } else { "Star this script" });
        let _ = del_resp.on_hover_text("Delete this script");
        if !e.preview.is_empty() {
            let _ = resp.on_hover_text(format!("{}\n{} scenes · right click for more", e.preview, e.scenes));
        }

        ui.add_space(5.0);
        rect
    }

    fn row_menu(&mut self, ctx: &egui::Context) {
        let Some((path, at, opened)) = self.rail.menu.clone() else {
            return;
        };
        let starred = self.entries.iter().find(|e| e.path == path).map(|e| e.starred).unwrap_or(false);
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("ns-row-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    ui.set_width(ui::menu_width(ui, &["Open", "Duplicate", "Show the file", "Remove the star", "Delete script"]));
                    let mut rects = Vec::new();
                    if ui::menu_item(ui, &mut rects, "Open", Icon::Pencil, false) {
                        if self.path.as_deref() != Some(path.as_path()) {
                            self.open(path.clone());
                        }
                        close = true;
                    }
                    if ui::menu_item(ui, &mut rects, "Duplicate", Icon::Copy, false) {
                        self.duplicate(&path);
                        close = true;
                    }
                    if ui::menu_item(ui, &mut rects, "Show the file", Icon::FolderOpen, false) {
                        storage::reveal_in_file_manager(&path);
                        close = true;
                    }
                    if ui::menu_item(
                        ui,
                        &mut rects,
                        if starred { "Remove the star" } else { "Star this script" },
                        if starred { Icon::Star } else { Icon::StarFilled },
                        false,
                    ) {
                        self.toggle_star(&path);
                        close = true;
                    }
                    ui::separator(ui);
                    if ui::menu_item(ui, &mut rects, "Delete script", Icon::Trash, true) {
                        self.ask_delete(path.clone());
                        close = true;
                    }
                });
            })
            .response;
        let d = ui::Dismisser { opened_on: opened };
        if close
            || d.should_close(ctx, self.frame_no, &[area.rect])
            || ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.rail.menu = None;
        }
    }

    // ---------- the right island ----------

    fn aside(&mut self, ctx: &egui::Context) {
        let details = self.settings.show_details;
        let nav = self.settings.show_scenes || self.settings.show_cast;
        if !(details || nav) || self.focus_mode {
            self.aside_rect = Rect::NOTHING;
            return;
        }
        egui::SidePanel::right("ns-aside")
            .exact_width(theme::OUTLINE_W + theme::GAP)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 0.0,
                right: theme::GAP,
                top: theme::GAP,
                bottom: theme::GAP,
            }))
            .show(ctx, |ui| {
                self.aside_rect = ui.max_rect();
                self.aside_islands.clear();
                // Two islands down the right, with the same gap of backdrop
                // between them as everywhere else: the script's details on
                // top, the navigator under it. Whichever is alone fills the
                // column.
                ui.spacing_mut().item_spacing.y = theme::GAP;
                if details {
                    let island = theme::island_frame(theme::R_ISLAND)
                        .inner_margin(egui::Margin::symmetric(12.0, 14.0));
                    if nav {
                        // never more than a little under half the column, so
                        // the navigator under it always has room to work
                        let cap = (ui.available_height() * 0.46 - 28.0).max(120.0);
                        let r = island.show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::vertical()
                                .id_salt("ns-details")
                                .max_height(cap)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    ui.set_max_width(ui.available_width() - 6.0);
                                    self.details_panel(ui);
                                });
                        });
                        self.aside_islands.push(r.response.rect);
                    } else {
                        let r = island.show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            let room = ui.available_height();
                            egui::ScrollArea::vertical()
                                .id_salt("ns-details")
                                .max_height(room)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_max_width(ui.available_width() - 6.0);
                                    self.details_panel(ui);
                                });
                        });
                        self.aside_islands.push(r.response.rect);
                    }
                }
                if nav {
                    let r = theme::island_frame(theme::R_ISLAND)
                        .inner_margin(egui::Margin::symmetric(12.0, 14.0))
                        .show(ui, |ui| {
                            ui.set_min_size(ui.available_size());
                            let both = self.settings.show_scenes && self.settings.show_cast;
                            if self.settings.show_scenes {
                                let h = if both {
                                    // the cast under it needs its heading and
                                    // summary at the least
                                    (ui.available_height() * 0.58)
                                        .min(ui.available_height() - 120.0)
                                        .max(60.0)
                                } else {
                                    ui.available_height()
                                };
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), h),
                                    Sense::hover(),
                                );
                                let mut child = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(rect)
                                        .layout(Layout::top_down(Align::Min)),
                                );
                                self.scene_outliner(&mut child);
                            }
                            if both {
                                ui::separator(ui);
                            }
                            if self.settings.show_cast {
                                // drawn into exactly the room that is left and
                                // clipped to it, so it can never make the
                                // island taller than the column
                                let room = Rect::from_min_size(
                                    ui.cursor().min,
                                    Vec2::new(ui.available_width(), ui.available_height().max(0.0)),
                                );
                                let mut child = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(room)
                                        .layout(Layout::top_down(Align::Min)),
                                );
                                child.set_clip_rect(room.intersect(ui.clip_rect()));
                                self.cast_panel(&mut child);
                                ui.allocate_rect(room, Sense::hover());
                            }
                        });
                    self.aside_islands.push(r.response.rect);
                }
            });
    }

    /// The script's details: its title page, and what it adds up to.
    fn details_panel(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let mut edited = false;
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            icons::show(ui, Icon::Info, 12.0, p.text_faint);
            ui.label(
                egui::RichText::new("DETAILS")
                    .font(theme::font_semi(theme::T_MICRO))
                    .color(p.text_faint),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui::icon_button(ui, Icon::Close, "Hide · Ctrl+I", 20.0) {
                    self.settings.show_details = false;
                    self.save_settings();
                }
            });
        });
        ui.add_space(4.0);
        ui::section(ui, "Title page");
        ui.spacing_mut().item_spacing.y = 4.0;
        for (label, value, hint) in [
            ("Title", &mut self.doc.meta.title, "The Long Way Down"),
            ("Written by", &mut self.doc.meta.author, "A. Writer"),
            ("Draft", &mut self.doc.meta.draft, "First Draft"),
            ("Contact", &mut self.doc.meta.contact, "agent@example.com"),
        ] {
            ui.label(
                egui::RichText::new(label)
                    .font(theme::font_med(theme::T_CAP))
                    .color(p.text_faint),
            );
            if ui
                .add(
                    egui::TextEdit::singleline(value)
                        .desired_width(f32::INFINITY)
                        .margin(egui::Margin::symmetric(10.0, 6.0))
                        .hint_text(hint),
                )
                .changed()
            {
                edited = true;
            }
            ui.add_space(4.0);
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 5.0);
            for (label, value) in [
                ("today", today()),
                ("first draft", "First Draft".to_string()),
                ("revised", format!("Revised {}", today())),
            ] {
                if ui::chip_button(ui, label) {
                    self.doc.meta.draft = value;
                    edited = true;
                }
            }
        });
        if edited {
            self.mark_changed();
        }

        ui.add_space(6.0);
        ui::separator(ui);
        ui::section(ui, "The script");
        let l = &self.layout;
        let rows = [
            ("Pages", format!("{}", l.pages)),
            ("Running time", format!("about {} min", l.pages)),
            ("Scenes", format!("{}", l.scenes)),
            ("Speaking parts", format!("{}", l.speakers.len())),
            ("Words", format!("{}", l.words)),
            ("Dialogue", format!("{:.0}%", l.dialogue * 100.0)),
        ];
        for (k, v) in rows {
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(k)
                        .font(theme::font(theme::T_SM))
                        .color(p.text_dim),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(v)
                            .font(theme::font_mono(theme::T_CAP))
                            .color(p.text),
                    );
                });
            });
        }
    }

    /// Every scene, in order, on a mini rail of its own — the navigator.
    fn scene_outliner(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        let scenes = self.doc.scenes();
        let live = self.ed.focus_block.and_then(|id| self.doc.scene_of(id));

        ui.horizontal(|ui| {
            ui.add_space(4.0);
            icons::show(ui, Icon::List, 12.0, p.text_faint);
            ui.label(
                egui::RichText::new("SCENES")
                    .font(theme::font_semi(theme::T_MICRO))
                    .color(p.text_faint),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{}", scenes.len()))
                        .font(theme::font_mono(theme::T_MICRO))
                        .color(p.text_faint),
                );
            });
        });
        ui.add_space(8.0);

        if scenes.is_empty() {
            ui.label(
                egui::RichText::new("No scene headings yet. Ctrl+1 turns a line into one.")
                    .font(theme::font(theme::T_CAP))
                    .color(p.text_faint),
            );
            return;
        }

        let mut jump = None;
        egui::ScrollArea::vertical()
            .id_salt("ns-outline")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let rail_shape = ui.painter().add(egui::Shape::Noop);
                // the rail is inset from the island's edge, not pinned to it
                let left = ui.min_rect().left() + 16.0;
                let text_left = left + 18.0;
                let mut dots: Vec<f32> = Vec::new();

                ui.spacing_mut().item_spacing.y = 2.0;
                for (i, sc) in scenes.iter().enumerate() {
                    let w = ui.available_width();
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 38.0), Sense::click());
                    let is_live = live == Some(i);
                    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);

                    // The live scene gets the editor's own light — a wash that
                    // fades away to the right, and a short bar in the accent —
                    // so the words on it stay readable. Hover is only a breath.
                    let slot = rect.shrink2(Vec2::new(0.0, 2.0));
                    if is_live {
                        theme::fill_grad_poly(
                            ui.painter(),
                            &theme::rounded_poly(slot, theme::R_SM),
                            theme::wash(p.primary, if p.dark { 44 } else { 36 }),
                            theme::wash(p.primary, 0),
                            Vec2::new(1.0, 0.0),
                        );
                        let bar = Rect::from_center_size(
                            Pos2::new(slot.left() + 3.5, slot.center().y),
                            Vec2::new(3.0, slot.height() - 16.0),
                        );
                        ui.painter().rect_filled(bar, egui::Rounding::same(1.5), p.primary_light);
                    } else if hot > 0.02 {
                        ui.painter().rect_filled(
                            slot,
                            egui::Rounding::same(theme::R_SM),
                            theme::wash(p.text_faint, (22.0 * hot) as u8),
                        );
                    }

                    let cy = rect.center().y;
                    dots.push(cy);
                    let r = if is_live { 5.6 } else { 4.4 };
                    let tint = sc.tint.map(|t| p.group(t));
                    // only the scene you are in is lit, and faintly
                    let lit = anim::ease(ui.ctx(), resp.id.with("lit"), is_live, 0.26);
                    if lit > 0.01 {
                        theme::glow_star(
                            ui.painter(),
                            Pos2::new(left, cy),
                            r,
                            tint.unwrap_or(p.sec_grad.1),
                            10.0,
                            0.14 * lit,
                        );
                    }
                    match tint {
                        Some(t) => theme::grad_star(ui.painter(), Pos2::new(left, cy), r, theme::darken(t, 0.2), theme::lighten(t, 0.2)),
                        None => theme::grad_star(ui.painter(), Pos2::new(left, cy), r, p.sec_grad.0, p.sec_grad.1),
                    }

                    let name = if sc.heading.trim().is_empty() {
                        format!("Scene {}", i + 1)
                    } else {
                        sc.heading.clone()
                    };
                    let label = if self.settings.scene_numbers {
                        format!("{}. {name}", sc.number)
                    } else {
                        name
                    };
                    ui.painter().text(
                        Pos2::new(text_left, cy - 6.5),
                        egui::Align2::LEFT_CENTER,
                        ui::elide(&label, ((rect.right() - text_left - 16.0) / 7.2) as usize),
                        if is_live {
                            theme::font_med(theme::T_SM)
                        } else {
                            theme::font(theme::T_SM)
                        },
                        if is_live { p.text } else { theme::mix(p.text_dim, p.text, hot) },
                    );
                    let (page, eighths) = self.layout.lengths.get(&sc.id).copied().unwrap_or((0, 0));
                    let meta = if page > 0 {
                        format!("p.{page} · {} pg · {} words", eighths_label(eighths), sc.words)
                    } else {
                        "empty".to_string()
                    };
                    ui.painter().text(
                        Pos2::new(text_left, cy + 8.5),
                        egui::Align2::LEFT_CENTER,
                        meta,
                        theme::font_mono(theme::T_MICRO),
                        p.text_faint,
                    );
                    if !sc.synopsis.trim().is_empty() {
                        let _ = resp.clone().on_hover_text(sc.synopsis.clone());
                    }
                    if resp.clicked() {
                        jump = Some(sc.id);
                    }
                }

                // the mini rail, broken around each star
                let mut shapes = Vec::new();
                let gap = 9.0;
                let mut cursor = ui.min_rect().top() + 4.0;
                let mut edges = Vec::new();
                for cy in &dots {
                    if cy - gap > cursor {
                        edges.push((cursor, cy - gap));
                    }
                    cursor = cy + gap;
                }
                for (a, b) in edges {
                    shapes.push(egui::Shape::circle_filled(Pos2::new(left, a), 1.0, p.line_strong));
                    shapes.push(egui::Shape::circle_filled(Pos2::new(left, b), 1.0, p.line_strong));
                    shapes.push(egui::Shape::line_segment(
                        [Pos2::new(left, a), Pos2::new(left, b)],
                        Stroke::new(2.0_f32, p.line_strong),
                    ));
                }
                ui.painter().set(rail_shape, egui::Shape::Vec(shapes));
            });

        if let Some(id) = jump {
            self.mode = Mode::Write;
            self.ed.jump_to(id);
        }
    }

    /// Everyone who speaks, and the balance of talk to action.
    fn cast_panel(&mut self, ui: &mut egui::Ui) {
        let p = pal();
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            icons::show(ui, Icon::Users, 13.0, p.text_faint);
            ui.label(
                egui::RichText::new("CAST")
                    .font(theme::font_semi(theme::T_MICRO))
                    .color(p.text_faint),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{}", self.layout.cast.len()))
                        .font(theme::font_mono(theme::T_MICRO))
                        .color(p.text_faint),
                );
            });
        });
        ui.add_space(6.0);

        if self.layout.cast.is_empty() {
            ui.label(
                egui::RichText::new("Nobody speaks yet. A character cue is Ctrl+3.")
                    .font(theme::font(theme::T_CAP))
                    .color(p.text_faint),
            );
            return;
        }

        let mut jump: Option<(String, u64)> = None;
        let cast = self.layout.cast.clone();
        let voices = self.voices();
        // How much of the script is talk, straight under the heading — a
        // summary belongs with its title, not stranded at the foot of a
        // list that may only have one name in it.
        let d = self.layout.dialogue;
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!("dialogue {:.0}%  ·  the rest {:.0}%", d * 100.0, (1.0 - d) * 100.0))
                    .font(theme::font_mono(theme::T_MICRO))
                    .color(p.text_faint),
            );
        });
        ui.add_space(-2.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
        let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 8.0, 4.0));
        ui.painter()
            .rect_filled(track, egui::Rounding::same(2.0), theme::wash(p.text_faint, 45));
        if d > 0.0 {
            let fill = Rect::from_min_max(track.min, Pos2::new(track.left() + track.width() * d, track.bottom()));
            theme::grad_rect(ui.painter(), fill, 2.0, p.prim_grad.0, p.prim_grad.1, Vec2::new(1.0, 0.0));
        }
        ui.add_space(2.0);
        // the list takes exactly what is left, and scrolls within it
        let list_h = ui.available_height().max(0.0);
        egui::ScrollArea::vertical()
            .id_salt("ns-cast")
            .auto_shrink([false; 2])
            .max_height(list_h)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (k, c) in cast.iter().enumerate() {
                    let w = ui.available_width();
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
                    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
                    if hot > 0.02 {
                        ui.painter().rect_filled(
                            rect,
                            egui::Rounding::same(theme::R_SM),
                            theme::wash(p.primary, (30.0 * hot) as u8),
                        );
                    }
                    ui.painter().circle_filled(
                        Pos2::new(rect.left() + 12.0, rect.center().y),
                        3.0,
                        voices
                            .as_ref()
                            .and_then(|v| v.get(&c.name).copied())
                            .unwrap_or_else(|| p.group(k)),
                    );
                    ui.painter().text(
                        Pos2::new(rect.left() + 24.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        ui::elide(&c.name, ((rect.width() - 70.0) / 6.3) as usize),
                        theme::font(theme::T_SM),
                        theme::mix(p.text_dim, p.text, hot),
                    );
                    ui.painter().text(
                        Pos2::new(rect.right() - 8.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{}", c.cues),
                        theme::font_mono(theme::T_MICRO),
                        p.text_faint,
                    );
                    let resp = resp.on_hover_text(format!(
                        "{} cue{} · {} words · {} scene{}\nClick to step through their lines",
                        c.cues,
                        if c.cues == 1 { "" } else { "s" },
                        c.words,
                        c.scenes,
                        if c.scenes == 1 { "" } else { "s" }
                    ));
                    if resp.clicked() {
                        let n = self.cast_cursor.get(&c.name).copied().unwrap_or(0);
                        if let Some(id) = c.cue_ids.get(n % c.cue_ids.len().max(1)) {
                            jump = Some((c.name.clone(), *id));
                        }
                    }
                }
            });
        if let Some((name, id)) = jump {
            *self.cast_cursor.entry(name).or_insert(0) += 1;
            self.mode = Mode::Write;
            self.ed.jump_to(id);
        }
    }

    // ---------- the pages ----------

    /// 0 → 1 across the moment after the page changed what it is showing.
    fn page_arrival(&self, ctx: &egui::Context) -> f32 {
        if !anim::enabled() {
            return 1.0;
        }
        let t = (self.page_born.elapsed().as_secs_f32() / 0.55).clamp(0.0, 1.0);
        if t < 1.0 {
            ctx.request_repaint();
        }
        anim::bounce(t)
    }

    fn page(&mut self, ctx: &egui::Context) {
        let arrive = self.page_arrival(ctx);
        let mode = self.mode;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::same(theme::GAP)))
            .show(ctx, |ui| {
                theme::island_frame(theme::R_ISLAND)
                    .inner_margin(egui::Margin::ZERO)
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        let island = ui.max_rect();
                        self.page_rect = island;
                        // the page rises into the island, masked by its edge
                        let mut ui = slide_in(ui, island, arrive, if mode == Mode::Write { 40.0 } else { 24.0 });
                        let ui = &mut ui;
                        match mode {
                            Mode::Write => self.write_page(ui),
                            Mode::Cards => self.cards_page(ui),
                            Mode::Read => self.read_page(ui),
                        }
                    });
            });
    }

    fn write_page(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let starts = std::mem::take(&mut self.layout.page_starts);
        let voices = self.voices();
        egui::ScrollArea::vertical()
            .id_salt("ns-write")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let avail = ui.available_width();
                let col = editor::column_width(ui, self.ed.font_px).min(avail - 16.0);
                let pad = ((avail - col) * 0.5).max(8.0);

                changed |= self.banner(ui, avail, pad);
                ui.add_space(18.0);

                let needle = if self.find.open { Some(self.find.needle.clone()) } else { None };
                let current = if self.find.open {
                    self.doc.find(&self.find.needle).get(self.find.current).copied()
                } else {
                    None
                };
                let view = editor::View {
                    page_starts: &starts,
                    page_breaks: self.settings.page_breaks,
                    scene_numbers: self.settings.scene_numbers,
                    smart_type: self.settings.smart_type,
                    typewriter: self.settings.typewriter,
                    focus_mode: self.focus_mode,
                    find: needle.as_deref(),
                    current,
                    element_colors: self.settings.element_colors,
                    char_colors: voices.as_ref(),
                };
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_width(col);
                        ui.set_max_width(col);
                        changed |= editor::show(ui, &mut self.doc, &mut self.ed, &view);
                        ui.add_space(90.0);
                    });
                });
            });
        self.layout.page_starts = starts;
        if changed {
            self.mark_changed();
        }
    }

    /// The band across the head of the script: its title, set large, on
    /// frosted glass with the island's own top corners; under it the file, who
    /// wrote it, the draft, and the way into the title page.
    fn banner(&mut self, ui: &mut egui::Ui, avail: f32, pad: f32) -> bool {
        let p = pal();
        let mut changed = false;
        let banner_bg = ui.painter().add(egui::Shape::Noop);
        let (band, _) = ui.allocate_exact_size(Vec2::new(avail, BANNER_H), Sense::hover());
        let mut head = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    Pos2::new(band.left() + pad, band.top() + 16.0),
                    Pos2::new(band.right() - pad, band.bottom()),
                ))
                .layout(Layout::top_down(Align::Min)),
        );
        let title_id = egui::Id::new("ns-script-title");
        let r = head.add(
            egui::TextEdit::singleline(&mut self.doc.meta.title)
                .id(title_id)
                .frame(false)
                .desired_width(f32::INFINITY)
                .font(theme::font_semi(theme::T_TITLE))
                .text_color(p.text)
                .margin(egui::Margin {
                    left: editor::GUTTER,
                    right: 16.0,
                    top: 2.0,
                    bottom: 2.0,
                })
                .hint_text("Name this script"),
        );
        if self.focus_title {
            r.request_focus();
            let n = caret::char_len(&self.doc.meta.title);
            caret::select(ui.ctx(), title_id, 0, n);
            self.focus_title = false;
        }
        if r.changed() {
            changed = true;
        }
        // Enter on the title goes on into the script
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(b) = self.doc.blocks.first() {
                self.ed.focus(b.id, Caret::End);
            }
        }

        // One row, never wrapped: a band is only so tall, and a wrapped
        // second line of chips would fall off the bottom of it. The chips
        // are measured first; the file name gets whatever room is left.
        let chip_font = theme::font(theme::T_MICRO + 0.5);
        let pg = self.layout.pages;
        let mut chips: Vec<(String, Option<Color32>)> = Vec::new();
        if !self.doc.meta.author.trim().is_empty() {
            chips.push((format!("by {}", self.doc.meta.author.trim()), Some(p.primary)));
        }
        if !self.doc.meta.draft.trim().is_empty() {
            chips.push((self.doc.meta.draft.trim().to_string(), Some(p.sec)));
        }
        chips.push((
            format!("{pg} page{}  ·  {pg} min", if pg == 1 { "" } else { "s" }),
            None,
        ));
        let details_label = if self.settings.show_details { "details" } else { "+ title page" };
        let gap = 6.0;
        let mut chips_w = 0.0;
        for (t, _) in &chips {
            chips_w += head.painter().layout_no_wrap(t.clone(), chip_font.clone(), p.text).rect.width() + 15.0 + gap;
        }
        chips_w += head.painter().layout_no_wrap(details_label.to_string(), chip_font.clone(), p.text).rect.width() + 17.0;
        let file = self
            .path
            .as_ref()
            .and_then(|x| x.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("not saved yet")
            .to_string();
        let room = head.available_width() - editor::GUTTER - chips_w - 16.0;
        let char_w = head
            .painter()
            .layout_no_wrap("m".to_string(), theme::font_mono(theme::T_CAP), p.text)
            .rect
            .width()
            .max(1.0);
        let file = if room < char_w * 6.0 {
            String::new()
        } else {
            ui::elide(&file, (room / char_w) as usize)
        };
        let mut toggle_details = false;
        head.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(gap, 0.0);
            ui.set_height(20.0);
            ui.add_space(editor::GUTTER);
            if !file.is_empty() {
                ui.label(
                    egui::RichText::new(file)
                        .font(theme::font_mono(theme::T_CAP))
                        .color(p.text_faint),
                );
                ui.add_space(4.0);
            }
            for (t, tint) in &chips {
                ui::chip(ui, t, *tint);
            }
            if ui::chip_button(ui, details_label) {
                toggle_details = true;
            }
        });
        if toggle_details {
            self.settings.show_details = !self.settings.show_details;
            self.save_settings();
        }
        ui.painter().set(banner_bg, banner_shape(band));
        changed
    }

    fn cards_page(&mut self, ui: &mut egui::Ui) {
        let live = self
            .ed
            .focus_block
            .and_then(|id| self.doc.scene_of(id))
            .and_then(|k| self.doc.scenes().get(k).map(|s| s.id));
        let lengths = self.layout.lengths.clone();
        let mut out = cards::Out { changed: false, act: None };
        egui::ScrollArea::vertical()
            .id_salt("ns-cards")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                out = cards::show(ui, &mut self.doc, &mut self.cards, &lengths, live, self.frame_no);
            });
        if out.changed {
            self.mark_changed();
        }
        match out.act {
            Some(cards::Act::Open(id)) => {
                self.mode = Mode::Write;
                self.ed.jump_to(id);
            }
            Some(cards::Act::Move(from, to)) => {
                self.checkpoint();
                if self.doc.move_scene(from, to) {
                    self.mark_changed();
                    self.snapshot_due = false;
                    self.baseline = self.doc.clone();
                    self.deck.ok("Scene moved", "Ctrl+Z puts it back");
                }
            }
            Some(cards::Act::Tint(id, t)) => {
                if let Some(i) = self.doc.index_of(id) {
                    self.checkpoint();
                    self.doc.blocks[i].tint = t;
                    self.mark_changed();
                    self.snapshot_due = false;
                    self.baseline = self.doc.clone();
                }
            }
            Some(cards::Act::Delete(n)) => self.ask_delete_scene(n),
            Some(cards::Act::NewScene) => {
                // at the very end, as the card sits at the end of the wall
                self.checkpoint();
                let b = self.doc.new_block(Element::SceneHeading, "");
                let id = b.id;
                self.doc.blocks.push(b);
                self.mark_changed();
                self.snapshot_due = false;
                self.baseline = self.doc.clone();
                self.mode = Mode::Write;
                self.ed.focus(id, Caret::Start);
                self.ed.scroll_to_focus = true;
            }
            None => {}
        }
    }

    fn read_page(&mut self, ui: &mut egui::Ui) {
        let voices = if self.settings.character_colors_reading { self.voices() } else { None };
        let jump = pages::show(ui, &self.doc, &mut self.pages, self.settings.scene_numbers, voices.as_ref());
        if let Some(id) = jump {
            self.mode = Mode::Write;
            self.ed.jump_to(id);
        }
    }


    // ---------- find & replace ----------

    fn find_bar(&mut self, ctx: &egui::Context) {
        if !self.find.open || self.mode != Mode::Write {
            return;
        }
        let p = pal();
        let matches = self.doc.find(&self.find.needle);
        if self.find.current >= matches.len() {
            self.find.current = 0;
        }
        let n_len = self.find.needle.chars().count();
        let field_id = egui::Id::new("ns-find-field");
        let replace_id = egui::Id::new("ns-find-replace");

        // Enter steps through the results without leaving the field
        let in_field = ctx.memory(|m| m.has_focus(field_id));
        let mut step: i32 = 0;
        if in_field {
            ctx.input_mut(|i| {
                if i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter) {
                    step = -1;
                } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    step = 1;
                }
            });
        }

        let w = 404.0;
        let h = 78.0;
        let at = Pos2::new(self.page_rect.right() - w - 16.0, self.page_rect.top() + 16.0);
        let mut replace_one = false;
        let mut replace_all = false;
        let mut close = false;
        let area = egui::Area::new(egui::Id::new("ns-find"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
                // glass: it floats over the work
                theme::lift_shadow(ui.painter(), rect, theme::R_MD, 1.0);
                theme::glass_surface(ui.painter(), rect, theme::R_MD, 0.96);
                let inner = rect.shrink2(Vec2::new(14.0, 8.0));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(inner)
                        .layout(Layout::top_down(Align::Min)),
                );
                child.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                child.horizontal(|ui| {
                    icons::show(ui, Icon::Search, 13.0, p.text_faint);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.find.needle)
                            .id(field_id)
                            .desired_width(186.0)
                            .margin(egui::Margin::symmetric(8.0, 5.0))
                            .hint_text("Find in the script"),
                    );
                    if self.find.focus {
                        resp.request_focus();
                        self.find.focus = false;
                    }
                    if resp.changed() {
                        self.find.current = 0;
                        step = 0;
                    }
                    let count = if self.find.needle.is_empty() {
                        String::new()
                    } else if matches.is_empty() {
                        "none".to_string()
                    } else {
                        format!("{} of {}", self.find.current + 1, matches.len())
                    };
                    ui.add_sized(
                        Vec2::new(52.0, 24.0),
                        egui::Label::new(
                            egui::RichText::new(count)
                                .font(theme::font_mono(theme::T_MICRO))
                                .color(p.text_faint),
                        ),
                    );
                    if ui::icon_button(ui, Icon::ChevronUp, "Previous · Shift+Enter", 24.0) {
                        step = -1;
                    }
                    if ui::icon_button(ui, Icon::ChevronDown, "Next · Enter", 24.0) {
                        step = 1;
                    }
                    if ui::icon_button_tinted(ui, Icon::Close, "Close · Esc", 24.0, None) {
                        close = true;
                    }
                });
                child.horizontal(|ui| {
                    icons::show(ui, Icon::Refresh, 13.0, p.text_faint);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.find.with)
                            .id(replace_id)
                            .desired_width(186.0)
                            .margin(egui::Margin::symmetric(8.0, 5.0))
                            .hint_text("Replace with"),
                    );
                    if ui::button_sized(ui, "Replace", None, false, Some(72.0)) {
                        replace_one = true;
                    }
                    if ui::button_sized(ui, "All", None, true, Some(48.0)) {
                        replace_all = true;
                    }
                });
            })
            .response;
        self.find.rect = Some(area.rect);

        if !matches.is_empty() && step != 0 {
            let n = matches.len() as i32;
            self.find.current = ((self.find.current as i32 + step).rem_euclid(n)) as usize;
        }
        if let Some(&(id, start)) = matches.get(self.find.current) {
            if step != 0 {
                // show the result, keeping the keyboard in the field
                self.ed.select(id, start, start + n_len);
                self.find.focus = true;
            }
        }
        if replace_one {
            if let Some(&(id, start)) = matches.get(self.find.current) {
                if let Some(i) = self.doc.index_of(id) {
                    self.checkpoint();
                    let chars: Vec<char> = self.doc.blocks[i].text.chars().collect();
                    let end = (start + n_len).min(chars.len());
                    let mut t: String = chars[..start].iter().collect();
                    t.push_str(&self.find.with);
                    t.extend(chars[end..].iter());
                    if self.doc.blocks[i].element.is_upper() {
                        t = t.to_uppercase();
                    }
                    self.doc.blocks[i].text = t;
                    self.mark_changed();
                    self.snapshot_due = false;
                    self.baseline = self.doc.clone();
                }
            }
        }
        if replace_all && !self.find.needle.is_empty() {
            self.checkpoint();
            let n = self.doc.replace_all(&self.find.needle.clone(), &self.find.with.clone());
            if n > 0 {
                self.mark_changed();
                self.snapshot_due = false;
                self.baseline = self.doc.clone();
                self.ed.fresh = true;
                self.deck.ok(
                    &format!("Replaced {n}"),
                    "Ctrl+Z puts them back",
                );
            } else {
                self.undo.pop();
                self.deck.say("Nothing to replace", "", Tone::Info);
            }
        }
        if close {
            self.find.open = false;
        }
    }

    // ---------- popovers ----------

    fn popovers(&mut self, ctx: &egui::Context) {
        if self.popover != Some(Popover::Settings) {
            // seeded shut, so the window rises again every time it opens
            ctx.animate_bool_with_time(egui::Id::new("ns-settings-veil"), false, 0.0);
            ctx.animate_bool_with_time(egui::Id::new("ns-settings-rise"), false, 0.0);
        }
        match self.popover {
            Some(Popover::Menu) => self.more_menu(ctx),
            Some(Popover::Settings) => self.settings_panel(ctx),
            Some(Popover::Snapshots) => self.snapshots_panel(ctx),
            Some(Popover::Elements) => self.elements_menu(ctx),
            None => {}
        }
        self.row_menu(ctx);
    }

    fn more_menu(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let anchor = self.menu_button_rect;
        let area = egui::Area::new(egui::Id::new("ns-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(
                (anchor.right() - 236.0).max(screen.left() + 8.0),
                theme::TOPBAR_H + 4.0,
            ))
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    ui.set_width(ui::menu_width(
                        ui,
                        &[
                            "Export Final Draft (.fdx)",
                            "Import a script…",
                            "Open the exports folder",
                            "Snapshots…",
                        ],
                    ));
                    let mut rects = Vec::new();
                    ui::section(ui, "Export");
                    if ui::menu_item(ui, &mut rects, "Export PDF", Icon::Print, false) {
                        self.export(Format::Pdf, false);
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Export Final Draft (.fdx)", Icon::Download, false) {
                        self.export(Format::FinalDraft, false);
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Export Fountain", Icon::Download, false) {
                        self.export(Format::Fountain, false);
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Export plain text", Icon::File, false) {
                        self.export(Format::Text, false);
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Open the exports folder", Icon::Folder, false) {
                        storage::open_with_desktop(&storage::exports_dir());
                        self.popover = None;
                    }
                    ui::separator(ui);
                    if ui::menu_item(ui, &mut rects, "Import a script…", Icon::Upload, false) {
                        self.start_import();
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Take a snapshot", Icon::Copy, false) {
                        self.take_snapshot();
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Snapshots…", Icon::History, false) {
                        self.popover = Some(Popover::Snapshots);
                        self.popover_frame = self.frame_no;
                    }
                    ui::separator(ui);
                    if ui::menu_item(ui, &mut rects, "Find and replace", Icon::Search, false) {
                        self.mode = Mode::Write;
                        self.find.open = true;
                        self.find.focus = true;
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Rename script", Icon::Pencil, false) {
                        let seed = self.doc.meta.title.clone();
                        self.deck.prompt(
                            "Rename script",
                            "The file on disk follows the title.",
                            "Script title",
                            &seed,
                            "Rename",
                            Ask::RenameScript,
                        );
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Duplicate", Icon::Copy, false) {
                        if let Some(path) = self.path.clone() {
                            self.duplicate(&path);
                        }
                        self.popover = None;
                    }
                    let starred = self.doc.meta.starred;
                    if ui::menu_item(
                        ui,
                        &mut rects,
                        if starred { "Remove the star" } else { "Star this script" },
                        if starred { Icon::Star } else { Icon::StarFilled },
                        false,
                    ) {
                        if let Some(path) = self.path.clone() {
                            self.toggle_star(&path);
                        }
                        self.popover = None;
                    }
                    if ui::menu_item(ui, &mut rects, "Show the file", Icon::FolderOpen, false) {
                        match &self.path {
                            Some(pth) => storage::reveal_in_file_manager(pth),
                            None => storage::open_with_desktop(&storage::scripts_dir()),
                        }
                        self.popover = None;
                    }
                    ui::separator(ui);
                    if ui::menu_item(ui, &mut rects, "Delete script", Icon::Trash, true) {
                        if let Some(path) = self.path.clone() {
                            self.ask_delete(path);
                        }
                        self.popover = None;
                    }
                    self.menu_item_rects = rects;
                });
            })
            .response;
        self.close_popover_if_outside(ctx, &[area.rect, self.menu_button_rect]);
    }

    fn snapshots_panel(&mut self, ctx: &egui::Context) {
        let p = pal();
        let screen = ctx.screen_rect();
        let anchor = self.menu_button_rect;
        let snaps = self
            .path
            .as_ref()
            .map(|x| storage::list_snapshots(x))
            .unwrap_or_default();
        let area = egui::Area::new(egui::Id::new("ns-snapshots"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(
                (anchor.right() - 280.0).max(screen.left() + 8.0),
                theme::TOPBAR_H + 4.0,
            ))
            .show(ctx, |ui| {
                ui::popover_frame()
                    .inner_margin(egui::Margin::symmetric(10.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_width(260.0);
                        ui.horizontal(|ui| {
                            icons::show(ui, Icon::History, 14.0, p.primary);
                            ui.label(
                                egui::RichText::new("Snapshots")
                                    .font(theme::font_semi(theme::T_H - 1.0))
                                    .color(p.text),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Dated copies of this script. Restoring one keeps what is there now as a snapshot too.")
                                .font(theme::font(theme::T_CAP))
                                .color(p.text_faint),
                        );
                        ui.add_space(6.0);
                        let mut rects = Vec::new();
                        if snaps.is_empty() {
                            ui.label(
                                egui::RichText::new("None yet.")
                                    .font(theme::font(theme::T_SM))
                                    .color(p.text_dim),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for s in &snaps {
                                    if ui::menu_item(
                                        ui,
                                        &mut rects,
                                        &format!("{}  ·  {} pg", s.when, s.pages),
                                        Icon::History,
                                        false,
                                    ) {
                                        self.ask_restore(s);
                                        self.popover = None;
                                    }
                                }
                            });
                        ui::separator(ui);
                        if ui::button_sized(ui, "Take a snapshot now", Some(Icon::Plus), true, Some(ui.available_width())) {
                            self.take_snapshot();
                        }
                    });
            })
            .response;
        self.close_popover_if_outside(ctx, &[area.rect, self.menu_button_rect]);
    }

    fn elements_menu(&mut self, ctx: &egui::Context) {
        let anchor = self.element_button_rect;
        let current = self.ed.current_element(&self.doc);
        let area = egui::Area::new(egui::Id::new("ns-elements"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(anchor.left(), anchor.bottom() + 6.0))
            .show(ctx, |ui| {
                ui::popover_frame().show(ui, |ui| {
                    let labels: Vec<String> = Element::ALL
                        .iter()
                        .enumerate()
                        .map(|(k, e)| format!("{}  ·  Ctrl+{}", e.label(), k + 1))
                        .collect();
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    ui.set_width(ui::menu_width(ui, &refs));
                    let mut rects = Vec::new();
                    for (k, e) in Element::ALL.iter().enumerate() {
                        let icon = if current == Some(*e) { Icon::Check } else { Icon::Minus };
                        if ui::menu_item(ui, &mut rects, &labels[k], icon, false) {
                            if editor::set_element(&mut self.doc, &mut self.ed, *e) {
                                self.mark_changed();
                            }
                            self.popover = None;
                        }
                    }
                });
            })
            .response;
        self.close_popover_if_outside(ctx, &[area.rect, self.element_button_rect]);
    }

    /// Settings, as a window of its own over the app — the categories down
    /// the left, the one you are in on the right. It opens on a short ease,
    /// dims what is behind it, and closes with Esc, its close button, or a
    /// click outside it.
    fn settings_panel(&mut self, ctx: &egui::Context) {
        let p = pal();
        let screen = ctx.screen_rect();
        let before = self.settings.clone();
        const TABS: [(&str, Icon); 7] = [
            ("Appearance", Icon::Palette),
            ("The page", Icon::File),
            ("Colour", Icon::Eye),
            ("Panels", Icon::Sidebar),
            ("Writing & saving", Icon::Pencil),
            ("YouTrack", Icon::External),
            ("About", Icon::Info),
        ];

        // the room behind goes quiet
        let veil = anim::ease(ctx, "ns-settings-veil", true, 0.24);
        let mut close = false;
        // The scrim sits a layer below the window. Two areas on one layer are
        // ordered by which was clicked last — click the scrim once and it
        // would come up over the window the next time it opened.
        egui::Area::new(egui::Id::new("ns-settings-scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .interactable(true)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(if chrome::maximized(ctx) { 0.0 } else { theme::R_WINDOW }),
                    Color32::from_black_alpha((120.0 * veil) as u8),
                );
                if resp.clicked() && self.frame_no > self.popover_frame + 1 {
                    close = true;
                }
            });

        let size = Vec2::new(
            (screen.width() - 80.0).clamp(560.0, 780.0),
            (screen.height() - 80.0).clamp(420.0, 620.0),
        );
        // arrives with a short settle, around 300 ms, and no more
        let t = anim::ease(ctx, "ns-settings-rise", true, 0.30);
        let scale = 0.97 + 0.03 * t;
        let rect = Rect::from_center_size(screen.center(), size * scale);
        let tab = self.settings_tab.min(TABS.len() - 1);

        egui::Area::new(egui::Id::new("ns-settings"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_opacity(0.2 + 0.8 * t);
                let (win, _) = ui.allocate_exact_size(rect.size(), Sense::click());
                // a window you work in is solid, lifted on a layered shadow
                theme::lift_shadow(ui.painter(), win, theme::R_ISLAND, 1.0);
                ui.painter().rect_filled(win, egui::Rounding::same(theme::R_ISLAND), p.solid);
                ui.painter().rect_stroke(
                    win.shrink(0.5),
                    egui::Rounding::same(theme::R_ISLAND),
                    Stroke::new(1.0_f32, theme::wash(Color32::WHITE, if p.dark { 18 } else { 120 })),
                );

                // ---- the head ----
                let head = Rect::from_min_size(win.min, Vec2::new(win.width(), 56.0));
                let mut h = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(head.shrink2(Vec2::new(20.0, 0.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );
                h.spacing_mut().item_spacing.x = 10.0;
                icons::show(&mut h, Icon::Settings, 16.0, p.primary_light);
                h.label(
                    egui::RichText::new("Settings")
                        .font(theme::font_semi(theme::T_H))
                        .color(p.text),
                );
                h.label(
                    egui::RichText::new(format!("{} · {}", pal().id.name(), if p.dark { "dark" } else { "light" }))
                        .font(theme::font(theme::T_CAP))
                        .color(p.text_faint),
                );
                h.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui::icon_button(ui, Icon::Close, "Close · Esc", 28.0) {
                        close = true;
                    }
                });

                // ---- the categories ----
                let nav = Rect::from_min_max(
                    Pos2::new(win.left() + 14.0, head.bottom()),
                    Pos2::new(win.left() + 14.0 + 184.0, win.bottom() - 14.0),
                );
                let mut n = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(nav)
                        .layout(Layout::top_down(Align::Min)),
                );
                n.spacing_mut().item_spacing.y = 2.0;
                let mut picked = None;
                for (k, (label, icon)) in TABS.iter().enumerate() {
                    if settings_tab_row(&mut n, label, *icon, k == tab) {
                        picked = Some(k);
                    }
                }
                if let Some(k) = picked {
                    self.settings_tab = k;
                }
                // foot of the categories: what this is
                let foot = Pos2::new(nav.left() + 10.0, nav.bottom() - 10.0);
                ui.painter().text(
                    foot,
                    egui::Align2::LEFT_BOTTOM,
                    format!("Northstar {}", env!("CARGO_PKG_VERSION")),
                    theme::font_mono(theme::T_MICRO),
                    p.text_faint,
                );

                // ---- the pane ----
                let pane = Rect::from_min_max(
                    Pos2::new(nav.right() + 14.0, head.bottom()),
                    Pos2::new(win.right() - 10.0, win.bottom() - 10.0),
                );
                ui.painter().rect_filled(
                    pane.expand2(Vec2::new(0.0, 0.0)),
                    egui::Rounding::same(theme::R_MD),
                    p.solid_hi,
                );
                let mut pane_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(pane.shrink2(Vec2::new(18.0, 14.0)))
                        .layout(Layout::top_down(Align::Min)),
                );
                pane_ui.label(
                    egui::RichText::new(TABS[tab].0)
                        .font(theme::font_semi(theme::T_H - 1.0))
                        .color(p.text),
                );
                pane_ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .id_salt(("ns-settings-pane", tab))
                    .auto_shrink([false, false])
                    .show(&mut pane_ui, |ui| {
                        // room on the right for the floating scroll bar
                        ui.set_max_width(ui.available_width() - 14.0);
                        self.settings_body(ui, tab);
                        ui.add_space(8.0);
                    });
            });

        if close {
            self.popover = None;
        }
        if self.settings != before {
            self.ed.font_px = self.settings.page_px;
            self.theme_dirty = true;
            self.save_settings();
        }
    }

    /// One category of settings, drawn into the right-hand pane.
    fn settings_body(&mut self, ui: &mut egui::Ui, tab: usize) {
        let p = pal();
        let tess = storage::tesseract_settings_path().exists();
        let _ = tess;
        match tab {
            0 => {
            ui::section(ui, "Theme");
            let mut v = self.settings.match_tesseract;
            if ui::toggle_row(
                ui,
                "Match Tesseract",
                if tess {
                    "Take theme, glass and motion from Tesseract, live."
                } else {
                    "Tesseract has not been run on this machine yet."
                },
                &mut v,
            ) {
                self.settings.match_tesseract = v;
                self.tess_seen = None;
                self.tess_checked = None;
            }
            ui.add_space(4.0);
            let mut picked = None;
            for t in theme::ThemeId::ALL {
                if theme_row(ui, t, self.settings.theme == t, !self.settings.light_mode) {
                    picked = Some(t);
                }
            }
            if let Some(t) = picked {
                self.settings.theme = t;
                self.stop_matching();
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let dark = !self.settings.light_mode;
                icons::show(ui, if dark { Icon::Moon } else { Icon::Sun }, 14.0, p.text_dim);
                ui.label(
                    egui::RichText::new(if dark { "Dark" } else { "Light" })
                        .font(theme::font_med(theme::T_SM))
                        .color(p.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut on = dark;
                    if ui::toggle(ui, &mut on) {
                        self.settings.light_mode = !on;
                        self.stop_matching();
                    }
                });
            });

                        ui::separator(ui);
            ui::section(ui, "Motion & glass");
            let mut v = self.settings.animations;
            if ui::toggle_row(ui, "Animations", "Every transition in the app, at once.", &mut v) {
                self.settings.animations = v;
                self.stop_matching();
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Window blur")
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            ui.horizontal(|ui| {
                for m in BlurMode::ALL {
                    let on = self.settings.blur == m;
                    if ui::button(ui, m.label(), None, on) && !on {
                        self.settings.blur = m;
                        self.stop_matching();
                    }
                }
            });
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    blur::session_name(),
                    self.blur.state.describe()
                ))
                .font(theme::font(theme::T_MICRO))
                .color(p.text_faint),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Glass")
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            let mut g = self.settings.glass_opacity;
            if ui::slider(ui, &mut g, 0.35..=1.0) {
                self.settings.glass_opacity = g;
                self.stop_matching();
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("Interface size · {:.0}%", self.settings.font_scale * 100.0))
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            let mut fs = self.settings.font_scale;
            if ui::slider(ui, &mut fs, 0.8..=1.4) {
                self.settings.font_scale = (fs * 20.0).round() / 20.0;
            }

                        }
            1 => {
            ui.label(
                egui::RichText::new(format!("Page text · {:.0} pt · Ctrl+plus / minus", self.settings.page_px))
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            let mut px = self.settings.page_px;
            if ui::slider(ui, &mut px, 11.0..=26.0) {
                self.settings.page_px = px.round();
            }
            ui.add_space(4.0);
            let mut v = self.settings.page_breaks;
            if ui::toggle_row(ui, "Page breaks", "Where each printed page begins, as you write.", &mut v) {
                self.settings.page_breaks = v;
            }
            let mut v = self.settings.scene_numbers;
            if ui::toggle_row(ui, "Scene numbers", "In the gutter, the navigator and the PDF.", &mut v) {
                self.settings.scene_numbers = v;
            }
            let mut v = self.settings.smart_type;
            if ui::toggle_row(ui, "SmartType", "Finish names, places and transitions. Tab takes it.", &mut v) {
                self.settings.smart_type = v;
            }
            let mut v = self.settings.typewriter;
            if ui::toggle_row(ui, "Typewriter scrolling", "Keep the line you are on mid-page.", &mut v) {
                self.settings.typewriter = v;
            }

                        }
            2 => {
            let mut v = self.settings.element_colors;
            if ui::toggle_row(
                ui,
                "Element colours",
                "A faint band behind each block, one colour per element. Never in Reading mode or on paper.",
                &mut v,
            ) {
                self.settings.element_colors = v;
            }
            let mut v = self.settings.character_colors;
            if ui::toggle_row(
                ui,
                "Character colours",
                "Each speaker's name and lines in a colour of their own.",
                &mut v,
            ) {
                self.settings.character_colors = v;
            }
            if self.settings.character_colors {
                let mut v = self.settings.character_colors_reading;
                if ui::toggle_row(ui, "…in Reading mode too", "", &mut v) {
                    self.settings.character_colors_reading = v;
                }
                ui.horizontal(|ui| {
                    // a live sample of the first few speakers
                    let voices = theme::character_colors(
                        &self.layout.speakers,
                        self.settings.colour_seed,
                    );
                    for name in self.layout.speakers.iter().take(8) {
                        if let Some(c) = voices.get(name) {
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 5.0, *c);
                            resp.on_hover_text(name.clone());
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::button(ui, "Shuffle", Some(Icon::Refresh), false) {
                            self.settings.colour_seed = self.settings.colour_seed.wrapping_add(1);
                        }
                    });
                });
            }

                        }
            3 => {
            ui::section(ui, "Panels");
            for (label, hint, key) in [
                ("Details", "Title page and figures · Ctrl+I", 0),
                ("Scenes", "The navigator · Ctrl+Shift+I", 1),
                ("Cast", "Who speaks, and how much", 2),
                ("Library", "Your scripts · Ctrl+B", 3),
            ] {
                let slot = match key {
                    0 => &mut self.settings.show_details,
                    1 => &mut self.settings.show_scenes,
                    2 => &mut self.settings.show_cast,
                    _ => &mut self.settings.show_library,
                };
                let mut v = *slot;
                if ui::toggle_row(ui, label, hint, &mut v) {
                    *slot = v;
                }
            }

                        ui::separator(ui);
            ui::section(ui, "Library");
            let mut v = self.settings.starred_first;
            if ui::toggle_row(ui, "Starred at the top", "", &mut v) {
                self.settings.starred_first = v;
            }
            ui.label(
                egui::RichText::new("Sort by")
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            ui.horizontal_wrapped(|ui| {
                for s in SortBy::ALL {
                    let on = self.settings.sort_by == s;
                    if ui::button(ui, s.label(), None, on) && !on {
                        self.settings.sort_by = s;
                    }
                }
            });

                        }
            4 => {
            ui::section(ui, "Writing");
            ui.label(
                egui::RichText::new(if self.settings.session_goal == 0 {
                    "Session goal · off".to_string()
                } else {
                    format!("Session goal · {} words", self.settings.session_goal)
                })
                .font(theme::font_med(theme::T_SM))
                .color(p.text),
            );
            let mut g = self.settings.session_goal as f32;
            if ui::slider(ui, &mut g, 0.0..=3000.0) {
                self.settings.session_goal = ((g / 50.0).round() * 50.0) as u32;
            }

                        ui::separator(ui);
            ui::section(ui, "Starting up & saving");
            let mut v = self.settings.splash;
            if ui::toggle_row(ui, "Splash screen", "", &mut v) {
                self.settings.splash = v;
            }
            ui.label(
                egui::RichText::new("After Quick Export")
                    .font(theme::font_med(theme::T_SM))
                    .color(p.text),
            );
            ui.horizontal(|ui| {
                for a in AfterExport::ALL {
                    let on = self.settings.after_export == a;
                    if ui::button(ui, a.label(), None, on) && !on {
                        self.settings.after_export = a;
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(format!(
                    "Autosave after {} ms of quiet",
                    self.settings.autosave_ms
                ))
                .font(theme::font_med(theme::T_SM))
                .color(p.text),
            );
            let mut ms = self.settings.autosave_ms as f32;
            if ui::slider(ui, &mut ms, 200.0..=5000.0) {
                self.settings.autosave_ms = (ms / 50.0).round() as u64 * 50;
            }

                        }
            5 => {
            ui.label(
                egui::RichText::new("Where View on YouTrack goes")
                    .font(theme::font(theme::T_CAP))
                    .color(p.text_faint),
            );
            ui.add_space(2.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.settings.youtrack_url)
                    .id(egui::Id::new("ns-youtrack-url"))
                    .desired_width(f32::INFINITY)
                    .font(theme::font_mono(theme::T_CAP))
                    .margin(egui::Margin::symmetric(10.0, 6.0))
                    .hint_text(crate::settings::DEFAULT_YOUTRACK),
            );
            if self.settings.youtrack_url.trim() != crate::settings::DEFAULT_YOUTRACK {
                ui.add_space(2.0);
                if ui::chip_button(ui, "reset to the default") {
                    self.settings.youtrack_url = crate::settings::DEFAULT_YOUTRACK.to_string();
                }
            }

                        }
            6 => {
            ui::section(ui, "Where things live");
            ui.label(
                egui::RichText::new(short_home(&storage::scripts_dir()))
                    .font(theme::font_mono(theme::T_MICRO))
                    .color(p.text_faint),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui::button(ui, "Open library", Some(Icon::Folder), false) {
                    storage::open_with_desktop(&storage::scripts_dir());
                }
                if ui::button(ui, "Exports", Some(Icon::Download), false) {
                    storage::open_with_desktop(&storage::exports_dir());
                }
            });
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!(
                    "Page font: {} · Northstar {} · MarkedExiled Software",
                    theme::PAGE_FONT_NAME,
                    env!("CARGO_PKG_VERSION")
                ))
                .font(theme::font(theme::T_MICRO))
                .color(p.text_faint),
            );
            }
            _ => {}
        }
    }

    /// A look chosen by hand here stops the look being taken from Tesseract.
    fn stop_matching(&mut self) {
        if self.settings.match_tesseract && storage::tesseract_settings_path().exists() {
            self.settings.match_tesseract = false;
            self.deck.say(
                "No longer matching Tesseract",
                "Turn Match Tesseract back on in Settings to follow it again.",
                Tone::Info,
            );
        }
    }

    // ---------- keys ----------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let hit =
            |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));

        if hit(Modifiers::COMMAND, Key::S) {
            self.save(true);
            self.deck.ok("Saved", "");
        }
        if hit(Modifiers::COMMAND, Key::N) {
            self.new_script();
        }
        if hit(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || hit(Modifiers::COMMAND, Key::Y) {
            self.redo();
        }
        if hit(Modifiers::COMMAND, Key::Z) {
            self.undo();
        }
        if hit(Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
            self.settings.show_library = true;
            self.focus_search = true;
        } else if hit(Modifiers::COMMAND, Key::F) || hit(Modifiers::COMMAND, Key::H) {
            self.mode = Mode::Write;
            self.find.open = true;
            self.find.focus = true;
        }
        if hit(Modifiers::COMMAND, Key::B) {
            self.settings.show_library = !self.settings.show_library;
            self.save_settings();
        }
        // the more specific chord first: egui lets an extra Shift through
        if hit(Modifiers::COMMAND | Modifiers::SHIFT, Key::I) {
            self.settings.show_scenes = !self.settings.show_scenes;
            self.save_settings();
        } else if hit(Modifiers::COMMAND, Key::I) {
            self.settings.show_details = !self.settings.show_details;
            self.save_settings();
        }
        if hit(Modifiers::COMMAND, Key::E) {
            self.export(Format::Pdf, true);
        }
        if hit(Modifiers::COMMAND, Key::Comma) {
            self.toggle_popover(Popover::Settings);
        }
        if hit(Modifiers::COMMAND, Key::Period) {
            self.focus_mode = !self.focus_mode;
        }
        if hit(Modifiers::COMMAND, Key::G) {
            self.mode = Mode::from_index((self.mode.index() + 1) % 3);
        }
        if hit(Modifiers::COMMAND, Key::Enter) {
            self.new_scene();
        }
        let mut px = None;
        if hit(Modifiers::COMMAND, Key::Plus) || hit(Modifiers::COMMAND, Key::Equals) {
            px = Some((self.ed.font_px + 1.0).min(26.0));
        }
        if hit(Modifiers::COMMAND, Key::Minus) {
            px = Some((self.ed.font_px - 1.0).max(11.0));
        }
        if let Some(px) = px {
            self.ed.font_px = px;
            self.settings.page_px = px;
            self.save_settings();
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
            if hit(Modifiers::COMMAND, *k) {
                if let Some(e) = Element::from_digit(i + 1) {
                    if editor::set_element(&mut self.doc, &mut self.ed, e) {
                        self.mark_changed();
                    }
                }
            }
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if self.popover.is_some() {
                self.popover = None;
            } else if self.find.open {
                self.find.open = false;
            } else if self.focus_mode {
                self.focus_mode = false;
            }
        }
    }

    // ---------- the frame ----------

    pub fn frame(&mut self, ctx: &egui::Context) {
        self.frame_no = self.frame_no.wrapping_add(1);

        self.follow_tesseract();
        self.watch_library();
        if self.theme_dirty {
            theme::set_palette(self.settings.theme, !self.settings.light_mode);
            anim::set_enabled(self.settings.animations);
            self.sync_glass();
            theme::apply(ctx);
            ctx.set_zoom_factor(self.settings.font_scale);
            self.theme_dirty = false;
        }

        // The ground the islands sit on, painted straight onto the background
        // layer: a second CentralPanel would fight the real one.
        chrome::backdrop(ctx, self.window_ground());
        // keep the compositor's blurred area cut to the window's own outline
        {
            let ppp = ctx.pixels_per_point();
            let size = ctx.screen_rect().size() * ppp;
            let radius = if chrome::maximized(ctx) { 0.0 } else { theme::R_WINDOW * ppp };
            self.blur.reshape(size.x as u32, size.y as u32, radius as u32);
        }

        if !self.deck.asking() {
            self.shortcuts(ctx);
        }

        // files dropped on the window, or handed over on the command line: a
        // script already in the library is opened, anything else imported
        let mut dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        dropped.append(&mut self.arrivals);
        for path in dropped {
            let in_library = path
                .canonicalize()
                .ok()
                .zip(storage::scripts_dir().canonicalize().ok())
                .map(|(f, lib)| f.starts_with(lib))
                .unwrap_or(false);
            if in_library {
                self.open(path);
            } else {
                self.import_path(&path);
            }
        }
        if let Some(rx) = &self.importing {
            match rx.try_recv() {
                Ok(Ok(Some(path))) => {
                    self.importing = None;
                    self.import_path(&path);
                }
                Ok(Ok(None)) => self.importing = None,
                Ok(Err(e)) => {
                    self.importing = None;
                    self.deck.warn("No file picker", &e);
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(200)),
                Err(mpsc::TryRecvError::Disconnected) => self.importing = None,
            }
        }

        // the centre page eases in whenever it changes what it is showing
        let key = format!(
            "{:?}|{}",
            self.mode,
            self.path.as_ref().map(|x| x.display().to_string()).unwrap_or_default()
        );
        if key != self.page_key {
            self.page_key = key;
            self.page_born = Instant::now();
            if self.mode == Mode::Read {
                // Reading mode opens on the page you were writing
                self.pages.show_block = self.ed.focus_block;
            }
        }

        self.refresh_layout(false);

        self.top_panel(ctx);
        self.browser(ctx);
        self.aside(ctx);
        self.page(ctx);
        self.find_bar(ctx);
        self.popovers(ctx);
        chrome::resize_handles(ctx);

        if let Some(answer) = self.deck.show(ctx) {
            self.answer(answer);
        }

        if let Some(splash) = self.splash.as_mut() {
            if splash.show(ctx) {
                if self.frame_no < 3 {
                    crate::splash::centre(ctx, crate::splash::CARD);
                }
            } else {
                self.splash = None;
                let full = Vec2::new(1320.0, 900.0);
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(880.0, 580.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(full));
                crate::splash::centre(ctx, full);
            }
        }

        // undo checkpoints and autosave, both keyed off a pause in typing
        if let Some(last) = self.last_change {
            if self.snapshot_due && last.elapsed() > SNAPSHOT_IDLE {
                self.snapshot_now();
            }
            if self.dirty && last.elapsed() > Duration::from_millis(self.settings.autosave_ms) {
                self.save(false);
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.save(true);
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

/// The seam the headless UI tests drive the app through. Everything here reads
/// or nudges state the real UI reaches by other means.
#[allow(dead_code)]
impl App {
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn set_mode(&mut self, m: Mode) {
        self.mode = m;
    }
    pub fn doc(&self) -> &Document {
        &self.doc
    }
    pub fn doc_mut(&mut self) -> &mut Document {
        self.mark_changed();
        &mut self.doc
    }
    pub fn path(&self) -> Option<PathBuf> {
        self.path.clone()
    }
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn focus_block(&self) -> Option<u64> {
        self.ed.focus_block
    }
    pub fn focus_on(&mut self, id: u64) {
        self.ed.focus(id, Caret::End);
    }
    pub fn debug_pending_ask(&self) -> bool {
        self.deck.asking()
    }
    pub fn debug_answer_yes(&mut self, text: Option<&str>) {
        if let Some(a) = self.deck.take_top(text.map(|s| s.to_string())) {
            self.answer(a);
        }
    }
    pub fn debug_popover_open(&self) -> bool {
        self.popover.is_some()
    }
    pub fn debug_settings_tab(&self) -> usize {
        self.settings_tab
    }
    pub fn debug_settings_open(&self) -> bool {
        self.popover == Some(Popover::Settings)
    }
    pub fn debug_menu_button(&self) -> Rect {
        self.menu_button_rect
    }
    pub fn debug_settings_button(&self) -> Rect {
        self.settings_button_rect
    }
    pub fn debug_menu_item(&self, n: usize) -> Option<Rect> {
        self.menu_item_rects.get(n).copied()
    }
    pub fn debug_card_rect(&self, n: usize) -> Option<Rect> {
        self.cards.last_rects.get(n).copied()
    }
    pub fn debug_sheet_count(&self) -> usize {
        self.pages.sheets
    }
    pub fn debug_sheet_on_show(&self) -> usize {
        self.pages.current
    }
    pub fn debug_sheet_rect(&self) -> Rect {
        self.pages.sheet_rect
    }
    pub fn debug_page_rect(&self) -> Rect {
        self.page_rect
    }
    pub fn debug_find_open(&self) -> bool {
        self.find.open
    }
    pub fn debug_set_find(&mut self, needle: &str, with: &str) {
        self.find.open = true;
        self.find.needle = needle.to_string();
        self.find.with = with.to_string();
    }
    pub fn debug_import(&mut self, path: &Path) {
        self.import_path(path);
    }
    pub fn debug_open(&mut self, path: PathBuf) {
        self.open(path);
    }
    pub fn debug_save(&mut self) {
        self.save(true);
    }
    pub fn debug_undo(&mut self) {
        self.undo();
    }
    pub fn debug_snapshot(&mut self) {
        self.take_snapshot();
    }
    pub fn debug_ask_delete_scene(&mut self, n: usize) {
        self.ask_delete_scene(n);
    }
    pub fn debug_ask_restore(&mut self, s: &storage::Snapshot) {
        self.ask_restore(s);
    }
    pub fn debug_page_starts(&mut self) -> Vec<(u64, usize)> {
        self.layout_stale = true;
        self.refresh_layout(true);
        self.layout.page_starts.iter().map(|(a, b)| (*a, *b)).collect()
    }
    pub fn debug_set_theme(&mut self, t: theme::ThemeId, light: bool) {
        self.settings.theme = t;
        self.settings.light_mode = light;
        self.settings.match_tesseract = false;
        self.theme_dirty = true;
    }
    pub fn debug_focus_mode(&mut self, on: bool) {
        self.focus_mode = on;
    }
    pub fn debug_row_rect(&self, path: &Path) -> Option<Rect> {
        self.rail.last_rects.get(path).copied()
    }
    pub fn debug_foot_rect(&self) -> Rect {
        self.foot_rect
    }
    pub fn debug_aside_rect(&self) -> Rect {
        self.aside_rect
    }
    pub fn debug_aside_islands(&self) -> Vec<Rect> {
        self.aside_islands.clone()
    }
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
    pub fn debug_session_words(&self) -> i64 {
        self.session_words
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Always transparent. The ground is painted by `chrome::backdrop` as a
        // rounded rectangle — clear to an opaque colour instead and the window
        // gets square corners, whatever we draw on top of it.
        [0.0, 0.0, 0.0, 0.0]
    }
}

// ---------- small pieces ----------

/// The one button that should catch your eye when the app opens.
fn new_script_button(ui: &mut egui::Ui, width: f32) -> bool {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);

    // The primary action is told apart by contrast and colour, not by light:
    // on hover it shifts into the stars' own gradient, with no glow.
    theme::grad_rect(
        ui.painter(),
        rect,
        theme::R_CTRL,
        theme::mix(p.prim_grad.1, p.sec_grad.1, hot),
        theme::mix(p.prim_grad.0, p.sec_grad.0, hot),
        Vec2::new(0.25, 1.0),
    );
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(theme::R_CTRL),
        Stroke::new(1.0_f32, theme::wash(theme::lighten(p.sec_light, 0.2), (200.0 * hot) as u8)),
    );

    let ink = theme::on(theme::mix(p.prim_grad.0, p.sec_grad.0, hot));
    icons::draw(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(rect.center().x - 40.0, rect.center().y),
            Vec2::splat(13.0),
        ),
        Icon::Plus,
        ink,
    );
    ui.painter().text(
        Pos2::new(rect.center().x - 28.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "New script",
        theme::font_semi(theme::T_SM),
        ink,
    );
    resp.on_hover_text("New script  ·  Ctrl+N").clicked()
}

/// One element in the ribbon's palette. Nothing until you reach for it; the
/// element the caret is in stands lit.
fn element_chip(ui: &mut egui::Ui, e: Element, on: bool) -> bool {
    let p = pal();
    let galley = ui.painter().layout_no_wrap(
        e.short().to_string(),
        theme::font_med(theme::T_LABEL - 1.0),
        p.text_dim,
    );
    let w = galley.rect.width() + 26.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    theme::hover_surface(ui.painter(), rect, theme::R_SM, p.primary, hot, on);
    let dot = theme::element_color(e);
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 10.0, rect.center().y),
        2.4,
        theme::wash(dot, if on { 255 } else { (140.0 + 100.0 * hot) as u8 }),
    );
    let ink = if on {
        p.primary_light
    } else {
        theme::mix(p.text_dim, p.text, hot)
    };
    ui.painter().galley(
        Pos2::new(rect.left() + 17.0, rect.center().y - galley.rect.height() * 0.5),
        galley,
        ink,
    );
    let k = Element::ALL.iter().position(|x| *x == e).unwrap_or(0) + 1;
    resp.on_hover_text(format!("{}  ·  Ctrl+{k}", e.label())).clicked()
}

/// How far a script sits in from the header of the section it is under.
const INDENT: f32 = 20.0;

fn row_id(path: &Path) -> egui::Id {
    egui::Id::new(("ns-script-row", path.display().to_string()))
}

struct HeaderOut {
    rect: Rect,
    clicked: bool,
}

/// A section heading in the library: the starred set, and everything.
fn section_header(
    ui: &mut egui::Ui,
    label: &str,
    icon: Icon,
    count: usize,
    collapsed: &mut bool,
    tint: Option<Color32>,
) -> HeaderOut {
    let p = pal();
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    let col = tint.unwrap_or(p.text_dim);
    if hot > 0.02 {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(theme::R_SM),
            theme::wash(col, (26.0 * hot) as u8),
        );
    }
    let turn = anim::to(ui.ctx(), resp.id.with("turn"), if *collapsed { 0.0 } else { 1.0 }, 0.14);
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 11.0, rect.center().y), Vec2::splat(11.0)),
        if turn > 0.5 { Icon::ChevronDown } else { Icon::ChevronRight },
        p.text_faint,
    );
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 28.0, rect.center().y), Vec2::splat(13.0)),
        icon,
        col,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 42.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        ui::elide(label, ((rect.width() - 96.0) / 6.6) as usize),
        theme::font_semi(theme::T_SM),
        theme::mix(p.text_dim, p.text, hot),
    );
    ui.painter().text(
        Pos2::new(rect.right() - 13.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{count}"),
        theme::font_mono(theme::T_MICRO),
        p.text_faint,
    );
    let clicked = resp.clicked();
    if clicked {
        *collapsed = !*collapsed;
    }
    ui.add_space(3.0);
    HeaderOut { rect, clicked }
}

/// The line that ties a section's header to the scripts under it.
fn draw_branch(ui: &egui::Ui, slot: egui::layers::ShapeIdx, head: Rect, kids: &[Rect], tint: Color32) {
    if kids.is_empty() {
        return;
    }
    let col = theme::wash(tint, 70);
    let x = head.left() + 13.0;
    let last = kids[kids.len() - 1];
    let mut shapes: Vec<egui::Shape> = vec![egui::Shape::line_segment(
        [
            Pos2::new(x, head.bottom() + 1.0),
            Pos2::new(x, last.center().y),
        ],
        Stroke::new(1.0_f32, col),
    )];
    for r in kids {
        shapes.push(egui::Shape::line_segment(
            [Pos2::new(x, r.center().y), Pos2::new(r.left() - 4.0, r.center().y)],
            Stroke::new(1.0_f32, col),
        ));
    }
    ui.painter().set(slot, egui::Shape::Vec(shapes));
}

/// One theme in the Settings list, with a live sample of its two gradients —
/// the same row Tesseract shows, with the star standing in for the rhombus.
fn theme_row(ui: &mut egui::Ui, t: theme::ThemeId, on: bool, dark: bool) -> bool {
    let p = pal();
    let sample = theme::palette(t, dark);
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered() || on, anim::HOVER);

    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(theme::R_SM),
        theme::mix(Color32::TRANSPARENT, sample.primary_quiet_hi, hot),
    );
    ui.painter().rect_stroke(
        rect,
        egui::Rounding::same(theme::R_SM),
        Stroke::new(
            if on { 1.6_f32 } else { 1.0 },
            if on { sample.primary } else { theme::wash(p.line, (200.0 * hot) as u8) },
        ),
    );

    let sw = Rect::from_min_size(rect.left_top() + Vec2::new(8.0, 7.0), Vec2::new(34.0, 20.0));
    theme::grad_rect(ui.painter(), sw, 6.0, sample.prim_grad.1, sample.prim_grad.0, Vec2::new(0.0, 1.0));
    let dot = Pos2::new(sw.right() - 6.0, sw.bottom() - 6.0);
    theme::grad_star(ui.painter(), dot, 7.0, sample.sec_grad.0, sample.sec_grad.1);

    ui.painter().text(
        Pos2::new(rect.left() + 52.0, rect.center().y - 6.5),
        egui::Align2::LEFT_CENTER,
        t.name(),
        theme::font_med(theme::T_SM),
        p.text,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 52.0, rect.center().y + 7.5),
        egui::Align2::LEFT_CENTER,
        t.blurb(),
        theme::font(theme::T_MICRO),
        p.text_faint,
    );
    if on {
        icons::draw(
            ui.painter(),
            Rect::from_center_size(Pos2::new(rect.right() - 16.0, rect.center().y), Vec2::splat(13.0)),
            Icon::Check,
            sample.primary,
        );
    }
    ui.add_space(4.0);
    resp.clicked()
}

/// A child of `ui` shifted down by however far the page still has to travel,
/// clipped to the island so whatever is off the bottom is simply not there.
fn slide_in(ui: &mut egui::Ui, island: Rect, arrive: f32, distance: f32) -> egui::Ui {
    let dy = (1.0 - arrive) * distance;
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(island.translate(Vec2::new(0.0, dy)))
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(island.intersect(ui.clip_rect()));
    child.set_opacity(0.08 + 0.92 * arrive);
    child
}

/// How tall the band across the head of the script is.
const BANNER_H: f32 = 86.0;

/// The banner: a band of frosted glass across the whole page. Its top corners
/// are the island's corners; only the faintest seam where it meets the page.
fn banner_shape(rect: Rect) -> egui::Shape {
    let p = pal();
    let sheen = Color32::WHITE;
    let round = egui::Rounding {
        nw: theme::R_ISLAND,
        ne: theme::R_ISLAND,
        sw: 0.0,
        se: 0.0,
    };
    let pts = {
        let mut v = theme::rounded_poly(
            Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.bottom() + theme::R_ISLAND)),
            theme::R_ISLAND,
        );
        v.retain(|q| q.y <= rect.bottom());
        v
    };
    egui::Shape::Vec(vec![
        egui::Shape::rect_filled(rect, round, theme::glass_body(0.82)),
        theme::grad_poly_shape(
            &pts,
            theme::wash(sheen, if p.dark { 18 } else { 120 }),
            theme::wash(sheen, 0),
            Vec2::new(0.0, 1.0),
        ),
        egui::Shape::line_segment(
            [
                rect.left_bottom() + Vec2::new(theme::R_ISLAND, 0.0),
                rect.right_bottom() - Vec2::new(theme::R_ISLAND, 0.0),
            ],
            Stroke::new(1.0_f32, theme::wash(sheen, if p.dark { 8 } else { 22 })),
        ),
    ])
}

#[allow(dead_code)]
fn empty_state(ui: &mut egui::Ui) {
    let p = pal();
    ui.vertical_centered(|ui| {
        ui.add_space(150.0);
        let (tile, _) = ui.allocate_exact_size(Vec2::splat(78.0), Sense::hover());
        logo::paint(ui.painter(), tile, 0.3 * anim::breathe(ui.ctx(), 4.0));
        ui.add_space(18.0);
        ui.label(
            egui::RichText::new("No script open")
                .font(theme::font_semi(theme::T_H))
                .color(p.text),
        );
    });
}

fn short_home(q: &Path) -> String {
    let s = q.display().to_string();
    match dirs::home_dir() {
        Some(h) => {
            let h = h.display().to_string();
            if s.starts_with(&h) {
                format!("~{}", &s[h.len()..])
            } else {
                s
            }
        }
        None => s,
    }
}

fn relative_time(t: SystemTime) -> String {
    let Ok(d) = SystemTime::now().duration_since(t) else {
        return "now".to_string();
    };
    ago_secs(d.as_secs())
}

fn ago_instant(t: Instant) -> String {
    ago_secs(t.elapsed().as_secs())
}

fn ago_secs(s: u64) -> String {
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

pub fn today() -> String {
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
    if let Some(b) = doc.blocks.first_mut() {
        b.note = "The writer faces the empty page.".to_string();
    }
    doc
}

/// The way to the project's tracker, dressed the way YouTrack dresses its own
/// buttons: a dark, quiet slab with the product's mark at the left, which
/// takes YouTrack's pink-to-violet glow when you reach for it.
fn youtrack_button(ui: &mut egui::Ui, url: &str) -> bool {
    let p = pal();
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    let pink = Color32::from_rgb(0xFF, 0x31, 0x8C);
    let violet = Color32::from_rgb(0x6B, 0x57, 0xFF);

    if hot > 0.02 {
        // a hovered card lifts off the rail on a faint light of its own
        theme::glow_rect(ui.painter(), rect, theme::R_CTRL, theme::mix(pink, violet, 0.5), 12.0, hot * 0.18);
    }
    let base = if p.dark {
        theme::mix(p.raised, Color32::from_rgb(0x19, 0x19, 0x1C), 0.5)
    } else {
        p.solid_hi
    };
    ui.painter().rect_filled(rect, egui::Rounding::same(theme::R_CTRL), base);
    // the rim takes YouTrack's gradient as you reach for it
    theme::fill_grad_poly(
        ui.painter(),
        &theme::rounded_poly(rect, theme::R_CTRL),
        theme::wash(pink, (26.0 * hot) as u8),
        theme::wash(violet, (26.0 * hot) as u8),
        Vec2::new(1.0, 0.2),
    );
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        egui::Rounding::same(theme::R_CTRL),
        Stroke::new(1.0_f32, theme::mix(theme::wash(p.text_faint, 50), theme::mix(pink, violet, 0.4), hot)),
    );

    let mark = Rect::from_center_size(Pos2::new(rect.left() + 20.0, rect.center().y), Vec2::splat(20.0));
    youtrack_logo(ui.painter(), mark);
    ui.painter().text(
        Pos2::new(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "View on YouTrack",
        theme::font_semi(theme::T_SM),
        theme::mix(p.text_dim, p.text, hot),
    );
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.right() - 16.0, rect.center().y), Vec2::splat(12.0)),
        Icon::External,
        theme::mix(p.text_faint, theme::lighten(pink, 0.3), hot),
    );
    resp.on_hover_text(url.to_string()).clicked()
}

/// YouTrack's product mark, drawn from primitives like every other icon here:
/// the gradient tile in YouTrack's pink, violet and cyan, with the black
/// square carrying the white "YT" and its bar.
pub fn youtrack_logo(painter: &egui::Painter, rect: Rect) {
    let s = rect.width().min(rect.height());
    let r = Rect::from_center_size(rect.center(), Vec2::splat(s));
    let pink = Color32::from_rgb(0xFF, 0x31, 0x8C);
    let violet = Color32::from_rgb(0x6B, 0x57, 0xFF);
    let cyan = Color32::from_rgb(0x07, 0xC3, 0xF2);
    // the tile: pink at the top left, violet across, cyan at the foot
    theme::grad_rect(painter, r, s * 0.18, pink, violet, Vec2::new(1.0, 0.6));
    let corner = Rect::from_min_max(
        Pos2::new(r.left() + s * 0.45, r.top() + s * 0.45),
        r.max,
    );
    theme::fill_grad_poly(
        painter,
        &theme::rounded_poly(corner, s * 0.18),
        theme::wash(cyan, 0),
        theme::wash(cyan, 230),
        Vec2::new(0.7, 1.0),
    );
    // the black square, and the letters on it
    let inner = Rect::from_min_max(
        Pos2::new(r.left() + s * 0.17, r.top() + s * 0.17),
        Pos2::new(r.right() - s * 0.17, r.bottom() - s * 0.17),
    );
    painter.rect_filled(inner, egui::Rounding::same(s * 0.04), Color32::BLACK);
    painter.text(
        Pos2::new(inner.left() + s * 0.08, inner.top() + s * 0.02),
        egui::Align2::LEFT_TOP,
        "YT",
        theme::font_bold(s * 0.33),
        Color32::WHITE,
    );
    let bar = Rect::from_min_size(
        Pos2::new(inner.left() + s * 0.09, inner.bottom() - s * 0.15),
        Vec2::new(s * 0.30, s * 0.055),
    );
    painter.rect_filled(bar, egui::Rounding::ZERO, Color32::WHITE);
}

/// A category in the Settings window's list: nothing until you reach for it,
/// and the one open stands in the accent's quiet fill with the bar the
/// library marks its open script with.
fn settings_tab_row(ui: &mut egui::Ui, label: &str, icon: Icon, on: bool) -> bool {
    let p = pal();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::click());
    let hot = anim::ease(ui.ctx(), resp.id, resp.hovered(), anim::HOVER);
    if on {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(theme::R_SM), p.primary_quiet);
        let bar = Rect::from_center_size(Pos2::new(rect.left() + 4.0, rect.center().y), Vec2::new(3.0, 16.0));
        ui.painter().rect_filled(bar, egui::Rounding::same(1.5), p.primary_light);
    } else if hot > 0.01 {
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(theme::R_SM),
            theme::wash(p.text_faint, (24.0 * hot) as u8),
        );
    }
    let ink = if on { p.text } else { theme::mix(p.text_dim, p.text, hot) };
    icons::draw(
        ui.painter(),
        Rect::from_center_size(Pos2::new(rect.left() + 22.0, rect.center().y), Vec2::splat(14.0)),
        icon,
        if on { p.primary_light } else { ink },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        if on { theme::font_med(theme::T_SM) } else { theme::font(theme::T_SM) },
        ink,
    );
    resp.clicked()
}
