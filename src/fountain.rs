//! Reading scripts written elsewhere.
//!
//! Fountain is the plain-text screenplay format Highland, Beat, Slugline and
//! afterwriting all speak; Final Draft's `.fdx` is the industry's XML. Both are
//! read into the same typed blocks Northstar writes, so an imported script is
//! an ordinary script from the moment it lands.

use crate::model::{Document, Element, Meta};

/// Parse a Fountain document.
///
/// The rules followed are Fountain 1.1's, in its order of precedence: forced
/// elements (`.`, `@`, `>`, `!`) first, then scene headings by their prefix,
/// transitions by their `TO:`, and characters as an all-caps line after a
/// blank one with something straight under it. Notes, boneyard and section
/// markers are dropped; synopses (`= ...`) become the scene's card.
pub fn parse(text: &str) -> Document {
    let text = strip_between(&strip_between(text, "/*", "*/"), "[[", "]]");
    let lines: Vec<&str> = text.lines().collect();

    let (meta, body_from) = title_page(&lines);
    let mut doc = Document::from_parts(meta, Vec::new());
    doc.blocks.clear();

    let mut i = body_from;
    let mut prev_blank = true;
    // inside a character's speech: the lines after the cue until a blank one
    let mut speaking = false;

    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim();
        let next_blank = lines.get(i + 1).map(|l| l.trim().is_empty()).unwrap_or(true);
        i += 1;

        if line.is_empty() {
            prev_blank = true;
            speaking = false;
            continue;
        }

        if speaking {
            if line.starts_with('(') && line.ends_with(')') {
                doc.push(Element::Parenthetical, line);
            } else {
                // consecutive dialogue lines are one speech
                match doc.blocks.last_mut() {
                    Some(b) if b.element == Element::Dialogue => {
                        b.text.push(' ');
                        b.text.push_str(line);
                    }
                    _ => doc.push(Element::Dialogue, line),
                }
            }
            prev_blank = false;
            continue;
        }

        // synopsis: the card of the scene above
        if let Some(rest) = line.strip_prefix('=') {
            if !rest.starts_with("==") {
                let note = rest.trim().to_string();
                if let Some(h) = doc
                    .blocks
                    .iter_mut()
                    .rev()
                    .find(|b| b.element == Element::SceneHeading)
                {
                    if h.note.is_empty() {
                        h.note = note;
                    } else {
                        h.note.push(' ');
                        h.note.push_str(&note);
                    }
                }
            }
            prev_blank = false;
            continue;
        }
        // sections and page breaks carry no text of their own
        if line.starts_with('#') || line.chars().all(|c| c == '=') {
            prev_blank = false;
            continue;
        }

        // forced action
        if let Some(rest) = line.strip_prefix('!') {
            doc.push(Element::Action, rest.trim());
            prev_blank = false;
            continue;
        }
        // forced scene heading (but not an ellipsis)
        if line.starts_with('.') && !line.starts_with("..") && line.len() > 1 {
            doc.push(Element::SceneHeading, &clean_heading(&line[1..]));
            prev_blank = false;
            continue;
        }
        // centred text is action here; forced transition otherwise
        if let Some(rest) = line.strip_prefix('>') {
            let rest = rest.trim();
            if let Some(inner) = rest.strip_suffix('<') {
                doc.push(Element::Action, inner.trim());
            } else {
                doc.push(Element::Transition, &rest.to_uppercase());
            }
            prev_blank = false;
            continue;
        }
        // forced character
        if let Some(rest) = line.strip_prefix('@') {
            doc.push(Element::Character, &rest.trim().trim_end_matches('^').trim().to_uppercase());
            speaking = true;
            prev_blank = false;
            continue;
        }

        let upper = line.to_uppercase();
        if prev_blank && is_heading(&upper) {
            doc.push(Element::SceneHeading, &clean_heading(line));
        } else if prev_blank && next_blank && upper == line && line.ends_with("TO:") {
            doc.push(Element::Transition, line);
        } else if prev_blank
            && !next_blank
            && upper == line
            && line.chars().any(|c| c.is_alphabetic())
        {
            doc.push(Element::Character, line.trim_end_matches('^').trim());
            speaking = true;
        } else {
            let stripped = line.trim_start_matches('~').trim();
            doc.push(Element::Action, stripped);
        }
        prev_blank = false;
    }

    doc.ensure_not_empty();
    doc.reseed_ids();
    doc
}

fn is_heading(upper: &str) -> bool {
    const PREFIXES: [&str; 10] = [
        "INT.", "EXT.", "EST.", "INT./EXT.", "INT/EXT", "I/E.", "I/E ", "INT ", "EXT ", "EST ",
    ];
    PREFIXES.iter().any(|p| upper.starts_with(p))
}

/// Drop a trailing scene number (`#12A#`) and fold to capitals.
fn clean_heading(s: &str) -> String {
    let t = s.trim();
    let t = match t.strip_suffix('#').and_then(|head| head.rfind('#')) {
        Some(k) => t[..k].trim(),
        None => t,
    };
    t.to_uppercase()
}

