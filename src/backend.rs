//! Where the library lives.
//!
//! The app never touches a file or a server itself. It talks to a [`Store`]:
//! on the desktop that is [`LocalBackend`], the user's own library folder,
//! exactly as Northstar has always kept it; in the browser it is the team
//! library on the server, behind the same trait (in the web crate).
//!
//! Every call answers at once. A store that has to go and ask somebody — the
//! server — answers from what it already knows and says the rest later, as
//! [`Event`]s the app collects every frame. The desktop store knows
//! everything at once, so for it nearly every answer is immediate.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui;
use web_time::SystemTime;

use crate::alerts::Tone;
use crate::model::Document;
use crate::settings::{AfterExport, Settings};
use crate::storage::{Entry, Snapshot};

/// Somebody on the team: their Discord id, the name they go by, their picture.
#[derive(Clone)]
pub struct Person {
    pub id: String,
    pub name: String,
    /// Their Discord avatar, already decoded. `None` draws their initials.
    pub avatar: Option<Arc<egui::ColorImage>>,
}

impl Person {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Person {
        Person {
            id: id.into(),
            name: name.into(),
            avatar: None,
        }
    }

    /// One or two letters for when there is no picture.
    pub fn initials(&self) -> String {
        let mut out = String::new();
        for word in self.name.split_whitespace().take(2) {
            if let Some(c) = word.chars().next() {
                out.extend(c.to_uppercase());
            }
        }
        if out.is_empty() {
            out.push('?');
        }
        out
    }

    /// Their first name, for "Alex is editing".
    pub fn first_name(&self) -> &str {
        self.name.split_whitespace().next().unwrap_or(&self.name)
    }
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Person({} {:?})", self.id, self.name)
    }
}

