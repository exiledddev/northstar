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
    assert_eq!(Settings::parse(&s.serialize()), s);

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
