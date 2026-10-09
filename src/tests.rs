//! Behavioural tests: run with `cargo test`.

use crate::export;
use crate::model::{guess_element, wrap, Document, Element};
use crate::storage;

fn sample() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "The Long Way Down".into();
    d.meta.author = "A. Writer".into();
    d.meta.draft = "First Draft".into();
    d.push(Element::SceneHeading, "INT. WAREHOUSE - NIGHT");
    d.push(Element::Action, "Rain hammers the corrugated roof. MARIA moves between the crates, one hand on the wall, counting doors under her breath until she reaches the one that isn't locked.");
    d.push(Element::Character, "MARIA (V.O.)");
    d.push(Element::Parenthetical, "(barely audible)");
    d.push(Element::Dialogue, "Three, four... there you are.");
    d.push(Element::Shot, "ANGLE ON THE DOOR");
    d.push(Element::Transition, "SMASH CUT TO:");
    d.reseed_ids();
    d
}

#[test]
fn markdown_round_trips() {
    let mut a = sample();
    a.normalize();
    let md = storage::to_markdown(&a);
    println!("--- markdown ---\n{md}");
    let b = storage::from_markdown(&md);
    assert_eq!(a.blocks.len(), b.blocks.len(), "block count");
    for (x, y) in a.blocks.iter().zip(b.blocks.iter()) {
        assert_eq!(x.element, y.element, "element for {:?}", x.text);
        assert_eq!(x.text, y.text, "text");
    }
    assert_eq!(a.meta.title, b.meta.title);
    assert_eq!(a.meta.author, b.meta.author);
    assert_eq!(a.meta.draft, b.meta.draft);
}

#[test]
fn wrapping_respects_columns() {
    let long = "Rain hammers the corrugated roof and nobody in this building has slept for two days straight.";
    for l in wrap(long, 35) {
        assert!(l.chars().count() <= 35, "line too wide: {l:?}");
    }
}

#[test]
fn layout_matches_industry_indents() {
    let d = sample();
    let lines = export::compose(&d);
    let find = |e: Element| lines.iter().find(|l| l.element == Some(e)).unwrap().indent;
    assert_eq!(find(Element::SceneHeading), 0);
    assert_eq!(find(Element::Action), 0);
    assert_eq!(find(Element::Character), 22);
    assert_eq!(find(Element::Parenthetical), 16);
    assert_eq!(find(Element::Dialogue), 10);
    assert_eq!(find(Element::Transition), 45);
    let txt = export::to_plain_text(&d);
    println!("--- plain text ---\n{txt}");
    assert!(txt.contains("                      MARIA (V.O.)"));
}

#[test]
fn pagination_is_sane() {
    let mut d = sample();
    for _ in 0..300 {
        d.push(Element::Action, "They run.");
    }
    d.reseed_ids();
    let pages = export::paginate(&export::compose(&d));
    assert!(pages.len() > 5, "expected multiple pages, got {}", pages.len());
    for p in &pages {
        assert!(p.len() <= 60, "page overflow: {}", p.len());
    }
}

#[test]
fn pdf_is_written() {
    let d = sample();
    let path = std::env::temp_dir().join(format!("ns-test-{}.pdf", std::process::id()));
    export::to_pdf(&d, &path).expect("pdf export");
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"%PDF"), "not a pdf");
    assert!(bytes.len() > 1000, "pdf too small: {}", bytes.len());
    println!("pdf bytes: {}", bytes.len());
}

#[test]
fn paste_guessing() {
    assert_eq!(guess_element("INT. CAR - DAY", None), Element::SceneHeading);
    assert_eq!(guess_element("(beat)", None), Element::Parenthetical);
    assert_eq!(guess_element("CUT TO:", None), Element::Transition);
    assert_eq!(guess_element("MARIA", None), Element::Character);
    assert_eq!(guess_element("Not now.", Some(Element::Character)), Element::Dialogue);
    assert_eq!(guess_element("She runs for the door.", Some(Element::Action)), Element::Action);
}

#[test]
fn slugs_are_filenames() {
    assert_eq!(storage::slugify("The Long Way Down"), "the-long-way-down");
    assert_eq!(storage::slugify("  Episode 2: Fallout!  "), "episode-2-fallout");
    assert_eq!(storage::slugify("///"), "untitled");
}

// ------------------------------------------------------------------ new --

use crate::fountain;
use crate::model::{base_character, complete, eighths_label};
use crate::settings::{AfterExport, BlurMode, Settings, SortBy};
use crate::theme::ThemeId;

/// The data home comes from the environment, which is process-wide, so tests
/// that touch disk take turns and each gets a fresh directory. The lock is
/// shared with the headless UI tests, which set the same variable.
pub(crate) static VAULT: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn sandbox(name: &str) -> std::sync::MutexGuard<'static, ()> {
    let guard = VAULT.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("northstar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    std::env::set_var("XDG_DATA_HOME", &dir);
    guard
}

fn three_scenes() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Three".into();
    d.push(Element::Action, "FADE IN on nothing in particular.");
    d.push(Element::SceneHeading, "INT. ONE - DAY");
    d.push(Element::Character, "MARIA (V.O.)");
    d.push(Element::Dialogue, "One.");
    d.push(Element::SceneHeading, "EXT. TWO - NIGHT");
    d.push(Element::Character, "COLE");
    d.push(Element::Parenthetical, "(beat)");
    d.push(Element::Dialogue, "Two, and then some.");
    d.push(Element::Character, "MARIA (CONT'D)");
    d.push(Element::Dialogue, "Two again.");
    d.push(Element::SceneHeading, "INT. THREE - DAWN");
    d.push(Element::Action, "Three.");
    d.reseed_ids();
    d
}

#[test]
fn a_scene_card_round_trips_through_the_file() {
    let mut a = three_scenes();
    a.blocks[1].note = "Maria speaks first.".into();
    a.blocks[1].tint = Some(3);
    a.blocks[4].note = "Cole answers --> twice".into();
    a.meta.starred = true;
    a.normalize();
    let md = storage::to_markdown(&a);
    assert!(md.contains("<!-- scene: tint=3 | Maria speaks first. -->"), "{md}");
    assert!(md.contains("starred: yes"));
    let b = storage::from_markdown(&md);
    assert_eq!(b.blocks.len(), a.blocks.len());
    assert_eq!(b.blocks[1].note, "Maria speaks first.");
    assert_eq!(b.blocks[1].tint, Some(3));
    assert_eq!(b.blocks[4].note, "Cole answers - -> twice", "a comment must not be able to close itself");
    assert_eq!(b.blocks[4].tint, None);
    assert!(b.meta.starred);
    // and nothing else turned into action on the way
    assert!(b.blocks.iter().all(|x| !x.text.contains("<!--")));
}

