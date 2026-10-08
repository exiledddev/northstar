//! The team library, driven headlessly through a store kept in memory: the
//! script picker, waiting for a script to arrive, read-only scripts, someone
//! else's newer save, stars that are yours alone, viewers. No server and no
//! files: the store is a stand-in that records what the app asked of it.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use eframe::egui::{self, pos2, vec2, Event as Input, Key, Modifiers, RawInput, Rect};
use web_time::SystemTime;

use crate::app::{App, Mode};
use crate::backend::{Access, EditSession, Event, Load, Member, Person, Role, Store, Team};
use crate::model::{Document, Element};
use crate::settings::{AfterExport, Settings};
use crate::storage::{self, Entry, Snapshot};
use crate::theme::ThemeId;

/// What the stand-in store remembers, shared with the test.
#[derive(Default)]
struct Shared {
    docs: Vec<(PathBuf, Document)>,
    /// Loads answer "on its way" and wait for the test to deliver.
    slow: bool,
    /// How each script opens for us.
    access: Vec<(PathBuf, Access)>,
    loads: Vec<PathBuf>,
    saves: Vec<(PathBuf, String)>,
    created: Vec<PathBuf>,
    closed: Vec<PathBuf>,
    edit_requests: Vec<PathBuf>,
    snapshots: Vec<(PathBuf, Option<String>)>,
    stars: Vec<(PathBuf, bool)>,
    starred: Vec<PathBuf>,
    roles: Vec<(String, Role)>,
    events: Vec<Event>,
    signed_out: bool,
}

struct Memory {
    team: Team,
    shared: Rc<RefCell<Shared>>,
}

fn alex() -> Person {
    Person::new("222222222222222222", "Alex Reyes")
}

fn sam() -> Person {
    Person::new("111111111111111111", "Sam Ito")
}

impl Store for Memory {
    fn team(&self) -> Option<&Team> {
        Some(&self.team)
    }
    fn location(&self) -> String {
        "the test team".into()
    }
    fn list(&mut self) -> Vec<Entry> {
        let s = self.shared.borrow();
        s.docs
            .iter()
            .map(|(path, d)| Entry {
                path: path.clone(),
                title: d.meta.title.clone(),
                modified: SystemTime::now(),
                preview: String::new(),
                starred: s.starred.contains(path),
                pages: 1,
                scenes: d.scene_count(),
                edited_by: Some(alex()),
                editing: None,
                author: d.meta.author.clone(),
            })
            .collect()
    }
    fn changed(&mut self) -> bool {
        false
    }
    fn load(&mut self, key: &Path) -> Load {
        let mut s = self.shared.borrow_mut();
        s.loads.push(key.to_path_buf());
        if s.slow {
            return Load::Pending;
        }
        let doc = s.docs.iter().find(|(p, _)| p == key).map(|(_, d)| d.clone());
        let access = s
            .access
            .iter()
            .find(|(p, _)| p == key)
            .map(|(_, a)| a.clone())
            .unwrap_or(Access::Edit);
        match doc {
            Some(d) => Load::Ready(d, access),
            None => Load::Failed("no such script".into()),
        }
    }
    fn close(&mut self, key: &Path) {
        self.shared.borrow_mut().closed.push(key.to_path_buf());
    }
    fn create(&mut self, doc: &Document) -> Result<PathBuf, String> {
        let mut s = self.shared.borrow_mut();
        let path = PathBuf::from(format!("team-{}", s.docs.len() + 1));
        s.docs.push((path.clone(), doc.clone()));
        s.created.push(path.clone());
        Ok(path)
    }
    fn save(&mut self, key: &Path, doc: &Document) -> Result<(), String> {
        let mut s = self.shared.borrow_mut();
        s.saves.push((key.to_path_buf(), storage::to_markdown(doc)));
        if let Some(slot) = s.docs.iter_mut().find(|(p, _)| p == key) {
            slot.1 = doc.clone();
        }
        Ok(())
    }
    fn duplicate(&mut self, _key: &Path) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn delete(&mut self, _key: &Path) -> Result<(), String> {
        Ok(())
    }
    fn set_starred(&mut self, key: &Path, on: bool) -> Result<(), String> {
        let mut s = self.shared.borrow_mut();
        s.stars.push((key.to_path_buf(), on));
        s.starred.retain(|p| p != key);
        if on {
            s.starred.push(key.to_path_buf());
        }
        Ok(())
    }
    fn request_edit(&mut self, key: &Path) {
        self.shared.borrow_mut().edit_requests.push(key.to_path_buf());
    }
    fn take_snapshot(&mut self, key: &Path, _doc: &Document, label: Option<&str>) -> Result<(), String> {
        self.shared
            .borrow_mut()
            .snapshots
            .push((key.to_path_buf(), label.map(|x| x.to_string())));
        Ok(())
    }
    fn snapshots(&mut self, _key: &Path) -> Option<Vec<Snapshot>> {
        None
    }
    fn load_snapshot(&mut self, _snap: &Path) -> Load {
        Load::Pending
    }
    fn history(&mut self, _key: &Path) -> Option<Vec<EditSession>> {
        let now = SystemTime::now();
        Some(vec![
            EditSession { by: alex(), from: now, to: now, words: 312, saves: 9 },
            EditSession { by: sam(), from: now, to: now, words: -40, saves: 2 },
        ])
    }
    fn trash(&mut self) -> Option<Vec<Entry>> {
        Some(Vec::new())
    }
    fn set_role(&mut self, id: &str, role: Role) {
        self.shared.borrow_mut().roles.push((id.to_string(), role));
    }
    fn sign_out(&mut self) {
        self.shared.borrow_mut().signed_out = true;
    }
    fn read_settings(&mut self) -> Settings {
        Settings::parse("splash = no\nanimations = no\nmatch_tesseract = no\nautosave_ms = 150\n")
    }
    fn write_settings(&mut self, _s: &Settings) -> Result<(), String> {
        Ok(())
    }
    fn deliver(&mut self, name: &str, _bytes: Vec<u8>, _after: Option<AfterExport>) -> Result<String, String> {
        Ok(name.to_string())
    }
    fn start_import(&mut self) {}
    fn open_url(&mut self, _url: &str) {}
    fn events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.shared.borrow_mut().events)
    }
}

