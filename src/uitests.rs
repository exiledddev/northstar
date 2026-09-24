//! Headless UI tests. egui runs without a window, so these build a real App,
//! feed it synthetic clicks, drags and keystrokes, and read back what it did —
//! the same harness Tesseract is tested with.

use std::time::Duration;

use crate::app::{App, Mode};
use crate::editor;
use crate::model::{Document, Element};
use crate::storage;
use crate::tests::VAULT;
use crate::theme::{self, ThemeId};
use eframe::egui::{self, pos2, vec2, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};

static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct Harness {
    ctx: egui::Context,
    app: App,
    size: egui::Vec2,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn sandbox(settings: &str) -> std::sync::MutexGuard<'static, ()> {
    let guard = VAULT.lock().unwrap_or_else(|e| e.into_inner());
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("northstar-ui-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(dir.join("northstar"));
    std::env::set_var("XDG_DATA_HOME", &dir);
    // No splash and no easing, so a frame is a frame.
    let _ = std::fs::write(
        dir.join("northstar/settings.conf"),
        format!("splash = no\nanimations = no\nmatch_tesseract = no\nautosave_ms = 150\n{settings}"),
    );
    guard
}

impl Harness {
    fn new() -> Harness {
        Harness::with(1320.0, 900.0, "")
    }

    fn with(w: f32, h: f32, settings: &str) -> Harness {
        let guard = sandbox(settings);
        let ctx = egui::Context::default();
        let warm = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h))),
            ..Default::default()
        };
        let _ = ctx.run(warm, |_| {});
        let app = App::with_context(&ctx);
        let mut h = Harness {
            ctx,
            app,
            size: vec2(w, h),
            _guard: guard,
        };
        h.frames(3);
        h
    }

    fn frame_with(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), self.size)),
            events,
            ..Default::default()
        };
        let ctx = self.ctx.clone();
        let app = &mut self.app;
        let _ = ctx.run(input, |c| app.frame(c));
    }

    fn frame(&mut self) {
        self.frame_with(Vec::new());
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame();
        }
    }

    fn type_text(&mut self, s: &str) {
        self.frame_with(vec![Event::Text(s.to_string())]);
        self.frame();
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.frame_with(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.frames(2);
    }

    fn click(&mut self, at: Pos2) {
        self.frame_with(vec![Event::PointerMoved(at)]);
        self.frame_with(vec![
            Event::PointerMoved(at),
            Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.frame_with(vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame();
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame_with(vec![Event::PointerMoved(from)]);
        self.frame_with(vec![
            Event::PointerMoved(from),
            Event::PointerButton {
                pos: from,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        for i in 1..=6 {
            let t = i as f32 / 6.0;
            let p = pos2(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            self.frame_with(vec![Event::PointerMoved(p)]);
        }
        self.frame_with(vec![Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frames(2);
    }

    /// Put the caret at the end of block `id`.
    fn focus(&mut self, id: u64) {
        self.app.focus_on(id);
        self.frames(3);
        assert_eq!(self.app.focus_block(), Some(id), "the block should have the caret");
    }

    fn block_rect(&self, id: u64) -> Option<Rect> {
        self.ctx.read_response(editor::block_id(id)).map(|r| r.rect)
    }

    fn confirm(&mut self) {
        assert!(self.app.debug_pending_ask(), "expected the app to ask before doing that");
        self.app.debug_answer_yes(None);
        self.frames(2);
    }

    fn saved_text(&self) -> String {
        std::fs::read_to_string(self.app.path().expect("a path")).unwrap_or_default()
    }

    /// Let autosave run.
    fn idle(&mut self) {
        std::thread::sleep(Duration::from_millis(200));
        self.frames(2);
    }

    fn load(&mut self, doc: Document) {
        let path = storage::unique_path(&doc.meta.title, None);
        storage::save(&path, &doc).unwrap();
        // written from outside the app, as a file dropped into the folder
        // would be; the library notices it on its own
        std::thread::sleep(Duration::from_millis(1600));
        self.frames(2);
        assert!(
            self.app.entries().iter().any(|e| e.path == path),
            "the library should pick up a script added from outside"
        );
        self.app.debug_open(path);
        self.frames(3);
    }
}

fn three_scenes() -> Document {
    let mut d = Document::default();
    d.blocks.clear();
    d.meta.title = "Three Scenes".into();
    d.push(Element::SceneHeading, "INT. ONE - DAY");
    d.push(Element::Character, "DETECTIVE COLE");
    d.push(Element::Dialogue, "One.");
    d.push(Element::SceneHeading, "EXT. TWO - NIGHT");
    d.push(Element::Action, "Two.");
    d.push(Element::SceneHeading, "INT. THREE - DAWN");
    d.push(Element::Action, "Three.");
    d.reseed_ids();
    d
}

fn ids(app: &App) -> Vec<u64> {
    app.doc().blocks.iter().map(|b| b.id).collect()
}

// ---------- the library ----------

#[test]
fn a_first_run_opens_on_a_starter_script_that_is_on_disk() {
    let h = Harness::new();
    assert_eq!(h.app.entry_count(), 1);
    assert!(h.app.path().unwrap().exists());
    assert!(h.saved_text().contains("## INT. EDIT BAY - NIGHT"));
    assert_eq!(h.app.doc().scenes().len(), 1);
}

#[test]
fn ctrl_n_makes_a_new_script_with_its_title_ready_to_type_over() {
    let mut h = Harness::new();
    h.press(Key::N, Modifiers::COMMAND);
    assert_eq!(h.app.entry_count(), 2);
    assert_eq!(h.app.doc().meta.title, "Untitled Script");
    // the title is selected, so typing replaces it
    h.type_text("Heist");
    assert_eq!(h.app.doc().meta.title, "Heist");
    h.press(Key::S, Modifiers::COMMAND);
    assert!(h.app.path().unwrap().ends_with("heist.md"), "the file follows the title");
}

#[test]
fn deleting_a_script_from_the_menu_asks_first() {
    let mut h = Harness::new();
    h.load(three_scenes());
    assert_eq!(h.app.entry_count(), 2);
    let doomed = h.app.path().unwrap();
    let menu = h.app.debug_menu_button();
    h.click(menu.center());
    assert!(h.app.debug_popover_open(), "the menu should open");
    // the last row of the menu deletes
    let mut last = None;
    for n in 0..30 {
        if let Some(r) = h.app.debug_menu_item(n) {
            last = Some(r);
        }
    }
    h.click(last.expect("menu rows").center());
    assert!(doomed.exists(), "nothing is destroyed on a single click");
    h.confirm();
    assert!(!doomed.exists());
    assert_eq!(h.app.entry_count(), 1);
    assert_ne!(h.app.path(), Some(doomed), "another script is opened in its place");
}

#[test]
fn a_script_is_opened_from_its_row() {
    let mut h = Harness::new();
    let starter = h.app.path().unwrap();
    h.load(three_scenes());
    let row = h.app.debug_row_rect(&starter).expect("the starter's row");
    h.click(row.left_center() + vec2(30.0, 0.0));
    assert_eq!(h.app.path(), Some(starter));
}

// ---------- writing ----------

#[test]
fn typing_in_a_block_reaches_the_file() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.type_text(" Rain.");
    assert!(h.app.doc().blocks[1].text.ends_with("Rain."));
    assert!(h.app.is_dirty());
    h.idle();
    assert!(!h.app.is_dirty(), "autosave ran");
    assert!(h.saved_text().contains("empty document. Rain."));
}

#[test]
fn enter_makes_the_element_that_comes_next() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let cue = h.app.doc().blocks[1].id; // DETECTIVE COLE
    h.focus(cue);
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[2].element, Element::Dialogue, "Character → Dialogue");
    h.type_text("Hello.");
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[3].element, Element::Action, "Dialogue → Action");
    h.press(Key::Tab, Modifiers::NONE); // this action → a character cue
    assert_eq!(h.app.doc().blocks[3].element, Element::Character);
    h.press(Key::Enter, Modifiers::SHIFT);
    assert_eq!(
        h.app.doc().blocks[4].element,
        Element::Character,
        "Shift+Enter keeps the type rather than moving on to dialogue"
    );
}

#[test]
fn tab_cycles_and_ctrl_digits_set_the_element() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id; // action
    h.focus(id);
    h.press(Key::Tab, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks[1].element, Element::Character);
    assert_eq!(h.app.doc().blocks[1].text, h.app.doc().blocks[1].text.to_uppercase(), "a cue is capitals");
    h.press(Key::Tab, Modifiers::SHIFT);
    assert_eq!(h.app.doc().blocks[1].element, Element::Action);
    h.press(Key::Num6, Modifiers::COMMAND);
    assert_eq!(h.app.doc().blocks[1].element, Element::Transition);
    h.press(Key::Z, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND);
    assert_ne!(h.app.doc().blocks[1].element, Element::Transition, "undo walks it back");
}

#[test]
fn smart_type_offers_a_name_and_tab_takes_it() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let dialogue = h.app.doc().blocks[2].id;
    h.focus(dialogue);
    h.press(Key::Enter, Modifiers::NONE); // → action
    h.press(Key::Tab, Modifiers::NONE); // → character
    let cue = h.app.focus_block().unwrap();
    assert_eq!(h.app.doc().block(cue).unwrap().element, Element::Character);
    h.type_text("DET");
    h.press(Key::Tab, Modifiers::NONE);
    assert_eq!(h.app.doc().block(cue).unwrap().text, "DETECTIVE COLE");
    assert_eq!(h.app.doc().block(cue).unwrap().element, Element::Character, "Tab took the name, not the next type");
}

#[test]
fn a_multi_line_paste_fans_out_into_typed_blocks() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    let before = h.app.doc().blocks.len();
    h.frame_with(vec![Event::Paste(
        "\nINT. CAR - DAY\nMARIA\n(beat)\nDrive.\nCUT TO:".to_string(),
    )]);
    h.frames(3);
    let kinds: Vec<Element> = h.app.doc().blocks.iter().map(|b| b.element).collect();
    assert_eq!(h.app.doc().blocks.len(), before + 5);
    assert_eq!(
        kinds[2..7],
        [Element::SceneHeading, Element::Character, Element::Parenthetical, Element::Dialogue, Element::Transition]
    );
}

#[test]
fn backspace_at_the_head_of_an_empty_block_removes_it() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.press(Key::Enter, Modifiers::NONE);
    let n = h.app.doc().blocks.len();
    h.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks.len(), n - 1);
    assert_eq!(h.app.focus_block(), Some(id));
}