#[test]
fn files_from_the_first_northstar_still_open_unchanged() {
    let old = "---\ntitle: Old\nauthor: Me\ncontact: \ndraft: One\n---\n\n## INT. ROOM - DAY\n\nA room.\n\n**BOB**\n\n> Hi.\n\n`CUT TO:`\n\n";
    let d = storage::from_markdown(old);
    assert_eq!(d.blocks.len(), 5);
    assert!(!d.meta.starred);
    // an unstarred script with no cards writes exactly the old format
    assert_eq!(storage::to_markdown(&d), old);
    // a comment that is not a card stays as the action it always was
    let d = storage::from_markdown("<!-- a note to self -->\n\n## INT. X - DAY\n");
    assert_eq!(d.blocks[0].element, Element::Action);
    assert_eq!(d.blocks[1].element, Element::SceneHeading);
}

/// Scripts as they sit on a writer's disk today: two written by the first
/// Northstar, one by this one with a card and a star. Every byte matters —
/// these are somebody's work.
const A_REAL_LIBRARY: [(&str, &str); 3] = [
    (
        "the-long-way-down.md",
        "---\ntitle: The Long Way Down\nauthor: A. Writer\ncontact: a.writer@example.com · 555-0100\ndraft: Second Draft — 3 March\n---\n\n## INT. WAREHOUSE - NIGHT\n\nRain hammers the corrugated roof. MARIA moves between the crates, counting doors under her breath — “four, five” — until one isn’t locked.\n\n**MARIA (V.O.)**\n\n*(barely audible)*\n\n> Three, four... there you are.\n\n### ANGLE ON THE DOOR\n\nIt swings inward on its own. Café light spills across the floor.\n\n`SMASH CUT TO:`\n\n## EXT. ROOFTOP - CONTINUOUS\n\n**COLE**\n\n> You came back.\n\n**MARIA (CONT'D)**\n\n> I never left.\n\n`FADE OUT.`\n\n",
    ),
    (
        "untitled-script.md",
        "---\ntitle: Untitled Script\nauthor: \ncontact: \ndraft: \n---\n\n## INT. KITCHEN - DAY\n\nNothing yet.\n\n",
    ),
    (
        "pilot.md",
        "---\ntitle: Pilot\nauthor: Sam\ncontact: \ndraft: First Draft\nstarred: yes\n---\n\n## INT. OFFICE - MORNING\n<!-- scene: tint=2 | Sam meets the team. -->\n\nPhones ring.\n\n**SAM**\n\n> Morning.\n\n",
    ),
];

