//! Core screenplay data model.
//!
//! Geometry follows standard US master-scene format: 8.5x11, 12pt Courier
//! (10 characters per inch), 1.5" left margin, 1" right margin, 1" top and
//! bottom. That leaves a 6.0" text column = 60 characters wide.
//!
//! All indents below are expressed in *character columns measured from the
//! left text margin* (i.e. from 1.5" on the physical page):
//!
//!   Scene heading   0   (1.5" on page)   ALL CAPS
//!   Action          0   (1.5" on page)
//!   Character      22   (3.7" on page)   ALL CAPS
//!   Parenthetical  16   (3.1" on page)
//!   Dialogue       10   (2.5" on page)   3.5" wide
//!   Transition     45   (6.0" on page)   ALL CAPS
//!   Shot            0   (1.5" on page)   ALL CAPS

/// Width of the printable text column, in characters.
pub const PAGE_COLS: usize = 60;
/// Lines of body text per printed page.
pub const LINES_PER_PAGE: usize = 55;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Element {
    SceneHeading,
    Action,
    Character,
    Parenthetical,
    Dialogue,
    Transition,
    Shot,
}

use Element::*;

impl Element {
    pub const ALL: [Element; 7] = [
        SceneHeading,
        Action,
        Character,
        Parenthetical,
        Dialogue,
        Transition,
        Shot,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SceneHeading => "Scene Heading",
            Action => "Action",
            Character => "Character",
            Parenthetical => "Parenthetical",
            Dialogue => "Dialogue",
            Transition => "Transition",
            Shot => "Shot",
        }
    }

    /// Compact label for the gutter tag.
    pub fn short(self) -> &'static str {
        match self {
            SceneHeading => "SCENE",
            Action => "ACTION",
            Character => "CHAR",
            Parenthetical => "PAREN",
            Dialogue => "DIALOG",
            Transition => "TRANS",
            Shot => "SHOT",
        }
    }

    /// Indent from the left text margin, in characters.
    pub fn indent_cols(self) -> usize {
        match self {
            SceneHeading | Action | Shot => 0,
            Character => 22,
            Parenthetical => 16,
            Dialogue => 10,
            Transition => 45,
        }
    }

    /// Wrapping width, in characters.
    pub fn width_cols(self) -> usize {
        match self {
            SceneHeading | Action | Shot => 60,
            Character => 38,
            Parenthetical => 28,
            Dialogue => 35,
            Transition => 15,
        }
    }

    pub fn is_upper(self) -> bool {
        matches!(self, SceneHeading | Character | Transition | Shot)
    }

    /// Blank lines emitted before this element when printing.
    pub fn blank_lines_before(self) -> usize {
        match self {
            SceneHeading => 2,
            Action | Character | Transition | Shot => 1,
            Parenthetical | Dialogue => 0,
        }
    }

    /// What pressing Enter at the end of this element should create next.
    pub fn on_enter(self) -> Element {
        match self {
            SceneHeading => Action,
            Action => Action,
            Shot => Action,
            Character => Dialogue,
            Parenthetical => Dialogue,
            Dialogue => Action,
            Transition => SceneHeading,
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|e| *e == self).unwrap_or(1)
    }

    /// Tab / Shift+Tab cycling through the element palette.
    pub fn cycle(self, forward: bool) -> Element {
        let n = Self::ALL.len();
        let i = self.index();
        let j = if forward { (i + 1) % n } else { (i + n - 1) % n };
        Self::ALL[j]
    }

    /// Ctrl+1 .. Ctrl+7
    pub fn from_digit(d: usize) -> Option<Element> {
        Self::ALL.get(d.checked_sub(1)?).copied()
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: u64,
    pub element: Element,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Meta {
    pub title: String,
    pub author: String,
    pub contact: String,
    pub draft: String,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub meta: Meta,
    pub blocks: Vec<Block>,
    next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        let mut doc = Document {
            meta: Meta {
                title: "Untitled Script".to_string(),
                ..Default::default()
            },
            blocks: Vec::new(),
            next_id: 1,
        };
        let first = doc_block(&mut doc.next_id, SceneHeading, "");
        doc.blocks.push(first);
        doc
    }
}

fn doc_block(next_id: &mut u64, element: Element, text: &str) -> Block {
    let id = *next_id;
    *next_id += 1;
    Block {
        id,
        element,
        text: text.to_string(),
    }
}