#[test]
fn alt_arrows_move_the_block() {
    let mut h = Harness::new();
    let before = ids(&h.app);
    h.focus(before[2]);
    h.press(Key::ArrowUp, Modifiers::ALT);
    let after = ids(&h.app);
    assert_eq!(after[1], before[2]);
    assert_eq!(after[2], before[1]);
}

#[test]
fn ctrl_enter_opens_a_new_scene_after_this_one() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let in_first = h.app.doc().blocks[2].id;
    h.focus(in_first);
    h.press(Key::Enter, Modifiers::COMMAND);
    let sc = h.app.doc().scenes();
    assert_eq!(sc.len(), 4);
    assert_eq!(sc[1].heading, "", "a fresh heading straight after the first scene");
    assert_eq!(sc[2].heading, "EXT. TWO - NIGHT");
    h.type_text("INT. NEW - DAY");
    assert_eq!(h.app.doc().scenes()[1].heading, "INT. NEW - DAY");
}

#[test]
fn page_text_size_follows_ctrl_plus_and_minus_and_is_remembered() {
    let mut h = Harness::new();
    let px = h.app.settings().page_px;
    h.press(Key::Plus, Modifiers::COMMAND);
    h.press(Key::Plus, Modifiers::COMMAND);
    assert_eq!(h.app.settings().page_px, px + 2.0);
    h.press(Key::Minus, Modifiers::COMMAND);
    assert_eq!(h.app.settings().page_px, px + 1.0);
    assert!(storage::read_settings().page_px == px + 1.0);
}