#[test]
fn an_existing_library_is_never_rewritten_by_the_app() {
    let _g = sandbox("golden");
    storage::ensure_dirs().unwrap();
    let dir = storage::scripts_dir();
    for (name, text) in A_REAL_LIBRARY {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let listing = || {
        let mut names: Vec<String> = std::fs::read_dir(storage::scripts_dir())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let before = listing();

    // starting the app over the library writes nothing
    let ctx = eframe::egui::Context::default();
    let mut app = crate::app::App::with_context(&ctx);
    assert_eq!(listing(), before, "starting up added or removed a file");
    for (name, text) in A_REAL_LIBRARY {
        assert_eq!(std::fs::read_to_string(dir.join(name)).unwrap(), text, "{name} changed on start-up");
    }

    // opening each one and saving it the way closing the window does
    // (rename allowed) gives back exactly the same file, under the same name
    for (name, text) in A_REAL_LIBRARY {
        let path = dir.join(name);
        app.debug_open(path.clone());
        assert_eq!(app.path().as_deref(), Some(path.as_path()));
        app.debug_save();
        assert_eq!(app.path().as_deref(), Some(path.as_path()), "{name} was renamed");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "{name} changed on save");
    }
    assert_eq!(listing(), before, "saving added or removed a file");
}

#[test]
fn scenes_and_cast_are_counted_the_way_a_schedule_would() {
    let d = three_scenes();
    let sc = d.scenes();
    assert_eq!(sc.len(), 3);
    assert_eq!(sc[0].number, 1);
    assert_eq!(sc[0].start, 1, "the action before the first heading is not a scene");
    assert_eq!(sc[1].cast, vec!["COLE".to_string(), "MARIA".to_string()]);
    let cast = d.cast();
    assert_eq!(cast[0].name, "MARIA", "extensions are not separate people");
    assert_eq!(cast[0].cues, 2);
    assert_eq!(cast[0].scenes, 2);
    assert_eq!(cast[1].name, "COLE");
    assert_eq!(cast[1].words, 4);
    assert_eq!(base_character("  maria (V.O.) ^"), "MARIA");
    assert_eq!(d.scene_of(d.blocks[7].id), Some(1));
    assert_eq!(d.scene_of(d.blocks[0].id), None);
}

#[test]
fn a_scene_moves_whole() {
    let mut d = three_scenes();
    assert!(d.move_scene(0, 3), "first scene to the end");
    let order: Vec<String> = d.scenes().iter().map(|s| s.heading.clone()).collect();
    assert_eq!(order, vec!["EXT. TWO - NIGHT", "INT. THREE - DAWN", "INT. ONE - DAY"]);
    assert_eq!(d.blocks.last().unwrap().text, "One.", "its dialogue came with it");
    assert!(d.move_scene(2, 0), "and back to the front");
    assert_eq!(d.scenes()[0].heading, "INT. ONE - DAY");
    assert_eq!(d.blocks[0].element, Element::Action, "the prologue stays put");
    assert!(!d.move_scene(1, 1) && !d.move_scene(1, 2), "a move onto itself is no move");
    assert!(d.remove_scene(1));
    assert_eq!(d.scenes().len(), 2);
    assert_eq!(d.cast().iter().find(|c| c.name == "COLE"), None);
}

#[test]
fn page_breaks_and_scene_lengths_come_from_the_printed_layout() {
    let mut d = sample();
    for i in 0..40 {
        d.push(Element::SceneHeading, &format!("INT. ROOM {i} - DAY"));
        for _ in 0..3 {
            d.push(Element::Action, "They run, and they keep running until the corridor ends in a door.");
        }
    }
    d.reseed_ids();
    let pages = export::page_count(&d);
    let starts = export::page_starts(&d);
    assert_eq!(starts.len(), pages - 1, "every page after the first starts somewhere");
    assert_eq!(starts[0].0, 2);
    let lens = export::scene_lengths(&d);
    assert_eq!(lens.len(), d.scenes().len());
    assert!(lens.iter().all(|(_, pg, e)| *pg >= 1 && *e >= 1));
    let total: usize = lens.iter().map(|x| x.2).sum();
    // the eighths add up to roughly the page count
    assert!((total as i64 - (pages * 8) as i64).abs() <= (pages * 8) as i64 / 3, "{total} eighths for {pages} pages");
    assert_eq!(eighths_label(11), "1 3/8");
    assert_eq!(eighths_label(3), "3/8");
    assert_eq!(eighths_label(16), "2");
}

#[test]
fn smart_type_finishes_names_places_and_transitions() {
    let mut d = three_scenes();
    let own = d.new_block(Element::Character, "");
    let own_id = own.id;
    d.blocks.push(own);
    assert_eq!(complete(&d, own_id, Element::Character, "MA").as_deref(), Some("RIA"));
    assert_eq!(complete(&d, own_id, Element::Character, "co").as_deref(), Some("LE"));
    assert_eq!(complete(&d, own_id, Element::Character, "MARIA"), None, "nothing left to add");
    assert_eq!(complete(&d, own_id, Element::Character, "ZED"), None);
    assert_eq!(complete(&d, own_id, Element::SceneHeading, "e").as_deref(), Some("XT. "));
    assert_eq!(
        complete(&d, own_id, Element::SceneHeading, "EXT. T").as_deref(),
        Some("WO - "),
        "a place already used, ready for its time of day"
    );
    assert_eq!(complete(&d, own_id, Element::SceneHeading, "INT. NEW - NI").as_deref(), Some("GHT"));
    assert_eq!(complete(&d, own_id, Element::Transition, "SMA").as_deref(), Some("SH CUT TO:"));
    assert_eq!(complete(&d, own_id, Element::Action, "MA"), None, "action is left alone");
}

#[test]
fn find_and_replace_keep_each_element_s_capitals() {
    let mut d = three_scenes();
    assert_eq!(d.find("two").len(), 3);
    let n = d.replace_all("two", "four");
    assert_eq!(n, 3);
    assert_eq!(d.blocks[4].text, "EXT. FOUR - NIGHT", "a heading stays in capitals");
    assert_eq!(d.blocks[7].text, "four, and then some.");
    assert_eq!(d.replace_all("", "x"), 0);
    let (s, n) = crate::model::replace_folded("Ab ab AB", "ab", "c");
    assert_eq!((s.as_str(), n), ("c c c", 3));
}

#[test]
fn fountain_is_read_into_typed_blocks() {
    let src = "Title: **The Long Way Down**\nCredit: written by\nAuthor: A. Writer\nDraft date: 1 May\n\nINT. WAREHOUSE - NIGHT #1#\n\n= Maria finds the door.\n\nRain hammers the roof. [[a note]]\n\nMARIA (V.O.)\n(barely audible)\nThree, four...\nthere you are.\n\n@McCLANE\nYippee.\n\n/* cut this\nwhole bit */\n\nCUT TO:\n\n.FLASHBACK\n\n> THE END <\n\n!SHOUTING IS ACTION\n\n> BURN TO WHITE\n";
    let d = fountain::parse(src);
    assert_eq!(d.meta.title, "The Long Way Down");
    assert_eq!(d.meta.author, "A. Writer");
    assert_eq!(d.meta.draft, "1 May");
    let kinds: Vec<Element> = d.blocks.iter().map(|b| b.element).collect();
    use Element::*;
    assert_eq!(
        kinds,
        vec![SceneHeading, Action, Character, Parenthetical, Dialogue, Character, Dialogue, Transition, SceneHeading, Action, Action, Transition]
    );
    assert_eq!(d.blocks[0].text, "INT. WAREHOUSE - NIGHT", "the scene number is dropped");
    assert_eq!(d.blocks[0].note, "Maria finds the door.");
    assert_eq!(d.blocks[1].text, "Rain hammers the roof.", "notes are dropped");
    assert_eq!(d.blocks[4].text, "Three, four... there you are.", "one speech, one block");
    assert_eq!(d.blocks[5].text, "MCCLANE");
    assert_eq!(d.blocks[9].text, "THE END");
    assert!(d.blocks.iter().all(|b| !b.text.contains("cut this")), "the boneyard is dropped");
}

#[test]
fn final_draft_round_trips() {
    let mut a = three_scenes();
    a.meta.author = "A. Writer & Co".into();
    a.blocks[1].note = "Maria <speaks> first.".into();
    a.push(Element::Shot, "ANGLE ON THE DOOR");
    a.push(Element::Transition, "SMASH CUT TO:");
    a.normalize();
    let x = export::to_fdx(&a);
    assert!(x.starts_with("<?xml"));
    assert!(x.contains("<Paragraph Type=\"Scene Heading\">"));
    assert!(x.contains("Maria &lt;speaks&gt; first."));
    let b = fountain::parse_fdx(&x);
    assert_eq!(b.meta.title, "Three");
    assert_eq!(b.meta.author, "A. Writer & Co");
    assert_eq!(b.blocks.len(), a.blocks.len());
    for (p, q) in a.blocks.iter().zip(b.blocks.iter()) {
        assert_eq!(p.element, q.element, "{}", p.text);
        assert_eq!(p.text, q.text);
    }
    assert_eq!(b.blocks[1].note, "Maria <speaks> first.");
}

#[test]
fn fountain_export_carries_the_synopsis_and_reads_back() {
    let mut a = three_scenes();
    a.blocks[1].note = "First.".into();
    let f = export::to_fountain(&a);
    assert!(f.contains("= First."));
    let b = fountain::parse(&f);
    assert_eq!(b.scenes().len(), 3);
    assert_eq!(b.scenes()[0].synopsis, "First.");
    assert_eq!(b.cast()[0].name, "MARIA");
}

#[test]
fn the_pdf_can_carry_scene_numbers() {
    let d = three_scenes();
    let path = std::env::temp_dir().join(format!("ns-numbers-{}.pdf", std::process::id()));
    export::to_pdf_with(&d, &path, true).expect("pdf");
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"%PDF"));
    let lines = export::compose(&d);
    let numbered: Vec<usize> = lines.iter().filter_map(|l| l.scene).collect();
    assert_eq!(numbered, vec![1, 2, 3]);
}