fn script(title: &str, line: &str) -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = title.into();
    d.push(Element::SceneHeading, "INT. ROOM - DAY");
    d.push(Element::Action, line);
    d.reseed_ids();
    d
}

struct Team1 {
    ctx: egui::Context,
    app: App,
    shared: Rc<RefCell<Shared>>,
}

impl Team1 {
    fn new(role: Role) -> Team1 {
        Team1::with(role, |_| {})
    }

    fn with(role: Role, setup: impl FnOnce(&mut Shared)) -> Team1 {
        let shared = Rc::new(RefCell::new(Shared::default()));
        {
            let mut s = shared.borrow_mut();
            s.docs.push((PathBuf::from("pilot"), script("Pilot", "Phones ring.")));
            s.docs.push((PathBuf::from("finale"), script("Finale", "Snow falls.")));
            setup(&mut s);
        }
        let me = sam();
        let team = Team {
            name: "MarkedExiled Software".into(),
            me: me.clone(),
            role,
            members: vec![
                Member { person: me, role, pending: false, fixed: role == Role::Owner },
                Member { person: alex(), role: Role::Editor, pending: false, fixed: false },
                Member { person: Person::new("333333333333333333", "333333333333333333"), role: Role::Viewer, pending: true, fixed: false },
            ],
        };
        let ctx = egui::Context::default();
        let _ = ctx.run(screen(), |_| {});
        let app = App::with_store(&ctx, Box::new(Memory { team, shared: shared.clone() }));
        let mut t = Team1 { ctx, app, shared };
        t.frames(3);
        t
    }

    fn frame_with(&mut self, events: Vec<Input>) {
        let mut input = screen();
        input.events = events;
        let ctx = self.ctx.clone();
        let app = &mut self.app;
        let _ = ctx.run(input, |c| app.frame(c));
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame_with(Vec::new());
        }
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.frame_with(vec![Input::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.frames(2);
    }

    fn say(&self, e: Event) {
        self.shared.borrow_mut().events.push(e);
    }
}

fn screen() -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 900.0))),
        ..Default::default()
    }
}