#[test]
fn page_breaks_are_marked_where_the_pdf_breaks() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    for _ in 0..90 {
        d.push(Element::Action, "They run, and they keep running until the corridor ends.");
    }
    d.reseed_ids();
    h.load(d);
    let starts = h.app.debug_page_starts();
    assert_eq!(starts.len() + 1, crate::export::page_count(h.app.doc()));
    assert!(starts.iter().any(|(_, n)| *n == 2));
}

// ---------- the other two views ----------

#[test]
fn cards_move_a_whole_scene_by_dragging_its_head() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let first = h.app.debug_card_rect(0).expect("a card");
    let third = h.app.debug_card_rect(2).expect("three cards");
    // drag the first card's head to just past the third card
    h.drag(first.center_top() + vec2(0.0, 14.0), third.right_center() + vec2(10.0, 0.0));
    let order: Vec<String> = h.app.doc().scenes().iter().map(|s| s.heading.clone()).collect();
    assert_eq!(order, vec!["EXT. TWO - NIGHT", "INT. THREE - DAWN", "INT. ONE - DAY"]);
    assert_eq!(h.app.doc().blocks.last().unwrap().text, "One.");
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc().scenes()[0].heading, "INT. ONE - DAY", "and Ctrl+Z puts it back");
}

#[test]
fn clicking_a_card_opens_its_scene_to_write() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let second = h.app.debug_card_rect(1).unwrap();
    h.click(second.center_top() + vec2(0.0, 14.0));
    assert_eq!(h.app.mode(), Mode::Write);
    h.frames(3);
    let heading = h.app.doc().scenes()[1].id;
    assert_eq!(h.app.focus_block(), Some(heading));
}