#[test]
fn settings_round_trip_and_share_tesseract_s_keys() {
    let mut s = Settings::default();
    assert_eq!(s.theme, ThemeId::Bloodmoon, "Northstar's own red, when not following Tesseract");
    s.theme = ThemeId::Void;
    s.light_mode = true;
    s.blur = BlurMode::Off;
    s.sort_by = SortBy::Length;
    s.after_export = AfterExport::Reveal;
    s.session_goal = 750;
    s.page_px = 19.0;
    s.scene_numbers = true;
    s.show_details = true;
    s.element_colors = false;
    s.character_colors = true;
    s.character_colors_reading = true;
    s.colour_seed = 7;
    s.youtrack_url = "https://example.youtrack.cloud/issues?q=for:me&sort=updated".into();
    assert_eq!(Settings::parse(&s.serialize()), s, "a link full of = and & survives");
    let d = Settings::default();
    assert_eq!(d.youtrack_url, "https://markedexiled.youtrack.cloud/dashboard?id=177-0");
    assert!(d.element_colors && !d.character_colors && !d.show_details);
    assert_eq!(Settings::parse("youtrack_url =\n").youtrack_url, d.youtrack_url, "an empty link falls back");
    s.theme = ThemeId::JetBrains;
    assert_eq!(Settings::parse(&s.serialize()).theme, ThemeId::JetBrains);

    // a file Tesseract wrote: its look is taken, nothing else
    let tess = "theme = frostbite\nlight_mode = yes\nanimations = no\nblur = always\nglass_opacity = 0.80\nshow_grid = no\nautosave_ms = 300\n";
    let t = Settings::parse(tess);
    let mut mine = Settings::default();
    assert!(mine.adopt_look(&t));
    assert_eq!(mine.theme, ThemeId::Frostbite);
    assert!(mine.light_mode && !mine.animations);
    assert_eq!(mine.blur, BlurMode::Always);
    assert!((mine.glass_opacity - 0.8).abs() < 1e-4);
    assert_eq!(mine.autosave_ms, 1200, "how often to save is Northstar's own business");
    assert!(!mine.adopt_look(&t), "adopting twice changes nothing");
    // garbage never panics and never goes out of range
    let g = Settings::parse("page_px = 900\nglass_opacity = -3\nsession_goal = x\n=\n");
    assert_eq!(g.page_px, 26.0);
    assert_eq!(g.glass_opacity, 0.35);
    assert_eq!(g.session_goal, 0);
}

#[test]
fn tesseract_s_default_theme_is_zen_when_its_file_names_none() {
    let _g = sandbox("tess-default");
    let dir = storage::tesseract_settings_path();
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    std::fs::write(&dir, "splash = no\n").unwrap();
    let (t, _) = storage::read_tesseract_settings().expect("read");
    assert_eq!(t.theme, ThemeId::Zen);
}

#[test]
fn snapshots_are_kept_per_script_and_follow_a_rename() {
    let _g = sandbox("snapshots");
    storage::ensure_dirs().unwrap();
    let d = three_scenes();
    let path = storage::unique_path("Three", None);
    storage::save(&path, &d).unwrap();
    storage::take_snapshot(&path, &d).unwrap();
    storage::take_snapshot(&path, &d).unwrap();
    let snaps = storage::list_snapshots(&path);
    assert_eq!(snaps.len(), 2, "two taken in the same second are both kept");
    assert!(snaps[0].pages >= 1);
    let back = storage::load(&snaps[0].path).unwrap();
    assert_eq!(back.scenes().len(), 3);
    let moved = storage::unique_path("Four", None);
    storage::follow_rename(&path, &moved);
    assert_eq!(storage::list_snapshots(&moved).len(), 2);
    assert!(storage::list_snapshots(&path).is_empty());
}