/// What someone may do on the team.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Everything, including who is on the team.
    Owner,
    /// Write, snapshot, restore, delete.
    Editor,
    /// Read and export.
    Viewer,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Owner, Role::Editor, Role::Viewer];
    pub fn label(self) -> &'static str {
        match self {
            Role::Owner => "Owner",
            Role::Editor => "Editor",
            Role::Viewer => "Viewer",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Editor => "editor",
            Role::Viewer => "viewer",
        }
    }
    pub fn from_slug(s: &str) -> Option<Role> {
        match s {
            "owner" => Some(Role::Owner),
            "editor" => Some(Role::Editor),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
    pub fn can_write(self) -> bool {
        self != Role::Viewer
    }
}

/// Someone on the team's list.
#[derive(Clone, Debug)]
pub struct Member {
    pub person: Person,
    pub role: Role,
    /// Added to the list but never signed in: only their Discord id is known.
    pub pending: bool,
    /// Listed as an owner in the server's configuration, so the app cannot
    /// change them.
    pub fixed: bool,
}

/// The team a library belongs to. Only team libraries have one.
#[derive(Clone, Debug)]
pub struct Team {
    pub name: String,
    pub me: Person,
    pub role: Role,
    pub members: Vec<Member>,
}

impl Team {
    pub fn person(&self, id: &str) -> Option<&Person> {
        self.members
            .iter()
            .map(|m| &m.person)
            .chain(std::iter::once(&self.me))
            .find(|p| p.id == id)
    }
}

/// One stretch of one person's work on a script: what History lists.
#[derive(Clone, Debug)]
pub struct EditSession {
    pub by: Person,
    pub from: SystemTime,
    pub to: SystemTime,
    /// Words added (or, negative, taken out) over the stretch.
    pub words: i64,
    pub saves: u32,
}

/// Whether the script just opened may be written to.
#[derive(Clone, Debug, PartialEq)]
pub enum Access {
    Edit,
    /// Someone else is editing it (`by`), or you are a viewer (`None`).
    ReadOnly { by: Option<Person> },
}

impl Access {
    pub fn can_edit(&self) -> bool {
        matches!(self, Access::Edit)
    }
}

/// What asking for a script gets you.
#[derive(Debug)]
pub enum Load {
    Ready(Document, Access),
    /// On its way: an [`Event::Opened`] (or [`Event::SnapshotDoc`]) follows.
    Pending,
    Failed(String),
}

/// Something a store has to say after the fact.
#[derive(Debug)]
pub enum Event {
    /// A script asked for with [`Store::load`] has arrived.
    Opened {
        key: PathBuf,
        doc: Document,
        access: Access,
    },
    /// The library changed: list it again.
    LibraryChanged,
    /// Somebody else saved this script after you opened it. Your version was
    /// kept as a snapshot before this was said; `theirs` is the script now.
    Conflict {
        key: PathBuf,
        theirs: Document,
        by: Person,
    },
    /// You can no longer edit this script: `by` is editing it.
    Locked { key: PathBuf, by: Person },
    /// The person editing this script has finished; it can be edited now.
    LockFree { key: PathBuf },
    /// A script you were reading was saved by its editor: here it is now.
    Refreshed { key: PathBuf, doc: Document },
    /// A snapshot asked for with [`Store::load_snapshot`].
    SnapshotDoc { snap: PathBuf, doc: Document },
    /// A file chosen in the import picker, by path (desktop) ...
    ImportPath(PathBuf),
    /// ... or by name and contents (browser).
    Imported { name: String, bytes: Vec<u8> },
    /// Something to tell the user.
    Notice {
        title: String,
        detail: String,
        tone: Tone,
    },
    /// The session ended: back to signing in.
    SignedOut,
}

impl Event {
    pub fn notice(title: &str, detail: &str, tone: Tone) -> Event {
        Event::Notice {
            title: title.to_string(),
            detail: detail.to_string(),
            tone,
        }
    }
}

/// A library, wherever it is.
pub trait Store {
    // ---- what this library is ----

    /// The team, for a team library; `None` for your own folder. Everything
    /// the app shows about people hangs off this.
    fn team(&self) -> Option<&Team> {
        None
    }
    /// Where the library is, in words, for Settings.
    fn location(&self) -> String;

    // ---- scripts ----

    fn list(&mut self) -> Vec<Entry>;
    /// Has the library changed since it was last listed? Cheap; asked often.
    fn changed(&mut self) -> bool;
    fn load(&mut self, key: &Path) -> Load;
    /// The app has moved on from this script.
    fn close(&mut self, _key: &Path) {}
    /// A new script in the library. Returns where it lives.
    fn create(&mut self, doc: &Document) -> Result<PathBuf, String>;
    fn save(&mut self, key: &Path, doc: &Document) -> Result<(), String>;
    /// Where the script should live now it is called `title`: the desktop
    /// renames its file to match (and its snapshots follow); a team library's
    /// ids never change.
    fn follow_title(&mut self, key: &Path, _title: &str) -> PathBuf {
        key.to_path_buf()
    }
    /// A copy, called "… (copy)". `Some(title)` if it is done; `None` if the
    /// store will say so when it is.
    fn duplicate(&mut self, key: &Path) -> Result<Option<String>, String>;
    fn delete(&mut self, key: &Path) -> Result<(), String>;
    fn set_starred(&mut self, key: &Path, on: bool) -> Result<(), String>;
    /// Ask to edit a script that someone else was editing.
    fn request_edit(&mut self, _key: &Path) {}

    // ---- snapshots and history ----

    fn take_snapshot(&mut self, key: &Path, doc: &Document, label: Option<&str>) -> Result<(), String>;
    /// `None` while they are on their way.
    fn snapshots(&mut self, key: &Path) -> Option<Vec<Snapshot>>;
    fn load_snapshot(&mut self, snap: &Path) -> Load;
    /// Who worked on it and when. Team libraries only.
    fn history(&mut self, _key: &Path) -> Option<Vec<EditSession>> {
        None
    }
    /// Recently deleted scripts. Team libraries only.
    fn trash(&mut self) -> Option<Vec<Entry>> {
        None
    }
    fn restore(&mut self, _key: &Path) {}

    // ---- the team ----

    fn add_member(&mut self, _discord_id: &str, _role: Role) {}
    fn set_role(&mut self, _discord_id: &str, _role: Role) {}
    fn remove_member(&mut self, _discord_id: &str) {}
    fn sign_out(&mut self) {}

    // ---- settings ----

    fn read_settings(&mut self) -> Settings;
    fn write_settings(&mut self, s: &Settings) -> Result<(), String>;
    /// Tesseract's look and when it last changed, where Tesseract is installed.
    fn tesseract_look(&mut self) -> Option<(Settings, SystemTime)> {
        None
    }
    /// Has Tesseract been run here, so there is a look to follow?
    fn has_tesseract(&self) -> bool {
        false
    }

    // ---- the world outside ----

    /// Hand an export over: into the exports folder, or to the browser as a
    /// download. `after` is what Quick Export does next; `None` just keeps
    /// it. Returns the name it went by.
    fn deliver(&mut self, name: &str, bytes: Vec<u8>, after: Option<AfterExport>) -> Result<String, String>;
    /// Open the import picker. What is picked comes back as an event.
    fn start_import(&mut self);
    fn open_url(&mut self, url: &str);
    /// Show a script (or, with `None`, the library) in the file manager.
    fn reveal(&mut self, _key: Option<&Path>) {}
    /// Open the exports folder.
    fn open_exports(&mut self) {}

    /// What has happened since the last time the app asked.
    fn events(&mut self) -> Vec<Event>;
    /// Is something on its way, so the app should look again soon?
    fn busy(&self) -> bool {
        false
    }
}

// ---------- the desktop ----------

#[cfg(not(target_arch = "wasm32"))]
pub use local::LocalBackend;

#[cfg(not(target_arch = "wasm32"))]
mod local {
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;

    use web_time::{Duration, Instant, SystemTime};

    use super::{Access, Event, Load, Store};
    use crate::alerts::Tone;
    use crate::model::Document;
    use crate::settings::{AfterExport, Settings};
    use crate::storage::{self, Entry, Snapshot};

    /// The library folder under `~/.local/share/northstar`, kept exactly as
    /// Northstar always has: these are the same `storage` calls the app made
    /// itself before there was a trait.
    #[derive(Default)]
    pub struct LocalBackend {
        /// The scripts folder's own modification time, so a script dropped in
        /// from outside shows up in the library without waiting for a save.
        library_seen: Option<(SystemTime, Instant)>,
        importing: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
        events: Vec<Event>,
    }

    impl LocalBackend {
        pub fn new() -> LocalBackend {
            let _ = storage::ensure_dirs();
            LocalBackend::default()
        }
    }

    impl Store for LocalBackend {
        fn location(&self) -> String {
            storage::scripts_dir().display().to_string()
        }

        fn list(&mut self) -> Vec<Entry> {
            let entries = storage::list_scripts();
            if let Ok(m) = std::fs::metadata(storage::scripts_dir()).and_then(|m| m.modified()) {
                self.library_seen = Some((m, Instant::now()));
            }
            entries
        }

        /// Look at the scripts folder now and then; if anything was added,
        /// removed or renamed there from outside, it has changed.
        fn changed(&mut self) -> bool {
            if let Some((_, at)) = self.library_seen {
                if at.elapsed() < Duration::from_millis(1500) {
                    return false;
                }
            }
            let now = std::fs::metadata(storage::scripts_dir()).and_then(|m| m.modified()).ok();
            match (now, self.library_seen) {
                (Some(m), Some((seen, _))) if m == seen => {
                    self.library_seen = Some((m, Instant::now()));
                    false
                }
                _ => true,
            }
        }

        fn load(&mut self, key: &Path) -> Load {
            match storage::load(key) {
                Ok(doc) => Load::Ready(doc, Access::Edit),
                Err(e) => Load::Failed(e.to_string()),
            }
        }

        fn create(&mut self, doc: &Document) -> Result<PathBuf, String> {
            let path = storage::unique_path(&doc.meta.title, None);
            storage::save(&path, doc).map_err(|e| e.to_string())?;
            Ok(path)
        }

        fn save(&mut self, key: &Path, doc: &Document) -> Result<(), String> {
            storage::save(key, doc).map_err(|e| e.to_string())
        }

        fn follow_title(&mut self, key: &Path, title: &str) -> PathBuf {
            let mut path = key.to_path_buf();
            let wanted = storage::slugify(title);
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
                let new_path = storage::unique_path(title, Some(&path));
                if path.exists() {
                    if std::fs::rename(&path, &new_path).is_ok() {
                        storage::follow_rename(&path, &new_path);
                        path = new_path;
                    }
                } else {
                    path = new_path;
                }
            }
            path
        }

        fn duplicate(&mut self, key: &Path) -> Result<Option<String>, String> {
            let mut doc = storage::load(key).map_err(|e| e.to_string())?;
            doc.meta.title = format!("{} (copy)", doc.meta.title);
            doc.meta.starred = false;
            let new_path = storage::unique_path(&doc.meta.title, None);
            storage::save(&new_path, &doc).map_err(|e| e.to_string())?;
            Ok(Some(doc.meta.title))
        }

        fn delete(&mut self, key: &Path) -> Result<(), String> {
            storage::delete(key).map_err(|e| e.to_string())
        }

        fn set_starred(&mut self, key: &Path, on: bool) -> Result<(), String> {
            let mut doc = storage::load(key).map_err(|e| e.to_string())?;
            doc.meta.starred = on;
            storage::save(key, &doc).map_err(|e| e.to_string())
        }

        fn take_snapshot(&mut self, key: &Path, doc: &Document, _label: Option<&str>) -> Result<(), String> {
            storage::take_snapshot(key, doc).map(|_| ()).map_err(|e| e.to_string())
        }

        fn snapshots(&mut self, key: &Path) -> Option<Vec<Snapshot>> {
            Some(storage::list_snapshots(key))
        }

        fn load_snapshot(&mut self, snap: &Path) -> Load {
            self.load(snap)
        }

        fn read_settings(&mut self) -> Settings {
            storage::read_settings()
        }

        fn write_settings(&mut self, s: &Settings) -> Result<(), String> {
            storage::write_settings(s).map_err(|e| e.to_string())
        }

        fn tesseract_look(&mut self) -> Option<(Settings, SystemTime)> {
            storage::read_tesseract_settings()
        }

        fn has_tesseract(&self) -> bool {
            storage::tesseract_settings_path().exists()
        }

        fn deliver(&mut self, name: &str, bytes: Vec<u8>, after: Option<AfterExport>) -> Result<String, String> {
            storage::ensure_dirs().map_err(|e| e.to_string())?;
            let path = storage::exports_dir().join(name);
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            match after {
                Some(AfterExport::Open) => storage::open_with_desktop(&path),
                Some(AfterExport::Reveal) => storage::reveal_in_file_manager(&path),
                Some(AfterExport::Nothing) | None => {}
            }
            Ok(name.to_string())
        }

        fn start_import(&mut self) {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(storage::pick_file_to_import());
            });
            self.importing = Some(rx);
        }

        fn open_url(&mut self, url: &str) {
            let _ = std::process::Command::new("xdg-open").arg(url).spawn();
        }

        fn reveal(&mut self, key: Option<&Path>) {
            match key {
                Some(p) => storage::reveal_in_file_manager(p),
                None => storage::open_with_desktop(&storage::scripts_dir()),
            }
        }

        fn open_exports(&mut self) {
            storage::open_with_desktop(&storage::exports_dir());
        }

        fn events(&mut self) -> Vec<Event> {
            if let Some(rx) = &self.importing {
                match rx.try_recv() {
                    Ok(Ok(Some(path))) => {
                        self.importing = None;
                        self.events.push(Event::ImportPath(path));
                    }
                    Ok(Ok(None)) => self.importing = None,
                    Ok(Err(e)) => {
                        self.importing = None;
                        self.events.push(Event::notice("No file picker", &e, Tone::Warn));
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => self.importing = None,
                }
            }
            std::mem::take(&mut self.events)
        }

        /// The import picker is open: look for its answer often.
        fn busy(&self) -> bool {
            self.importing.is_some()
        }
    }
}