#[test]
fn a_team_library_opens_on_the_picker_and_takes_nothing() {
    let t = Team1::new(Role::Editor);
    assert!(t.app.is_home(), "a team opens on Home");
    assert!(t.app.path().is_none(), "no script is open");
    let s = t.shared.borrow();
    assert!(s.loads.is_empty(), "starting up must not open (and so lock) a script");
    assert!(s.created.is_empty(), "starting up must not add a starter script to a team");
    assert!(s.saves.is_empty());
    drop(s);
    assert_eq!(t.app.debug_home_order().len(), 2, "every script is a card");
}

#[test]
fn a_script_from_the_picker_opens_when_it_arrives() {
    let mut t = Team1::with(Role::Editor, |s| s.slow = true);
    t.app.debug_open_from_home(PathBuf::from("finale"));
    t.frames(2);
    assert!(t.app.is_home(), "the picker stays up until the script is here");
    t.say(Event::Opened {
        key: PathBuf::from("finale"),
        doc: script("Finale", "Snow falls."),
        access: Access::Edit,
    });
    t.frames(2);
    assert!(!t.app.is_home());
    assert_eq!(t.app.path(), Some(PathBuf::from("finale")));
    assert_eq!(t.app.doc().meta.title, "Finale");
    assert_eq!(t.app.mode(), Mode::Write);

    // and going home lets go of it
    t.app.debug_go_home();
    t.frames(2);
    assert!(t.app.is_home());
    assert!(t.shared.borrow().closed.contains(&PathBuf::from("finale")));
}

#[test]
fn a_script_someone_else_is_editing_cannot_be_changed() {
    let mut t = Team1::with(Role::Editor, |s| {
        s.access.push((PathBuf::from("pilot"), Access::ReadOnly { by: Some(alex()) }));
    });
    t.app.debug_open_from_home(PathBuf::from("pilot"));
    t.frames(2);
    assert!(!t.app.access().can_edit());
    assert_eq!(t.app.mode(), Mode::Read, "it opens the way it prints");

    // Write is not reachable, by mode switch or by keys
    t.app.set_mode(Mode::Write);
    t.frames(2);
    assert_eq!(t.app.mode(), Mode::Read);
    t.press(Key::G, Modifiers::COMMAND);
    assert_eq!(t.app.mode(), Mode::Read);

    // a change that slips through anyway is put back, and nothing is saved
    let before = t.app.doc().blocks[1].text.clone();
    t.app.doc_mut().blocks[1].text = "Vandalised.".into();
    t.frames(2);
    assert_eq!(t.app.doc().blocks[1].text, before);
    t.app.debug_save();
    t.press(Key::S, Modifiers::COMMAND);
    t.frames(5);
    assert!(t.shared.borrow().saves.is_empty(), "nothing is ever written from a read-only script");

    // when Alex lets go, it can be asked for
    t.say(Event::LockFree { key: PathBuf::from("pilot") });
    t.frames(2);
    assert!(t.app.debug_lock_free());
    t.app.debug_edit_now();
    assert_eq!(t.shared.borrow().edit_requests, vec![PathBuf::from("pilot")]);
    t.say(Event::Opened {
        key: PathBuf::from("pilot"),
        doc: script("Pilot", "Phones ring twice."),
        access: Access::Edit,
    });
    t.frames(2);
    assert!(t.app.access().can_edit());
    assert_eq!(t.app.doc().blocks[1].text, "Phones ring twice.", "it reopens as Alex left it");
}

#[test]
fn losing_the_script_mid_sentence_keeps_what_you_wrote() {
    let mut t = Team1::new(Role::Editor);
    t.app.debug_open_from_home(PathBuf::from("pilot"));
    t.frames(2);
    t.app.doc_mut().blocks[1].text = "Phones ring and ring.".into();
    t.say(Event::Locked { key: PathBuf::from("pilot"), by: alex() });
    t.frames(2);
    assert!(!t.app.access().can_edit());
    assert_eq!(t.app.mode(), Mode::Read);
    let s = t.shared.borrow();
    assert!(
        s.snapshots.iter().any(|(p, l)| p == Path::new("pilot") && l.as_deref() == Some("Your version (kept)")),
        "unsaved words are kept as a snapshot: {:?}",
        s.snapshots
    );
}