#[test]
fn imports_land_as_ordinary_scripts() {
    let _g = sandbox("import");
    let dir = std::env::temp_dir().join(format!("ns-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("my-heist_film.fountain");
    std::fs::write(&f, "EXT. BANK - DAY\n\nNobody moves.\n\nROSA\nNow.\n").unwrap();
    let d = storage::import_file(&f).unwrap();
    assert_eq!(d.meta.title, "my heist film", "named after the file when it has no title page");
    assert_eq!(d.scenes().len(), 1);
    assert_eq!(d.cast()[0].name, "ROSA");
    let x = dir.join("x.fdx");
    std::fs::write(&x, export::to_fdx(&three_scenes())).unwrap();
    assert_eq!(storage::import_file(&x).unwrap().scenes().len(), 3);
    assert!(storage::import_file(&dir.join("missing.fountain")).is_err());
}

#[test]
fn the_library_lists_pages_scenes_and_stars() {
    let _g = sandbox("library");
    storage::ensure_dirs().unwrap();
    let mut d = three_scenes();
    d.meta.starred = true;
    storage::save(&storage::unique_path("Three", None), &d).unwrap();
    let e = storage::list_scripts();
    assert_eq!(e.len(), 1);
    assert!(e[0].starred);
    assert_eq!(e[0].scenes, 3);
    assert_eq!(e[0].pages, 1);
    assert_eq!(e[0].preview, "FADE IN on nothing in particular.");
}

#[test]
fn odd_input_never_panics() {
    for s in [
        "", "---", "---\n", "##", "**", "``", "*()*", ">", "<!--", "<!-- scene: -->", "<!-- scene: tint=x -->",
        "## \u{1F3AC} ÉTÉ - JOUR\n<!-- scene: tint=99 | ü -->", "\u{0}\u{FFFF}", "#\n#\n#",
    ] {
        let d = storage::from_markdown(s);
        let _ = export::compose(&d);
        let _ = export::scene_lengths(&d);
        let _ = storage::to_markdown(&d);
        let _ = fountain::parse(s);
        let _ = fountain::parse_fdx(s);
        let _ = d.cast();
        let _ = complete(&d, 0, Element::SceneHeading, s);
    }
    // a tint past the palette wraps rather than panicking
    let d = storage::from_markdown("## A\n<!-- scene: tint=99 | x -->\n");
    assert_eq!(d.blocks[0].tint, Some(99));
    let _ = crate::theme::palette(ThemeId::Zen, true).group(99);
}

#[test]
fn every_palette_is_complete_in_both_casts() {
    for t in ThemeId::ALL {
        for dark in [true, false] {
            let p = crate::theme::palette(t, dark);
            assert_ne!(p.text, p.solid, "{t:?}");
            assert_eq!(p.dark, dark);
        }
    }
}

#[test]
fn the_icon_is_an_svg_drawn_from_the_mark() {
    let s = crate::logo::svg();
    assert!(s.starts_with("<svg"));
    assert_eq!(s.matches("<polygon").count(), 16, "eight rays, two facets each");
}

#[test]
fn every_speaker_gets_a_colour_of_their_own_that_does_not_move() {
    crate::theme::set_palette(ThemeId::JetBrains, true);
    let names: Vec<String> = ["MARIA", "COLE", "JONAH", "ADA", "THE WRITER", "RUTH", "KAI"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let a = crate::theme::character_colors(&names, 0);
    let distinct: std::collections::HashSet<_> = a.values().map(|c| c.to_array()).collect();
    assert_eq!(distinct.len(), names.len(), "no two speakers share a colour");
    // someone new arrives: nobody else changes colour
    let mut more = names.clone();
    more.push("NEWCOMER".into());
    let b = crate::theme::character_colors(&more, 0);
    for n in &names {
        assert_eq!(a[n], b[n], "{n} kept their colour");
    }
    // Shuffle turns the wheel
    let c = crate::theme::character_colors(&names, 1);
    assert_ne!(a["MARIA"], c["MARIA"]);
    // readable on the page in both casts
    for dark in [true, false] {
        crate::theme::set_palette(ThemeId::Zen, dark);
        for col in crate::theme::character_colors(&names, 3).values() {
            let l = 0.299 * col.r() as f32 + 0.587 * col.g() as f32 + 0.114 * col.b() as f32;
            if dark {
                assert!(l > 120.0, "light enough on a dark page: {col:?}");
            } else {
                assert!(l < 150.0, "dark enough on a white page: {col:?}");
            }
        }
    }
    assert_eq!(three_scenes().speakers(), vec!["MARIA".to_string(), "COLE".to_string()]);
}

#[test]
fn the_jetbrains_theme_is_one_of_the_set() {
    assert_eq!(ThemeId::from_slug("jetbrains"), Some(ThemeId::JetBrains));
    let p = crate::theme::palette(ThemeId::JetBrains, true);
    // the near-black ground and the warm end of the arches
    assert!(p.backdrop.r() < 0x20 && p.backdrop.g() < 0x20);
    assert!(p.sec_grad.1.r() >= 0xE8 && p.sec_grad.1.g() > 0xA0, "marigold");
    // warm, but never pure neon
    assert!(p.sec_grad.1.b() > 0x30 && p.prim_grad.1.g() > 0x20);
    assert_eq!(ThemeId::ALL.len(), 6);
}

#[test]
fn a_tint_is_exactly_as_strong_as_asked() {
    use eframe::egui::Color32;
    let c = Color32::from_rgb(200, 100, 50);
    let t = crate::theme::tint(c, 0.05);
    assert_eq!(t.a(), 13);
    assert_eq!((t.r(), t.g(), t.b()), (10, 5, 3), "premultiplied at 5%, not boosted");
    assert_eq!(crate::theme::tint(c, 0.0), Color32::TRANSPARENT);
}

#[test]
fn a_page_selection_reads_like_a_print_dialog() {
    use export::{describe_pages, parse_page_range as r};
    assert_eq!(r("1-3, 7, 10-", 12).unwrap(), vec![1, 2, 3, 7, 10, 11, 12]);
    assert_eq!(r("3", 12).unwrap(), vec![3]);
    assert_eq!(r("-2", 12).unwrap(), vec![1, 2], "an open start is page 1");
    assert_eq!(r("7 3;3,1", 12).unwrap(), vec![1, 3, 7], "sorted, without repeats");
    assert_eq!(r("10-99", 12).unwrap(), vec![10, 11, 12], "a range past the end stops at it");
    assert_eq!(r("2\u{2013}4", 12).unwrap(), vec![2, 3, 4], "an en dash is a dash");
    for bad in ["", "0", "5-2", "abc", "13", "1-x"] {
        assert!(r(bad, 12).is_err(), "{bad:?} should be refused");
    }
    assert_eq!(r("20", 12).unwrap_err(), "There are only 12 pages");
    assert_eq!(describe_pages(&[1, 2, 3, 7, 10, 11, 12]), "1-3, 7, 10-12");
    assert_eq!(describe_pages(&[4]), "4");
}

fn long_speeches() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Voices".into();
    d.push(Element::SceneHeading, "INT. HALL - NIGHT");
    for k in 0..40 {
        d.push(Element::Action, "Somebody crosses the hall and the floor answers every step.");
        d.push(Element::Character, if k % 2 == 0 { "MARIA" } else { "COLE (V.O.)" });
        d.push(Element::Parenthetical, "(low)");
        d.push(Element::Dialogue, "Say it once, say it plainly, and then say nothing at all for the rest of the night.");
    }
    d.reseed_ids();
    d
}

#[test]
fn a_pdf_plan_keeps_chosen_pages_their_numbers_and_their_voices() {
    let d = long_speeches();
    let total = export::page_count(&d);
    assert!(total >= 3, "{total}");
    let inks: std::collections::HashMap<String, [u8; 3]> =
        [("MARIA".to_string(), [200, 0, 0]), ("COLE".to_string(), [0, 0, 200])].into_iter().collect();
    let opts = export::PdfOptions {
        pages: Some(vec![2, total]),
        voices: Some(inks),
        ..export::PdfOptions::default()
    };
    let plan = export::pdf_plan(&d, &opts);
    assert_eq!(plan.iter().map(|p| p.number).collect::<Vec<_>>(), vec![2, total], "numbered as in the whole script");
    for page in &plan {
        for (line, ink) in &page.lines {
            match line.element {
                Some(Element::Action) | Some(Element::SceneHeading) => assert_eq!(*ink, None, "{}", line.text),
                Some(Element::Character) | Some(Element::Parenthetical) | Some(Element::Dialogue) => {
                    assert!(ink.is_some(), "speech carries its speaker's ink: {}", line.text)
                }
                _ => {}
            }
        }
    }
    // a speech broken over a page keeps its voice on the next page
    let all = export::pdf_plan(&d, &export::PdfOptions { voices: opts.voices.clone(), ..Default::default() });
    for w in all.windows(2) {
        if let Some((l, ink)) = w[1].lines.iter().find(|(l, _)| l.element.is_some()) {
            if l.element == Some(Element::Dialogue) {
                assert!(ink.is_some(), "carried over the break");
            }
        }
    }
    // without voices, everything is black
    assert!(export::pdf_plan(&d, &export::PdfOptions::default())
        .iter()
        .all(|p| p.lines.iter().all(|(_, i)| i.is_none())));
}

#[test]
fn a_pdf_of_some_pages_has_just_those_pages() {
    let d = long_speeches();
    let dir = std::env::temp_dir();
    let count = |path: &std::path::Path| {
        let b = std::fs::read(path).unwrap();
        let t = String::from_utf8_lossy(&b);
        t.matches("/Type /Page\n").count() + t.matches("/Type/Page\n").count()
            + t.matches("/Type /Page>>").count() + t.matches("/Type/Page/").count()
            + t.matches("/Type /Page/").count() + t.matches("/Type /Page ").count()
    };
    let whole = dir.join(format!("ns-whole-{}.pdf", std::process::id()));
    export::to_pdf_opts(&d, &whole, &export::PdfOptions::default()).unwrap();
    let some = dir.join(format!("ns-some-{}.pdf", std::process::id()));
    let inks = crate::theme::character_inks(&d.speakers(), 0);
    export::to_pdf_opts(
        &d,
        &some,
        &export::PdfOptions {
            title_page: false,
            pages: Some(vec![2]),
            voices: Some(inks),
            scene_numbers: true,
        },
    )
    .unwrap();
    let (w, s) = (count(&whole), count(&some));
    assert_eq!(w, 1 + export::page_count(&d), "title page and every page");
    assert_eq!(s, 1, "one chosen page, no title page");
    assert!(export::to_pdf_opts(
        &d,
        &some,
        &export::PdfOptions { title_page: false, pages: Some(vec![]), ..Default::default() }
    )
    .is_err(), "an empty PDF is refused");
}

#[test]
fn character_inks_read_on_paper_whatever_the_app_theme() {
    let names: Vec<String> = (0..9).map(|k| format!("SPEAKER {k}")).collect();
    for dark in [true, false] {
        crate::theme::set_palette(ThemeId::JetBrains, dark);
        let inks = crate::theme::character_inks(&names, 5);
        let app = crate::theme::character_colors(&names, 5);
        for n in &names {
            let [r, g, b] = inks[n];
            let l = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            assert!(l < 140.0, "{n} is dark enough for white paper ({l})");
            // the same hue as in the app: the strongest channel agrees
            let c = app[n];
            let top = |x: [u8; 3]| (0..3).max_by_key(|&i| x[i]).unwrap();
            assert_eq!(top([r, g, b]), top([c.r(), c.g(), c.b()]), "{n} keeps its hue");
        }
    }
    // scene numbers used to be one switch; an old file keeps its PDF as it was
    assert!(Settings::parse("scene_numbers = yes\n").pdf_scene_numbers);
    assert!(!Settings::parse("scene_numbers = yes\npdf_scene_numbers = no\n").pdf_scene_numbers);
}

// ------------------------------------------------------------------ acts --

/// Two acts, two scenes each — the second act long enough to run over a page.
fn two_acts() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Pilot".into();
    d.push(Element::Act, "ACT ONE");
    d.push(Element::SceneHeading, "INT. DINER - DAY");
    d.push(Element::Action, "Coffee steams.");
    d.push(Element::SceneHeading, "EXT. LOT - DAY");
    d.push(Element::Character, "MARIA");
    d.push(Element::Dialogue, "We're late.");
    d.push(Element::Act, "ACT TWO");
    d.push(Element::SceneHeading, "INT. CAR - NIGHT");
    for k in 0..30 {
        d.push(Element::Action, &format!("The road unrolls, mile {k} of it, headlights on the white line."));
    }
    d.push(Element::SceneHeading, "EXT. MOTEL - NIGHT");
    d.push(Element::Action, "A sign buzzes.");
    d.reseed_ids();
    d
}

#[test]
fn acts_are_titled_and_closed_in_words() {
    use crate::model::{act_title, end_of};
    assert_eq!(act_title(1), "ACT ONE");
    assert_eq!(act_title(5), "ACT FIVE");
    assert_eq!(act_title(20), "ACT TWENTY");
    assert_eq!(act_title(21), "ACT 21");
    assert_eq!(end_of("ACT TWO"), "END OF ACT TWO");
    assert_eq!(end_of("teaser"), "END OF TEASER");
    assert_eq!(end_of(""), "END OF ACT");
}

#[test]
fn an_act_round_trips_through_the_file_as_a_level_one_heading() {
    let a = two_acts();
    let md = storage::to_markdown(&a);
    assert!(md.contains("\n# ACT ONE\n\n## INT. DINER - DAY\n"), "{md}");
    assert!(md.contains("\n# ACT TWO\n"));
    let b = storage::from_markdown(&md);
    let kinds = |d: &Document| d.blocks.iter().map(|b| (b.element, b.text.clone())).collect::<Vec<_>>();
    assert_eq!(kinds(&a), kinds(&b));
    // an act not yet named is still an act
    let mut c = two_acts();
    c.blocks[0].text.clear();
    let back = storage::from_markdown(&storage::to_markdown(&c));
    assert_eq!(back.blocks[0].element, Element::Act);
    assert_eq!(back.blocks[0].text, "");
}

#[test]
fn scenes_end_where_an_act_begins_and_acts_span_their_scenes() {
    let d = two_acts();
    let scenes = d.scenes();
    assert_eq!(scenes.len(), 4);
    // the lot scene stops before ACT TWO
    assert_eq!(scenes[1].end, 6);
    assert_eq!(d.blocks[scenes[1].end].element, Element::Act);
    let acts = d.acts();
    assert_eq!(acts.len(), 2);
    assert_eq!((acts[0].number, acts[0].title.as_str(), acts[0].start, acts[0].end), (1, "ACT ONE", 0, 6));
    assert_eq!((acts[1].start, acts[1].end), (6, d.blocks.len()));
    // the act itself is in no scene; what follows its heading is
    assert_eq!(d.scene_of(d.blocks[6].id), None);
    assert_eq!(d.scene_of(d.blocks[8].id), Some(2));
    assert_eq!(d.act_of(d.blocks[8].id), Some(1));
    assert_eq!(d.act_of(d.blocks[0].id), Some(0));
    // and a script without acts has none
    assert!(sample().acts().is_empty());
    assert_eq!(sample().scenes().len(), 1);
}

#[test]
fn every_act_starts_a_page_centred_and_ends_with_its_end_of_line() {
    let d = two_acts();
    let lines = export::compose(&d);
    let act_lines: Vec<&export::Line> = lines.iter().filter(|l| l.element == Some(Element::Act)).collect();
    let texts: Vec<&str> = act_lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, ["ACT ONE", "END OF ACT ONE", "ACT TWO", "END OF ACT TWO"]);
    for l in &act_lines {
        assert_eq!(l.indent, (60 - l.text.len()) / 2, "{} is centred", l.text);
    }
    assert!(act_lines[1].is_act_end() && act_lines[1].block.is_none());
    assert!(act_lines[0].is_act_title());

    let pages = export::paginate(&lines);
    let first = |pg: &Vec<export::Line>| pg.iter().find(|l| !l.text.trim().is_empty()).map(|l| l.text.clone());
    assert_eq!(first(&pages[0]).as_deref(), Some("ACT ONE"));
    let two = pages.iter().position(|pg| first(pg).as_deref() == Some("ACT TWO")).expect("ACT TWO opens a page");
    assert_eq!(two, 1, "act one fits on one page, so act two is page 2");
    // act one's close is on its own last page, not the next act's
    assert!(pages[0].iter().any(|l| l.text == "END OF ACT ONE"));
    assert!(pages.len() >= 3, "act two runs over");
    // the page map still points at real blocks
    let starts = export::page_starts(&d);
    assert_eq!(starts[0], (2, d.blocks[6].id));
    assert_eq!(export::page_of_block(&d, d.blocks[6].id), Some(2));
}