impl Document {
    pub fn from_parts(meta: Meta, blocks: Vec<Block>) -> Self {
        let mut doc = Document {
            meta,
            blocks,
            next_id: 1,
        };
        doc.ensure_not_empty();
        doc.reseed_ids();
        doc
    }

    pub fn new_block(&mut self, element: Element, text: &str) -> Block {
        doc_block(&mut self.next_id, element, text)
    }

    pub fn push(&mut self, element: Element, text: &str) {
        let b = self.new_block(element, text);
        self.blocks.push(b);
    }

    /// Rebuild the id counter after loading blocks from disk.
    pub fn reseed_ids(&mut self) {
        let max = self.blocks.iter().map(|b| b.id).max().unwrap_or(0);
        self.next_id = max + 1;
    }

    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.blocks.iter().position(|b| b.id == id)
    }

    pub fn block(&self, id: u64) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }

    pub fn ensure_not_empty(&mut self) {
        if self.blocks.is_empty() {
            let b = self.new_block(SceneHeading, "");
            self.blocks.push(b);
        }
    }

    /// Tidy-ups applied before saving or exporting: trim trailing spaces,
    /// force caps where the format demands it, wrap parentheticals.
    pub fn normalize(&mut self) {
        for b in &mut self.blocks {
            let t = b.text.trim().to_string();
            b.text = if b.element.is_upper() {
                t.to_uppercase()
            } else {
                t
            };
            if b.element == Parenthetical && !b.text.is_empty() {
                let inner = b.text.trim_matches(|c| c == '(' || c == ')').trim();
                b.text = format!("({})", inner);
            }
        }
    }

    pub fn word_count(&self) -> usize {
        self.blocks
            .iter()
            .map(|b| b.text.split_whitespace().count())
            .sum()
    }

    pub fn scene_count(&self) -> usize {
        self.blocks
            .iter()
            .filter(|b| b.element == SceneHeading && !b.text.trim().is_empty())
            .count()
    }

    /// Every scene heading, for the outline panel: (block id, text).
    pub fn outline(&self) -> Vec<(u64, String)> {
        self.blocks
            .iter()
            .filter(|b| b.element == SceneHeading)
            .map(|b| (b.id, b.text.clone()))
            .collect()
    }
}

/// Best guess at what a raw line of pasted text is, given what came before it.
/// Used when someone pastes a whole scene in from somewhere else.
pub fn guess_element(line: &str, prev: Option<Element>) -> Element {
    let t = line.trim();
    if t.is_empty() {
        return Action;
    }
    let upper = t.to_uppercase() == t;

    const SLUGS: [&str; 8] = [
        "INT.", "EXT.", "INT ", "EXT ", "I/E.", "I/E ", "EST.", "INT./EXT.",
    ];
    if SLUGS.iter().any(|p| t.to_uppercase().starts_with(p)) {
        return SceneHeading;
    }
    if t.starts_with('(') && t.ends_with(')') {
        return Parenthetical;
    }
    if upper && (t.ends_with("TO:") || t == "FADE IN:" || t == "FADE OUT.") {
        return Transition;
    }
    if upper && t.chars().count() <= 38 && !matches!(prev, Some(Character)) {
        return Character;
    }
    if matches!(prev, Some(Character) | Some(Parenthetical)) {
        return Dialogue;
    }
    Action
}

/// Greedy word wrap to `width` characters. Never splits a word shorter than
/// the column; over-long words are hard-broken so nothing runs off the page.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();

    for word in text.split_whitespace() {
        let wlen = word.chars().count();
        let llen = line.chars().count();

        if llen == 0 {
            if wlen <= width {
                line.push_str(word);
            } else {
                // hard-break a monster word
                let mut rest: Vec<char> = word.chars().collect();
                while rest.len() > width {
                    let chunk: String = rest.drain(..width).collect();
                    out.push(chunk);
                }
                line = rest.into_iter().collect();
            }
        } else if llen + 1 + wlen <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            if wlen <= width {
                line.push_str(word);
            } else {
                let mut rest: Vec<char> = word.chars().collect();
                while rest.len() > width {
                    let chunk: String = rest.drain(..width).collect();
                    out.push(chunk);
                }
                line = rest.into_iter().collect();
            }
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
