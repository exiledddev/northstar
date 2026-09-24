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
    /// A scene heading's synopsis — what the index card says. Empty for every
    /// other element.
    pub note: String,
    /// A scene heading's card colour, an index into the theme's six tints.
    pub tint: Option<usize>,
}

impl Block {
    pub fn new(id: u64, element: Element, text: &str) -> Block {
        Block {
            id,
            element,
            text: text.to_string(),
            note: String::new(),
            tint: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Meta {
    pub title: String,
    pub author: String,
    pub contact: String,
    pub draft: String,
    /// Gathered at the top of the library.
    pub starred: bool,
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
    Block::new(id, element, text)
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

// ---------------------------------------------------------------- scenes --

/// One scene: its heading block and every block up to the next heading.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// The heading block's id.
    pub id: u64,
    /// 1-based, in script order.
    pub number: usize,
    pub heading: String,
    pub synopsis: String,
    pub tint: Option<usize>,
    /// Block index range, heading included, end exclusive.
    pub start: usize,
    pub end: usize,
    /// Everyone who speaks in it, in order of first cue.
    pub cast: Vec<String>,
    pub words: usize,
}

/// What a character name is once its extensions are taken off:
/// `MARIA (V.O.)` and `MARIA (CONT'D)` are both MARIA.
pub fn base_character(name: &str) -> String {
    let t = name.trim().trim_end_matches('^').trim();
    let cut = t.find('(').unwrap_or(t.len());
    t[..cut].trim().to_uppercase()
}

#[derive(Clone, Debug, PartialEq)]
pub struct CastMember {
    pub name: String,
    /// How many times the character speaks.
    pub cues: usize,
    /// Words of dialogue.
    pub words: usize,
    /// How many scenes they speak in.
    pub scenes: usize,
    /// The id of every cue, in order, for jumping between them.
    pub cue_ids: Vec<u64>,
}

impl Document {
    /// Every scene in the script. Anything before the first heading is not a
    /// scene and is left out.
    pub fn scenes(&self) -> Vec<Scene> {
        let mut out: Vec<Scene> = Vec::new();
        for (i, b) in self.blocks.iter().enumerate() {
            if b.element == SceneHeading {
                if let Some(last) = out.last_mut() {
                    last.end = i;
                }
                out.push(Scene {
                    id: b.id,
                    number: out.len() + 1,
                    heading: b.text.clone(),
                    synopsis: b.note.clone(),
                    tint: b.tint,
                    start: i,
                    end: self.blocks.len(),
                    cast: Vec::new(),
                    words: 0,
                });
            }
        }
        for sc in &mut out {
            for b in &self.blocks[sc.start..sc.end] {
                sc.words += b.text.split_whitespace().count();
                if b.element == Character && !b.text.trim().is_empty() {
                    let name = base_character(&b.text);
                    if !name.is_empty() && !sc.cast.contains(&name) {
                        sc.cast.push(name);
                    }
                }
            }
        }
        out
    }

    /// Which scene the block `id` belongs to, as an index into `scenes()`.
    pub fn scene_of(&self, id: u64) -> Option<usize> {
        let at = self.index_of(id)?;
        let mut found = None;
        let mut n = 0usize;
        for (i, b) in self.blocks.iter().enumerate() {
            if i > at {
                break;
            }
            if b.element == SceneHeading {
                found = Some(n);
                n += 1;
            }
        }
        found
    }

    /// Everyone who speaks, most lines first.
    pub fn cast(&self) -> Vec<CastMember> {
        let mut out: Vec<CastMember> = Vec::new();
        let mut current: Option<usize> = None;
        let mut scene_no = 0usize;
        let mut seen_in: Vec<(usize, usize)> = Vec::new(); // (cast index, scene)
        for b in &self.blocks {
            match b.element {
                SceneHeading => {
                    scene_no += 1;
                    current = None;
                }
                Character => {
                    let name = base_character(&b.text);
                    if name.is_empty() {
                        current = None;
                        continue;
                    }
                    let k = match out.iter().position(|c| c.name == name) {
                        Some(k) => k,
                        None => {
                            out.push(CastMember {
                                name,
                                cues: 0,
                                words: 0,
                                scenes: 0,
                                cue_ids: Vec::new(),
                            });
                            out.len() - 1
                        }
                    };
                    out[k].cues += 1;
                    out[k].cue_ids.push(b.id);
                    if !seen_in.contains(&(k, scene_no)) {
                        seen_in.push((k, scene_no));
                        out[k].scenes += 1;
                    }
                    current = Some(k);
                }
                Dialogue => {
                    if let Some(k) = current {
                        out[k].words += b.text.split_whitespace().count();
                    }
                }
                Parenthetical => {}
                _ => current = None,
            }
        }
        out.sort_by(|a, b| b.cues.cmp(&a.cues).then(a.name.cmp(&b.name)));
        out
    }

    /// Move scene `from` so it lands before what is now scene `to` (or at the
    /// end when `to` is past the last). Returns true if anything moved.
    pub fn move_scene(&mut self, from: usize, to: usize) -> bool {
        let scenes = self.scenes();
        if from >= scenes.len() || to > scenes.len() || to == from || to == from + 1 {
            return false;
        }
        let src = &scenes[from];
        let chunk: Vec<Block> = self.blocks.drain(src.start..src.end).collect();
        let len = chunk.len();
        let insert_at = if to == scenes.len() {
            self.blocks.len()
        } else {
            let target = scenes[to].start;
            if target > src.start {
                target - len
            } else {
                target
            }
        };
        for (k, b) in chunk.into_iter().enumerate() {
            self.blocks.insert(insert_at + k, b);
        }
        true
    }

    /// Take a whole scene out: its heading and everything under it.
    pub fn remove_scene(&mut self, n: usize) -> bool {
        let scenes = self.scenes();
        let Some(sc) = scenes.get(n) else {
            return false;
        };
        self.blocks.drain(sc.start..sc.end);
        self.ensure_not_empty();
        true
    }

    /// Words of dialogue against words of everything else, for the balance
    /// readout.
    pub fn dialogue_share(&self) -> f32 {
        let (mut talk, mut all) = (0usize, 0usize);
        for b in &self.blocks {
            let w = b.text.split_whitespace().count();
            all += w;
            if b.element == Dialogue {
                talk += w;
            }
        }
        if all == 0 {
            0.0
        } else {
            talk as f32 / all as f32
        }
    }

    /// Every occurrence of `needle`, as (block id, char start), case folded.
    pub fn find(&self, needle: &str) -> Vec<(u64, usize)> {
        let needle = needle.to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let n_len = needle.chars().count();
        let mut out = Vec::new();
        for b in &self.blocks {
            let hay: Vec<char> = b.text.to_lowercase().chars().collect();
            let pat: Vec<char> = needle.chars().collect();
            if hay.len() < n_len {
                continue;
            }
            let mut i = 0;
            while i + n_len <= hay.len() {
                if hay[i..i + n_len] == pat[..] {
                    out.push((b.id, i));
                    i += n_len;
                } else {
                    i += 1;
                }
            }
        }
        out
    }

    /// Replace every occurrence of `needle` (case folded) with `with`,
    /// keeping each element's capitalisation rule. Returns how many.
    pub fn replace_all(&mut self, needle: &str, with: &str) -> usize {
        if needle.is_empty() {
            return 0;
        }
        let mut count = 0;
        for b in &mut self.blocks {
            let (text, n) = replace_folded(&b.text, needle, with);
            if n > 0 {
                count += n;
                b.text = if b.element.is_upper() {
                    text.to_uppercase()
                } else {
                    text
                };
            }
        }
        count
    }
}

/// Case-insensitive replace that leaves everything else in the string alone.
pub fn replace_folded(hay: &str, needle: &str, with: &str) -> (String, usize) {
    let h: Vec<char> = hay.chars().collect();
    let lower: Vec<char> = hay.to_lowercase().chars().collect();
    let pat: Vec<char> = needle.to_lowercase().chars().collect();
    // lowercasing can change the length of exotic characters; bail out safely
    if lower.len() != h.len() || pat.is_empty() {
        return (hay.to_string(), 0);
    }
    let mut out = String::new();
    let mut i = 0;
    let mut n = 0;
    while i < h.len() {
        if i + pat.len() <= h.len() && lower[i..i + pat.len()] == pat[..] {
            out.push_str(with);
            i += pat.len();
            n += 1;
        } else {
            out.push(h[i]);
            i += 1;
        }
    }
    (out, n)
}

// ------------------------------------------------------------- SmartType --

const SLUG_PREFIXES: [&str; 5] = ["INT. ", "EXT. ", "INT./EXT. ", "EXT./INT. ", "EST. "];
const TIMES: [&str; 11] = [
    "DAY",
    "NIGHT",
    "MORNING",
    "EVENING",
    "AFTERNOON",
    "DAWN",
    "DUSK",
    "CONTINUOUS",
    "LATER",
    "MOMENTS LATER",
    "SAME",
];
const TRANSITIONS: [&str; 10] = [
    "CUT TO:",
    "SMASH CUT TO:",
    "MATCH CUT TO:",
    "JUMP CUT TO:",
    "DISSOLVE TO:",
    "FADE IN:",
    "FADE OUT.",
    "FADE TO BLACK.",
    "INTERCUT WITH:",
    "BACK TO:",
];

/// What SmartType would add to `typed`, the text of block `own` so far — the
/// remainder only, never the part already there. Character cues come from
/// everyone who already speaks; scene headings from the prefixes, the places
/// already used and the usual times of day; transitions from the standard set.
pub fn complete(doc: &Document, own: u64, element: Element, typed: &str) -> Option<String> {
    let typed_up = typed.to_uppercase();
    if typed_up.trim().is_empty() {
        return None;
    }
    // (candidate, weight) — heavier wins, shorter breaks ties
    let mut cands: Vec<(String, usize)> = Vec::new();
    let mut add = |c: String, w: usize| {
        if let Some(x) = cands.iter_mut().find(|(s, _)| *s == c) {
            x.1 += w;
        } else {
            cands.push((c, w));
        }
    };
    match element {
        Character => {
            for b in &doc.blocks {
                if b.element == Character && b.id != own {
                    let n = base_character(&b.text);
                    if !n.is_empty() {
                        add(n, 1);
                    }
                }
            }
        }
        SceneHeading => {
            for p in SLUG_PREFIXES {
                add(p.to_string(), 1);
            }
            // until INT. or EXT. is written out, that is all that is offered
            if !SLUG_PREFIXES.iter().any(|p| typed_up.starts_with(p)) {
                cands.retain(|(c, _)| c.len() > typed_up.len() && c.starts_with(&typed_up));
                cands.sort_by(|a, b| a.0.len().cmp(&b.0.len()));
                let best = cands.into_iter().next()?.0;
                return Some(best[typed_up.len()..].to_string());
            }
            for b in &doc.blocks {
                if b.element != SceneHeading || b.id == own {
                    continue;
                }
                let h = b.text.trim().to_uppercase();
                if h.is_empty() {
                    continue;
                }
                // the place on its own, ready for a time of day
                if let Some(cut) = h.rfind(" - ") {
                    add(format!("{} - ", &h[..cut]), 2);
                }
                add(h, 1);
            }
            if let Some(cut) = typed_up.rfind(" - ") {
                let head = &typed_up[..cut];
                for t in TIMES {
                    add(format!("{head} - {t}"), 1);
                }
            }
        }
        Transition => {
            for t in TRANSITIONS {
                add(t.to_string(), 1);
            }
        }
        _ => return None,
    }
    cands.retain(|(c, _)| c.len() > typed_up.len() && c.starts_with(&typed_up));
    cands.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.len().cmp(&b.0.len())));
    let best = cands.into_iter().next()?.0;
    Some(best[typed_up.len()..].to_string())
}

/// Screen time for a length measured in eighths of a page — the unit a
/// schedule is drawn up in.
pub fn eighths_label(eighths: usize) -> String {
    let whole = eighths / 8;
    let part = eighths % 8;
    match (whole, part) {
        (0, 0) => "0".to_string(),
        (0, p) => format!("{p}/8"),
        (w, 0) => format!("{w}"),
        (w, p) => format!("{w} {p}/8"),
    }
}