#[test]
fn an_end_of_line_is_never_alone_at_the_top_of_a_page() {
    // fill act one so its last line lands exactly at the foot of page one
    for fill in 40..60 {
        let mut d = Document::default();
        d.blocks.clear();
        d.push(Element::Act, "ACT ONE");
        d.push(Element::SceneHeading, "INT. HALL - DAY");
        for k in 0..fill {
            d.push(Element::Character, "MARIA");
            d.push(Element::Dialogue, &format!("Line {k}."));
        }
        d.reseed_ids();
        for pg in export::paginate(&export::compose(&d)) {
            let first = pg.iter().find(|l| !l.text.trim().is_empty()).unwrap();
            assert!(!first.is_act_end(), "END OF opened a page with {fill} speeches");
            assert!(pg.len() <= crate::model::LINES_PER_PAGE + 4);
        }
    }
}

#[test]
fn acts_survive_fountain_and_final_draft_without_doubling_their_close() {
    let d = two_acts();
    let f = export::to_fountain(&d);
    assert!(f.contains("# ACT ONE\n\n>ACT ONE<"), "{f}");
    assert!(f.contains(">END OF ACT ONE<\n\n===\n\n# ACT TWO"), "{f}");
    assert!(f.trim_end().ends_with(">END OF ACT TWO<"));
    let back = crate::fountain::parse(&f);
    let acts: Vec<String> = back.blocks.iter().filter(|b| b.element == Element::Act).map(|b| b.text.clone()).collect();
    assert_eq!(acts, ["ACT ONE", "ACT TWO"]);
    assert!(!back.blocks.iter().any(|b| b.text.contains("END OF")), "the close is not kept twice");
    assert_eq!(back.scenes().len(), 4);

    let x = export::to_fdx(&d);
    assert!(x.contains("<Paragraph Type=\"New Act\">\n      <Text>ACT ONE</Text>"));
    assert!(x.contains("<Paragraph Type=\"End of Act\">\n      <Text>END OF ACT TWO</Text>"));
    let back = crate::fountain::parse_fdx(&x);
    let acts: Vec<String> = back.blocks.iter().filter(|b| b.element == Element::Act).map(|b| b.text.clone()).collect();
    assert_eq!(acts, ["ACT ONE", "ACT TWO"]);
    assert!(!back.blocks.iter().any(|b| b.text.contains("END OF")));

    // other apps' ways of marking acts
    let other = "Title: X\n\n# Teaser\n\nEXT. SEA - DAY\n\nWaves.\n\n>END OF TEASER<\n\n===\n\n>**_ACT ONE_**<\n\nINT. SHIP - DAY\n\nCreaks.\n\n## A sequence\n\n>THE END<\n";
    let back = crate::fountain::parse(other);
    let kinds: Vec<(Element, &str)> = back.blocks.iter().map(|b| (b.element, b.text.as_str())).collect();
    assert_eq!(
        kinds,
        [
            (Element::Act, "TEASER"),
            (Element::SceneHeading, "EXT. SEA - DAY"),
            (Element::Action, "Waves."),
            (Element::Act, "ACT ONE"),
            (Element::SceneHeading, "INT. SHIP - DAY"),
            (Element::Action, "Creaks."),
            (Element::Action, "THE END"),
        ]
    );
}