/// Remove everything between each `open` and the next `close`, inclusive.
fn strip_between(s: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find(open) {
        out.push_str(&rest[..a]);
        match rest[a + open.len()..].find(close) {
            Some(b) => rest = &rest[a + open.len() + b + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The `Key: value` block at the head of a Fountain file, and where the
/// script proper starts.
fn title_page(lines: &[&str]) -> (Meta, usize) {
    let mut meta = Meta::default();
    let first = lines.iter().position(|l| !l.trim().is_empty());
    let Some(first) = first else {
        return (meta, 0);
    };
    let looks_like_key = |l: &str| {
        l.split_once(':')
            .map(|(k, _)| {
                let k = k.trim();
                !k.is_empty()
                    && !l.starts_with(' ')
                    && !l.starts_with('\t')
                    && k.chars().all(|c| c.is_alphabetic() || c == ' ')
                    && k.len() < 24
            })
            .unwrap_or(false)
    };
    if !looks_like_key(lines[first]) {
        return (meta, 0);
    }
    let mut key = String::new();
    let mut i = first;
    while i < lines.len() && !lines[i].trim().is_empty() {
        let l = lines[i];
        let (k, v) = if looks_like_key(l) {
            let (k, v) = l.split_once(':').unwrap_or((l, ""));
            key = k.trim().to_lowercase();
            (key.clone(), v.trim().to_string())
        } else {
            (key.clone(), l.trim().to_string())
        };
        let target = match k.as_str() {
            "title" => Some(&mut meta.title),
            "author" | "authors" => Some(&mut meta.author),
            "draft date" | "draft" | "date" => Some(&mut meta.draft),
            "contact" => Some(&mut meta.contact),
            _ => None,
        };
        if let Some(t) = target {
            if !v.is_empty() {
                if !t.is_empty() {
                    t.push_str(", ");
                }
                // title-page markup is emphasis, which the page does not carry
                t.push_str(v.trim_matches(|c| c == '*' || c == '_'));
            }
        }
        i += 1;
    }
    (meta, i)
}

// ---------------------------------------------------------- final draft --

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Every `<Text ...>...</Text>` in `s`, joined.
fn texts(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(a) = rest.find("<Text") {
        let after = &rest[a..];
        let Some(open_end) = after.find('>') else { break };
        if after[..open_end].ends_with('/') {
            rest = &after[open_end + 1..];
            continue;
        }
        let body = &after[open_end + 1..];
        let Some(close) = body.find("</Text>") else { break };
        out.push_str(&unescape(&body[..close]));
        rest = &body[close + 7..];
    }
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let a = tag.find(&key)? + key.len();
    let b = tag[a..].find('"')? + a;
    Some(tag[a..b].to_string())
}

/// Read a Final Draft file. Paragraph types map one to one; anything Final
/// Draft has that Northstar does not (General, Cast List...) becomes action.
pub fn parse_fdx(text: &str) -> Document {
    let mut doc = Document::from_parts(Meta::default(), Vec::new());
    doc.blocks.clear();

    let content = section(text, "<Content>", "</Content>").unwrap_or("");
    let mut rest = content;
    while let Some(a) = rest.find("<Paragraph") {
        let after = &rest[a..];
        let Some(open_end) = after.find('>') else { break };
        let tag = &after[..open_end];
        let body_start = &after[open_end + 1..];
        // the scene's summary sits in its own nested paragraphs; take it out
        // first so the paragraph's end is the right one
        let (summary, body_from) = match (body_start.find("<SceneProperties"), body_start.find("</Paragraph>")) {
            (Some(sp), Some(end)) if sp < end => match body_start.find("</SceneProperties>") {
                Some(spe) => (
                    Some(texts(&body_start[sp..spe])),
                    &body_start[spe + "</SceneProperties>".len()..],
                ),
                None => (None, body_start),
            },
            _ => (None, body_start),
        };
        let Some(end) = body_from.find("</Paragraph>") else { break };
        let words = texts(&body_from[..end]);
        rest = &body_from[end + "</Paragraph>".len()..];

        let kind = attr(tag, "Type").unwrap_or_default();
        let element = match kind.as_str() {
            "Scene Heading" => Element::SceneHeading,
            "Character" => Element::Character,
            "Parenthetical" => Element::Parenthetical,
            "Dialogue" => Element::Dialogue,
            "Transition" => Element::Transition,
            "Shot" => Element::Shot,
            _ => Element::Action,
        };
        let words = words.trim();
        if words.is_empty() && summary.is_none() {
            continue;
        }
        let text = if element.is_upper() {
            words.to_uppercase()
        } else {
            words.to_string()
        };
        doc.push(element, &text);
        if let (Some(sum), Some(last)) = (summary, doc.blocks.last_mut()) {
            if last.element == Element::SceneHeading {
                last.note = sum.trim().to_string();
            }
        }
    }

    // the title page: the first line is the title; whatever follows a
    // "written by" is the author
    if let Some(tp) = section(text, "<TitlePage>", "</TitlePage>") {
        let lines: Vec<String> = tp
            .split("</Paragraph>")
            .map(texts)
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if let Some(t) = lines.first() {
            doc.meta.title = title_case(t);
        }
        if let Some(k) = lines
            .iter()
            .position(|l| matches!(l.to_lowercase().as_str(), "written by" | "by"))
        {
            if let Some(a) = lines.get(k + 1) {
                doc.meta.author = a.clone();
            }
        }
    }

    doc.ensure_not_empty();
    doc.reseed_ids();
    doc
}

fn section<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)? + a;
    Some(&s[a..b])
}

/// A title set in capitals on a title page reads better as a title.
fn title_case(s: &str) -> String {
    if s.to_uppercase() != s {
        return s.to_string();
    }
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