#[test]
fn a_synopsis_typed_on_a_card_is_saved_with_the_scene() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(3);
    let card = h.app.debug_card_rect(0).unwrap();
    h.click(card.center());
    h.type_text("Cole gets the call.");
    h.idle();
    assert_eq!(h.app.doc().scenes()[0].synopsis, "Cole gets the call.");
    assert!(h.saved_text().contains("<!-- scene: Cole gets the call. -->"));
}

#[test]
fn pages_draw_the_title_page_and_every_printed_page() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    for _ in 0..70 {
        d.push(Element::Action, "Line after line after line of action on the page.");
    }
    d.reseed_ids();
    h.load(d);
    h.app.set_mode(Mode::Read);
    h.frames(3);
    assert_eq!(h.app.debug_sheet_count(), 1 + crate::export::page_count(h.app.doc()));
}

#[test]
fn ctrl_g_steps_through_the_three_views() {
    let mut h = Harness::new();
    assert_eq!(h.app.mode(), Mode::Write);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Cards);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Read);
    h.press(Key::G, Modifiers::COMMAND);
    assert_eq!(h.app.mode(), Mode::Write);
}

// ---------- find, snapshots, import ----------

#[test]
fn ctrl_f_opens_find_and_escape_closes_it() {
    let mut h = Harness::new();
    h.press(Key::F, Modifiers::COMMAND);
    assert!(h.app.debug_find_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_find_open());
}

#[test]
fn deleting_a_scene_from_its_card_asks_and_can_be_undone() {
    let mut h = Harness::new();
    h.load(three_scenes());
    h.app.set_mode(Mode::Cards);
    h.frames(2);
    h.app.debug_ask_delete_scene(1);
    h.frames(2);
    assert_eq!(h.app.doc().scenes().len(), 3, "nothing goes before the answer");
    h.confirm();
    let left: Vec<String> = h.app.doc().scenes().iter().map(|s| s.heading.clone()).collect();
    assert_eq!(left, vec!["INT. ONE - DAY", "INT. THREE - DAWN"]);
    assert!(!h.app.doc().blocks.iter().any(|b| b.text == "Two."), "its action went with it");
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.doc().scenes().len(), 3);
    assert_eq!(h.app.doc().scenes()[1].heading, "EXT. TWO - NIGHT");
}