#[test]
fn a_pdf_with_acts_is_written() {
    let bytes = export::pdf_bytes(&two_acts(), &export::PdfOptions::default()).unwrap();
    assert_eq!(&bytes[..5], b"%PDF-");
    let txt = export::to_plain_text(&two_acts());
    assert!(txt.contains(&format!("{}ACT ONE\n", " ".repeat(26))));
    assert!(txt.contains("END OF ACT TWO"));
}

// ----------------------------------------------------- chosen colours --

#[test]
fn chosen_colours_ride_in_the_front_matter_and_only_when_there_are_some() {
    let mut d = sample();
    d.meta.set_voice("MARIA", Some(212));
    d.meta.set_voice("COLE", Some(384));
    let md = storage::to_markdown(&d);
    assert!(md.contains("\nvoices: MARIA=212; COLE=24\n---\n"), "{md}");
    let back = storage::from_markdown(&md);
    assert_eq!(back.meta.voices, vec![("MARIA".to_string(), 212), ("COLE".to_string(), 24)]);
    assert_eq!(back.meta.voice("COLE"), Some(24));
    // handing one back to chance
    let mut e = back.clone();
    e.meta.set_voice("MARIA", None);
    assert_eq!(e.meta.voices, vec![("COLE".to_string(), 24)]);
    // no colours, no line: the file is exactly as it was
    assert!(!storage::to_markdown(&sample()).contains("voices"));
    // anything odd in the line is skipped, not fatal
    let odd = "---\ntitle: X\nvoices: MARIA=; =40; BOB=12; bob=99; SAL=x\n---\n\n## INT. A - DAY\n";
    assert_eq!(storage::from_markdown(odd).meta.voices, vec![("BOB".to_string(), 12)]);
}

