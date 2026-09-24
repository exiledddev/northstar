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
    let path = std::path::PathBuf::from("/tmp/ns-test.pdf");
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