#[test]
fn a_snapshot_can_be_restored_and_nothing_is_lost() {
    let mut h = Harness::new();
    h.load(three_scenes());
    let path = h.app.path().unwrap();
    h.app.debug_snapshot();
    h.app.doc_mut().blocks[1].text = "SOMEONE ELSE".into();
    h.frames(2);
    let snaps = storage::list_snapshots(&path);
    assert_eq!(snaps.len(), 1);
    h.app.debug_ask_restore(&snaps[0]);
    h.frames(2);
    assert_eq!(h.app.doc().blocks[1].text, "SOMEONE ELSE", "restoring asks first");
    h.confirm();
    assert_eq!(h.app.doc().blocks[1].text, "DETECTIVE COLE");
    assert_eq!(h.app.doc().meta.title, "Three Scenes", "the title is kept");
    let after = storage::list_snapshots(&path);
    assert_eq!(after.len(), 2, "what was there before the restore was kept too");
    assert!(std::fs::read_to_string(&after[0].path).unwrap().contains("SOMEONE ELSE")
        || std::fs::read_to_string(&after[1].path).unwrap().contains("SOMEONE ELSE"));
    assert!(h.saved_text().contains("**DETECTIVE COLE**"));
}

#[test]
fn a_fountain_file_is_imported_as_a_new_script() {
    let mut h = Harness::new();
    let f = std::env::temp_dir().join(format!("ns-ui-import-{}.fountain", std::process::id()));
    std::fs::write(
        &f,
        "Title: Bank Job\n\nEXT. BANK - DAY\n\nNobody moves.\n\nROSA\nNow.\n\nINT. VAULT - LATER\n\nEmpty.\n",
    )
    .unwrap();
    h.app.debug_import(&f);
    h.frames(2);
    assert_eq!(h.app.entry_count(), 2);
    assert_eq!(h.app.doc().meta.title, "Bank Job");
    assert_eq!(h.app.doc().scenes().len(), 2);
    assert!(h.app.path().unwrap().ends_with("bank-job.md"));
}

// ---------- the look ----------

#[test]
fn the_settings_panel_opens_from_the_ribbon() {
    let mut h = Harness::new();
    let r = h.app.debug_settings_button();
    h.click(r.center());
    assert!(h.app.debug_settings_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_settings_open());
}

#[test]
fn northstar_follows_tesseract_s_theme_live() {
    let mut h = Harness::with(1320.0, 900.0, "match_tesseract = yes\n");
    let tess = storage::tesseract_settings_path();
    std::fs::create_dir_all(tess.parent().unwrap()).unwrap();
    std::fs::write(&tess, "theme = void\nlight_mode = yes\n").unwrap();
    // it looks again every second and a half
    std::thread::sleep(Duration::from_millis(1600));
    h.frames(2);
    assert_eq!(theme::pal().id, ThemeId::Void);
    assert!(!theme::pal().dark);
    // Tesseract changes its mind; Northstar follows
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&tess, "theme = ember\nlight_mode = no\n").unwrap();
    std::thread::sleep(Duration::from_millis(1600));
    h.frames(2);
    assert_eq!(theme::pal().id, ThemeId::Ember);
    assert!(theme::pal().dark);
}

#[test]
fn every_theme_paints_every_view_without_panicking() {
    let mut h = Harness::new();
    h.load(three_scenes());
    for t in ThemeId::ALL {
        for light in [false, true] {
            h.app.debug_set_theme(t, light);
            for m in [Mode::Write, Mode::Cards, Mode::Read] {
                h.app.set_mode(m);
                h.frames(2);
            }
            assert_eq!(theme::pal().id, t);
        }
    }
}

#[test]
fn every_view_holds_at_the_minimum_window_size() {
    let mut h = Harness::with(880.0, 580.0, "show_cast = yes\n");
    h.load(three_scenes());
    for m in [Mode::Write, Mode::Cards, Mode::Read] {
        h.app.set_mode(m);
        h.frames(3);
        let page = h.app.debug_page_rect();
        assert!(page.width() > 200.0, "{m:?}: the page keeps some room ({page:?})");
    }
    h.app.debug_focus_mode(true);
    h.app.set_mode(Mode::Write);
    h.frames(3);
    assert!(h.app.debug_page_rect().width() > 800.0, "focus mode gives the page the window");
}

#[test]
fn the_session_counts_the_words_written() {
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.type_text(" one two three");
    std::thread::sleep(Duration::from_millis(400));
    h.frames(2);
    assert_eq!(h.app.debug_session_words(), 3);
}

#[test]
fn nothing_is_drawn_outside_the_block_being_typed_into() {
    // the block the caret is in is where the editor thinks it is
    let mut h = Harness::new();
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    let r = h.block_rect(id).expect("drawn");
    assert!(h.app.debug_page_rect().contains_rect(r.shrink(1.0)), "{r:?}");
}