#[test]
fn custom_colours_keep_the_chosen_hue_and_deal_the_rest_as_random_does() {
    use crate::theme;
    let names: Vec<String> = ["MARIA", "COLE", "JUNE"].iter().map(|s| s.to_string()).collect();
    for dark in [true, false] {
        theme::set_palette(theme::ThemeId::Bloodmoon, dark);
        let random = theme::character_colors(&names, 3);
        // nothing chosen is Random, exactly
        assert_eq!(theme::voice_colors(&names, 3, &[]), random);
        let chosen = vec![("COLE".to_string(), 120u16), ("NOBODY".to_string(), 10)];
        let custom = theme::voice_colors(&names, 3, &chosen);
        assert_eq!(custom["COLE"], theme::voice_of_hue(120));
        assert_eq!(custom["MARIA"], random["MARIA"], "the others are dealt as before");
        assert_eq!(custom["JUNE"], random["JUNE"]);
        assert!(!custom.contains_key("NOBODY"), "a colour for someone who never speaks is ignored");
        // on paper: the same hue, as ink
        let inks = theme::voice_inks(&names, 3, &chosen);
        assert_eq!(theme::voice_inks(&names, 3, &[]), theme::character_inks(&names, 3));
        let [r, g, b] = inks["COLE"];
        assert!(g > r && g > b, "120 is green on paper too: {:?}", inks["COLE"]);
        assert!((r as u32 + g as u32 + b as u32) < 3 * 160, "deep enough to read as ink");
    }
}

#[test]
fn the_colour_mode_is_remembered_and_written_only_once_chosen() {
    use crate::settings::Settings;
    let plain = Settings::default().serialize();
    assert!(!plain.contains("character_colors_mode"), "a file that never chose stays as it was");
    let s = Settings {
        custom_colors: true,
        ..Default::default()
    };
    let text = s.serialize();
    assert!(text.contains("character_colors_mode = custom"));
    assert!(Settings::parse(&text).custom_colors);
    assert!(!Settings::parse(&plain).custom_colors);
}

// ------------------------------------------------------------- shortcuts --

#[test]
fn every_default_shortcut_reads_back_and_the_specific_chord_is_heard_first() {
    use crate::keys::{self, Command, Keymap};
    use eframe::egui::{Key, KeyboardShortcut, Modifiers};
    for web in [false, true] {
        for cmd in Command::ALL {
            assert_eq!(Command::from_slug(&cmd.slug()), Some(cmd));
            for k in cmd.defaults(web) {
                assert_eq!(keys::parse(&keys::text(&k)), Some(k), "{}", keys::text(&k));
            }
        }
        // a chord with Shift or Alt is asked for before the plainer one with
        // the same key, as the hand-written order always had it
        let order = Keymap::default().listen(&Command::ALL, web);
        for (i, (_, a)) in order.iter().enumerate() {
            for (_, b) in &order[i + 1..] {
                let more = |x: &KeyboardShortcut| x.modifiers.shift as u8 + x.modifiers.alt as u8;
                if a.logical_key == b.logical_key {
                    assert!(more(a) >= more(b), "{} before {}", keys::text(a), keys::text(b));
                }
            }
        }
    }
    assert_eq!(keys::text(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma)), "Ctrl+,");
    assert_eq!(keys::parse("Ctrl++"), Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::Plus)));
    assert_eq!(keys::parse("ctrl+shift+enter"), Some(KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Enter)));
    assert_eq!(keys::parse("Hyper+Q"), None);
    assert_eq!(Keymap::default().label(Command::NewScript, true), "Ctrl+Alt+N");
    assert_eq!(Keymap::default().label(Command::Element(3), true), "Alt+3");
    assert_eq!(Keymap::default().label(Command::Element(3), false), "Ctrl+3");
}

#[test]
fn a_chosen_shortcut_is_kept_only_while_it_differs_from_the_default() {
    use crate::keys::{self, Command, Keymap};
    use crate::settings::Settings;
    use eframe::egui::{Key, KeyboardShortcut, Modifiers};
    let alt_d = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::ALT, Key::D);
    let mut s = Settings::default();
    assert!(!s.serialize().contains("key."), "nothing chosen, nothing written");
    s.keys.set(Command::ToggleDetails, alt_d, true);
    assert_eq!(s.keys.chords(Command::ToggleDetails, true), vec![alt_d]);
    let text = s.serialize();
    assert!(text.contains("key.details = Ctrl+Alt+D\n"), "{text}");
    let back = Settings::parse(&text);
    assert_eq!(back.keys, s.keys);
    // choosing the default again is the default
    s.keys.set(Command::ToggleDetails, KeyboardShortcut::new(Modifiers::COMMAND, Key::I), true);
    assert!(s.keys.is_default());
    // who already has a chord
    let km = Keymap::default();
    let ctrl_s = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
    assert_eq!(km.taken_by(&ctrl_s, Command::Undo, &Command::ALL, false), Some(Command::Save));
    assert_eq!(km.taken_by(&ctrl_s, Command::Save, &Command::ALL, false), None);
    // what cannot be a shortcut
    let refuse = |m: Modifiers, k: Key, web: bool| keys::refused(&KeyboardShortcut::new(m, k), web);
    assert!(refuse(Modifiers::NONE, Key::Q, false).is_some(), "a plain key is for typing");
    assert!(refuse(Modifiers::SHIFT, Key::Enter, false).is_some());
    assert!(refuse(Modifiers::NONE, Key::F2, false).is_none(), "function keys are fine bare");
    assert!(refuse(Modifiers::COMMAND, Key::W, true).is_some(), "a browser keeps Ctrl+W");
    assert!(refuse(Modifiers::COMMAND, Key::Num4, true).is_some());
    assert!(refuse(Modifiers::ALT, Key::Num4, true).is_none());
    assert!(refuse(Modifiers::COMMAND, Key::V, false).is_some());
    assert!(refuse(Modifiers::ALT, Key::ArrowUp, false).is_some());
    assert!(refuse(Modifiers::COMMAND | Modifiers::ALT, Key::D, true).is_none());
}