#[test]
fn a_newer_save_from_someone_else_replaces_the_page() {
    let mut t = Team1::new(Role::Editor);
    t.app.debug_open_from_home(PathBuf::from("pilot"));
    t.frames(2);
    t.say(Event::Conflict {
        key: PathBuf::from("pilot"),
        theirs: script("Pilot", "Alex's version."),
        by: alex(),
    });
    t.frames(2);
    assert_eq!(t.app.doc().blocks[1].text, "Alex's version.");
    assert!(t.app.access().can_edit(), "you carry on from theirs");
    assert!(!t.app.is_dirty());
}

#[test]
fn a_star_on_a_team_is_yours_and_never_written_into_the_script() {
    let mut t = Team1::new(Role::Editor);
    t.app.debug_open_from_home(PathBuf::from("pilot"));
    t.frames(2);
    t.app.debug_toggle_star(Path::new("pilot"));
    t.frames(3);
    let s = t.shared.borrow();
    assert_eq!(s.stars, vec![(PathBuf::from("pilot"), true)]);
    assert!(!s.saves.iter().any(|(_, md)| md.contains("starred")), "a team star never goes into the shared file");
    drop(s);
    assert!(!t.app.doc().meta.starred);
}

#[test]
fn a_new_team_script_is_signed_with_your_name() {
    let mut t = Team1::new(Role::Editor);
    t.app.debug_new_script();
    t.frames(2);
    assert!(!t.app.is_home());
    assert_eq!(t.app.doc().meta.author, "Sam Ito");
    assert_eq!(t.shared.borrow().created.len(), 1);
}

#[test]
fn a_viewer_reads_and_writes_nothing() {
    let mut t = Team1::with(Role::Viewer, |s| {
        s.access.push((PathBuf::from("pilot"), Access::ReadOnly { by: None }));
    });
    t.press(Key::N, Modifiers::COMMAND);
    t.app.debug_new_script();
    t.frames(2);
    assert!(t.shared.borrow().created.is_empty(), "a viewer cannot start a script");
    t.app.debug_open_from_home(PathBuf::from("pilot"));
    t.frames(2);
    assert_eq!(t.app.mode(), Mode::Read);
    t.press(Key::Z, Modifiers::COMMAND);
    t.press(Key::Num3, Modifiers::COMMAND);
    t.frames(5);
    assert!(t.shared.borrow().saves.is_empty());
}

#[test]
fn every_team_panel_draws_in_every_theme() {
    let mut t = Team1::new(Role::Owner);
    for theme in ThemeId::ALL {
        for light in [false, true] {
            t.app.debug_set_theme(theme, light);
            t.app.debug_go_home();
            t.frames(3);
            t.app.debug_open_settings(7);
            t.frames(3);
            t.app.debug_open_from_home(PathBuf::from("pilot"));
            t.frames(2);
            t.app.settings_mut().show_details = true;
            t.app.debug_open_popover_history();
            t.frames(3);
            t.app.debug_open_popover_trash();
            t.frames(3);
        }
    }
    assert_eq!(t.app.debug_settings_tab(), 7);
}

#[test]
fn the_desktop_has_no_team_and_no_picker() {
    let _g = crate::tests::sandbox("no-team");
    let ctx = egui::Context::default();
    let _ = ctx.run(screen(), |_| {});
    let mut app = App::with_context(&ctx);
    assert!(!app.is_home());
    app.debug_open_settings(7);
    let mut input = screen();
    input.events = vec![];
    let _ = ctx.run(input, |c| app.frame(c));
    // the settings window clamps to the tabs it has: the desktop has seven,
    // and no Team tab to fall into
    let _ = ctx.run(screen(), |c| app.frame(c));
    assert!(app.access().can_edit());
}