#[test]
fn the_library_foot_ends_level_with_the_other_islands() {
    let mut h = Harness::with(1320.0, 900.0, "show_details = yes\n");
    let foot = h.app.debug_foot_rect();
    let page = h.app.debug_page_rect();
    // the library island has a 14px inner margin under its last line; its
    // outer edge must be exactly where the page island's is
    assert!(
        (foot.bottom() + 14.0 - page.bottom()).abs() < 1.5,
        "foot ends at {}, page island at {}",
        foot.bottom(),
        page.bottom()
    );
    let aside = h.app.debug_aside_rect();
    assert!((aside.bottom() - page.bottom()).abs() < 1.5);
    h.app.debug_set_theme(ThemeId::JetBrains, false);
    h.frames(2);
    assert_eq!(theme::pal().id, ThemeId::JetBrains);
}

#[test]
fn ctrl_i_shows_the_details_and_ctrl_shift_i_the_scenes() {
    let mut h = Harness::new();
    assert!(!h.app.settings().show_details);
    let scenes = h.app.settings().show_scenes;
    h.press(Key::I, Modifiers::COMMAND);
    assert!(h.app.settings().show_details);
    assert_eq!(h.app.settings().show_scenes, scenes, "only the details moved");
    h.press(Key::I, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(h.app.settings().show_scenes, !scenes);
    assert!(h.app.settings().show_details, "and only the scenes moved");
    assert!(storage::read_settings().show_details, "remembered");
}

#[test]
fn every_colour_option_paints_every_view() {
    let mut h = Harness::with(880.0, 580.0, "show_details = yes\nshow_cast = yes\n");
    h.load(three_scenes());
    for (elements, voices, reading) in [(true, false, false), (false, true, false), (true, true, true)] {
        {
            let s = h.app.settings_mut();
            s.element_colors = elements;
            s.character_colors = voices;
            s.character_colors_reading = reading;
        }
        for m in [Mode::Write, Mode::Cards, Mode::Read] {
            h.app.set_mode(m);
            h.frames(2);
        }
    }
    // at the smallest window, with every panel open, nothing runs off the
    // bottom of the right-hand column
    let aside = h.app.debug_aside_rect();
    assert!(aside.bottom() <= 580.0 - 14.0 + 1.0, "{aside:?}");
}

#[test]
fn no_island_on_the_right_ever_runs_past_the_page() {
    // one speaker, as in a narrated script — the case that used to overflow
    let mut one = Document::default();
    one.blocks.clear();
    one.meta.title = "Narrated".into();
    for i in 0..30 {
        one.push(Element::SceneHeading, &format!("INT. ROOM {i} - DAY"));
        one.push(Element::Character, "NARRATOR");
        one.push(Element::Dialogue, "And so it went.");
    }
    one.reseed_ids();
    for (w, h) in [(880.0, 580.0), (1100.0, 700.0), (1320.0, 900.0)] {
        for (details, scenes, cast) in [
            (false, false, true),
            (false, true, false),
            (true, false, false),
            (false, true, true),
            (true, true, true),
            (true, false, true),
        ] {
            let yes = |b: bool| if b { "yes" } else { "no" };
            let mut hh = Harness::with(
                w,
                h,
                &format!(
                    "show_details = {}\nshow_scenes = {}\nshow_cast = {}\n",
                    yes(details),
                    yes(scenes),
                    yes(cast)
                ),
            );
            hh.load(one.clone());
            hh.frames(3);
            let page = hh.app.debug_page_rect();
            let islands = hh.app.debug_aside_islands();
            assert!(!islands.is_empty());
            for r in islands {
                assert!(
                    r.bottom() <= page.bottom() + 0.5,
                    "{w}x{h} details={details} scenes={scenes} cast={cast}: island ends at {} but the page at {}",
                    r.bottom(),
                    page.bottom()
                );
            }
        }
    }
}

#[test]
fn reading_mode_shows_one_page_at_a_time_and_turns() {
    let mut h = Harness::new();
    let mut d = three_scenes();
    for _ in 0..160 {
        d.push(Element::Action, "Line after line after line of action on the page.");
    }
    d.push(Element::Action, "THE LAST LINE.");
    d.reseed_ids();
    h.load(d);
    let body = crate::export::page_count(h.app.doc());
    assert!(body >= 3);
    // written near the end: Reading mode opens on that page, not the title
    let last = h.app.doc().blocks.last().unwrap().id;
    h.focus(last);
    h.app.set_mode(Mode::Read);
    h.frames(3);
    assert_eq!(h.app.debug_sheet_count(), 1 + body);
    assert_eq!(h.app.debug_sheet_on_show(), body, "opens where the writing was");
    // the whole page is in view, with room for the control under it
    let sheet = h.app.debug_sheet_rect();
    let page = h.app.debug_page_rect();
    assert!(page.contains_rect(sheet), "{sheet:?} inside {page:?}");
    assert!((sheet.height() / sheet.width() - 11.0 / 8.5).abs() < 0.01, "US letter");
    h.press(Key::Home, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), 0, "Home is the title page");
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.press(Key::PageDown, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), 2);
    h.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), 1);
    h.press(Key::End, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), body);
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.app.debug_sheet_on_show(), body, "no page past the last");
    // the wheel turns pages too
    h.press(Key::Home, Modifiers::NONE);
    let at = page.center();
    for _ in 0..3 {
        h.frame_with(vec![
            Event::PointerMoved(at),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -40.0),
                modifiers: Modifiers::NONE,
            },
        ]);
    }
    h.frames(2);
    assert_eq!(h.app.debug_sheet_on_show(), 1);
}

