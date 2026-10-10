//! A look at the team edition's screens without a server: Home, a script
//! someone else is editing, History, the Team tab. Everything lives in memory
//! and nothing is written anywhere — run it with
//!
//! ```text
//! cargo run --example team_demo [owner|editor|viewer]
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use northstar::app::App;
use northstar::backend::{Access, EditSession, Event, Load, Member, Person, Role, Store, Team};
use northstar::eframe::{self, egui};
use northstar::model::{Document, Element};
use northstar::settings::{AfterExport, Settings};
use northstar::storage::{Entry, Snapshot};
use web_time::{Duration, SystemTime};

/// A script in the demo: where, what, saved by, being edited by, how long ago.
type DemoScript = (PathBuf, Document, Option<Person>, Option<Person>, u64);

struct Demo {
    team: Team,
    docs: Vec<DemoScript>,
    starred: Vec<PathBuf>,
    settings: Settings,
}

fn person(id: &str, name: &str, hue: f32) -> Person {
    Person {
        id: id.into(),
        name: name.into(),
        avatar: Some(Arc::new(face(hue))),
    }
}

/// A soft two-tone disc standing in for a Discord picture.
fn face(hue: f32) -> egui::ColorImage {
    let n = 64;
    let mut px = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 / n as f32, y as f32 / n as f32);
            let head = ((fx - 0.5).powi(2) + (fy - 0.40).powi(2)).sqrt() < 0.17;
            let body = ((fx - 0.5).powi(2) + (fy - 0.98).powi(2)).sqrt() < 0.40;
            let base = northstar::theme::hsl(hue, 0.45, 0.46 + 0.08 * fy);
            px.push(if head || body {
                northstar::theme::hsl(hue, 0.25, 0.86)
            } else {
                base
            });
        }
    }
    egui::ColorImage { size: [n, n], pixels: px }
}

fn script(title: &str, author: &str, lines: &[(Element, &str)]) -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = title.into();
    d.meta.author = author.into();
    d.meta.draft = "First Draft".into();
    for (e, t) in lines {
        d.push(*e, t);
    }
    d.reseed_ids();
    d
}