#[test]
fn settings_is_a_window_that_closes_by_esc_button_or_outside() {
    let mut h = Harness::new();
    let button = h.app.debug_settings_button();
    h.click(button.center());
    assert!(h.app.debug_settings_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_settings_open(), "Esc closes it");
    h.click(button.center());
    assert!(h.app.debug_settings_open());
    // a click on the dimmed app outside the window closes it
    h.click(pos2(20.0, 880.0));
    assert!(!h.app.debug_settings_open(), "a click outside closes it");
    // categories: the second row is The page
    h.click(button.center());
    let win_left = (1320.0 - 780.0) / 2.0;
    h.click(pos2(win_left + 60.0, (900.0 - 620.0) / 2.0 + 56.0 + 17.0 + 36.0));
    assert!(h.app.debug_settings_open(), "a click inside keeps it open");
    assert_eq!(h.app.debug_settings_tab(), 1);
}

#[test]
fn the_export_window_exports_the_pages_asked_for() {
    use crate::app::PageChoice;
    let mut h = Harness::with(1320.0, 900.0, "after_export = nothing\n");
    let mut d = three_scenes();
    for _ in 0..140 {
        d.push(Element::Action, "Line after line after line of action on the page.");
    }
    d.reseed_ids();
    h.load(d);
    let id = h.app.doc().blocks[1].id;
    h.focus(id);
    h.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.app.debug_export_open());
    assert_eq!(h.app.focus_block(), Some(id));
    // Enter answers the window, not the script behind it
    let blocks = h.app.doc().blocks.len();
    h.app.debug_export_choose(PageChoice::Chosen, "2-3");
    h.frames(2);
    h.press(Key::Enter, Modifiers::NONE);
    assert_eq!(h.app.doc().blocks.len(), blocks, "no line was split behind the window");
    assert!(!h.app.debug_export_open(), "exporting closes it");
    let out = storage::exports_dir().join("three-scenes-pages-2-3.pdf");
    assert!(out.exists(), "{out:?}");

    // a selection that will not do keeps the window open and exports nothing
    h.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    h.app.debug_export_choose(PageChoice::Chosen, "40-2");
    h.frames(2);
    h.press(Key::Enter, Modifiers::NONE);
    assert!(h.app.debug_export_open());
    h.press(Key::Escape, Modifiers::NONE);
    assert!(!h.app.debug_export_open());

    // this page: the one the caret is on
    h.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    h.app.debug_export_choose(PageChoice::This, "");
    h.frames(2);
    h.press(Key::Enter, Modifiers::NONE);
    assert!(storage::exports_dir().join("three-scenes-pages-1.pdf").exists());
    // and the choices on the page are remembered
    assert!(storage::read_settings().pdf_title_page);
}