impl Store for Demo {
    fn team(&self) -> Option<&Team> {
        Some(&self.team)
    }
    fn location(&self) -> String {
        "a demo kept in memory".into()
    }
    fn list(&mut self) -> Vec<Entry> {
        self.docs
            .iter()
            .map(|(p, d, by, editing, ago)| Entry {
                path: p.clone(),
                title: d.meta.title.clone(),
                modified: SystemTime::now() - Duration::from_secs(*ago),
                preview: d.blocks.get(1).map(|b| b.text.clone()).unwrap_or_default(),
                starred: self.starred.contains(p),
                pages: 1 + d.blocks.len() / 6,
                scenes: d.scene_count(),
                edited_by: by.clone(),
                editing: editing.clone(),
                author: d.meta.author.clone(),
            })
            .collect()
    }
    fn changed(&mut self) -> bool {
        false
    }
    fn load(&mut self, key: &Path) -> Load {
        match self.docs.iter().find(|x| x.0 == key) {
            Some((_, d, _, editing, _)) => Load::Ready(
                d.clone(),
                if self.team.role == Role::Viewer {
                    Access::ReadOnly { by: None }
                } else {
                    match editing {
                        Some(p) => Access::ReadOnly { by: Some(p.clone()) },
                        None => Access::Edit,
                    }
                },
            ),
            None => Load::Failed("not in the demo".into()),
        }
    }
    fn create(&mut self, doc: &Document) -> Result<PathBuf, String> {
        let p = PathBuf::from(format!("demo-{}", self.docs.len()));
        self.docs.push((p.clone(), doc.clone(), Some(self.team.me.clone()), None, 0));
        Ok(p)
    }
    fn save(&mut self, key: &Path, doc: &Document) -> Result<(), String> {
        if let Some(x) = self.docs.iter_mut().find(|x| x.0 == key) {
            x.1 = doc.clone();
            x.2 = Some(self.team.me.clone());
            x.4 = 0;
        }
        Ok(())
    }
    fn duplicate(&mut self, _key: &Path) -> Result<Option<String>, String> {
        Err("not in the demo".into())
    }
    fn delete(&mut self, _key: &Path) -> Result<(), String> {
        Err("not in the demo".into())
    }
    fn set_starred(&mut self, key: &Path, on: bool) -> Result<(), String> {
        self.starred.retain(|p| p != key);
        if on {
            self.starred.push(key.to_path_buf());
        }
        Ok(())
    }
    fn take_snapshot(&mut self, _: &Path, _: &Document, _: Option<&str>) -> Result<(), String> {
        Ok(())
    }
    fn snapshots(&mut self, _key: &Path) -> Option<Vec<Snapshot>> {
        Some(Vec::new())
    }
    fn load_snapshot(&mut self, _snap: &Path) -> Load {
        Load::Failed("not in the demo".into())
    }
    fn history(&mut self, _key: &Path) -> Option<Vec<EditSession>> {
        let now = SystemTime::now();
        let ago = |m: u64| now - Duration::from_secs(m * 60);
        Some(
            self.team
                .members
                .iter()
                .filter(|m| !m.pending)
                .enumerate()
                .map(|(k, m)| EditSession {
                    by: m.person.clone(),
                    from: ago(40 + 300 * k as u64),
                    to: ago(5 + 300 * k as u64),
                    words: 412 - 180 * k as i64,
                    saves: 12,
                })
                .collect(),
        )
    }
    fn trash(&mut self) -> Option<Vec<Entry>> {
        Some(Vec::new())
    }
    fn read_settings(&mut self) -> Settings {
        self.settings.clone()
    }
    fn write_settings(&mut self, s: &Settings) -> Result<(), String> {
        self.settings = s.clone();
        Ok(())
    }
    fn deliver(&mut self, name: &str, _: Vec<u8>, _: Option<AfterExport>) -> Result<String, String> {
        Ok(format!("{name} (not written: this is the demo)"))
    }
    fn start_import(&mut self) {}
    fn open_url(&mut self, _url: &str) {}
    fn events(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

fn main() -> eframe::Result<()> {
    let role = match std::env::args().nth(1).as_deref() {
        Some("viewer") => Role::Viewer,
        Some("owner") => Role::Owner,
        _ => Role::Editor,
    };
    let sam = person("111111111111111111", "Sam Ito", 0.58);
    let alex = person("222222222222222222", "Alex Reyes", 0.95);
    let june = person("333333333333333333", "June Park", 0.32);
    let team = Team {
        name: "MarkedExiled Software".into(),
        me: sam.clone(),
        role,
        members: vec![
            Member { person: sam.clone(), role, pending: false, fixed: role == Role::Owner },
            Member { person: alex.clone(), role: Role::Editor, pending: false, fixed: false },
            Member { person: june.clone(), role: Role::Viewer, pending: false, fixed: false },
            Member {
                person: Person::new("444444444444444444", "444444444444444444"),
                role: Role::Editor,
                pending: true,
                fixed: false,
            },
        ],
    };
    use Element::*;
    let docs = vec![
        (
            PathBuf::from("long-way-down"),
            script("The Long Way Down", "A. Writer", &[
                (Act, "COLD OPEN"),
                (SceneHeading, "INT. WAREHOUSE - NIGHT"),
                (Action, "Rain hammers the corrugated roof. MARIA moves between the crates."),
                (Character, "MARIA"),
                (Dialogue, "Three, four... there you are."),
                (Act, "ACT ONE"),
                (SceneHeading, "EXT. HARBOUR - DAWN"),
                (Action, "Gulls over black water. COLE waits by the bollard, collar up."),
                (Character, "COLE"),
                (Dialogue, "You said six."),
                (Character, "MARIA"),
                (Parenthetical, "(not stopping)"),
                (Dialogue, "I said six-ish."),
                (SceneHeading, "INT. CAR - MOVING - DAY"),
                (Action, "The wipers lose the argument."),
                (Act, "ACT TWO"),
                (SceneHeading, "INT. DINER - NIGHT"),
                (Action, "A booth by the window. Two coffees nobody drinks."),
            ]),
            Some(sam.clone()),
            None,
            60 * 50,
        ),
        (
            PathBuf::from("pilot"),
            script("Northbound — Pilot", "Alex Reyes", &[
                (SceneHeading, "EXT. PLATFORM - DAWN"),
                (Action, "The last train north idles in the cold."),
            ]),
            Some(alex.clone()),
            Some(alex.clone()),
            60 * 5,
        ),
        (
            PathBuf::from("short"),
            script("Paper Moons", "June Park", &[
                (SceneHeading, "INT. KITCHEN - DAY"),
                (Action, "Flour everywhere."),
            ]),
            Some(june.clone()),
            None,
            3600 * 26,
        ),
        (
            PathBuf::from("spec"),
            script("Untitled Heist Spec", "", &[(SceneHeading, "INT. VAULT - NIGHT")]),
            Some(alex.clone()),
            None,
            3600 * 24 * 6,
        ),
    ];
    let demo = Demo {
        team,
        docs,
        starred: vec![PathBuf::from("long-way-down")],
        // NS_DEMO_SETTINGS adds lines to the settings, e.g. "theme = zen"
        settings: Settings::parse(&format!(
            "splash = no\n{}\n",
            std::env::var("NS_DEMO_SETTINGS").unwrap_or_default().replace(';', "\n")
        )),
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Northstar — team demo")
            .with_inner_size([1320.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Northstar team demo",
        options,
        Box::new(move |cc| Ok(Box::new(App::with_store(&cc.egui_ctx, Box::new(demo))))),
    )
}
