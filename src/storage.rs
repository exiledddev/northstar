//! Where scripts live on disk, and how they are written.
//!
//! Library root (created on first run):
//!
//! ```text
//! ~/.local/share/northstar/
//!     scripts/     <- one .md per screenplay
//!     exports/     <- pdf / txt / fountain output
//! ```
//!
//! # The file format
//!
//! Plain CommonMark with YAML front matter. Every screenplay element maps to
//! exactly one markdown construct, so the round trip is lossless *and* the
//! file still reads well in any markdown viewer:
//!
//! | Element        | Markdown            |
//! |----------------|---------------------|
//! | Scene heading  | `## INT. BAR - DAY` |
//! | Shot           | `### ANGLE ON DOOR` |
//! | Action         | plain paragraph     |
//! | Character      | `**MARIA (V.O.)**`  |
//! | Parenthetical  | `*(quietly)*`       |
//! | Dialogue       | `> Don't move.`     |
//! | Transition     | `` `CUT TO:` ``     |
//!
//! Blocks are separated by a blank line and never contain a newline, which is
//! what makes parsing a single pass with no ambiguity.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::model::{Block, Document, Element, Meta};

pub fn library_root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local/share")
        })
        .join("northstar")
}

pub fn scripts_dir() -> PathBuf {
    library_root().join("scripts")
}

pub fn exports_dir() -> PathBuf {
    library_root().join("exports")
}

pub fn ensure_dirs() -> io::Result<()> {
    fs::create_dir_all(scripts_dir())?;
    fs::create_dir_all(exports_dir())?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub title: String,
    pub modified: SystemTime,
    pub preview: String,
}

pub fn list_scripts() -> Vec<Entry> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(scripts_dir()) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let text = fs::read_to_string(&path).unwrap_or_default();
        let (meta, body) = split_front_matter(&text);
        let title = if meta.title.trim().is_empty() {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled")
                .to_string()
        } else {
            meta.title.clone()
        };
        let preview = body
            .lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .trim_start_matches(['#', '*', '>', '`', ' '])
            .chars()
            .take(70)
            .collect::<String>();
        out.push(Entry {
            path,
            title,
            modified,
            preview,
        });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

pub fn slugify(title: &str) -> String {
    let mut s = String::new();
    let mut prev_dash = false;
    for ch in title.trim().chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                s.push(lower);
            }
            prev_dash = false;
        } else if !prev_dash && !s.is_empty() {
            s.push('-');
            prev_dash = true;
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "untitled".to_string()
    } else {
        s
    }
}

/// A free path for `title`, adding -2, -3 ... if needed.
pub fn unique_path(title: &str, ignore: Option<&Path>) -> PathBuf {
    let base = slugify(title);
    let dir = scripts_dir();
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{base}.md")
        } else {
            format!("{base}-{n}.md")
        };
        let candidate = dir.join(name);
        let taken = candidate.exists() && Some(candidate.as_path()) != ignore;
        if !taken {
            return candidate;
        }
        n += 1;
    }
}

// ---------- serialisation ----------

pub fn to_markdown(doc: &Document) -> String {
    let mut s = String::new();
    s.push_str("---\n");
    s.push_str(&format!("title: {}\n", esc(&doc.meta.title)));
    s.push_str(&format!("author: {}\n", esc(&doc.meta.author)));
    s.push_str(&format!("contact: {}\n", esc(&doc.meta.contact)));
    s.push_str(&format!("draft: {}\n", esc(&doc.meta.draft)));
    s.push_str("---\n\n");

    for b in &doc.blocks {
        let text = b.text.trim();
        if text.is_empty() {
            continue;
        }
        let line = match b.element {
            Element::SceneHeading => format!("## {}", text),
            Element::Shot => format!("### {}", text),
            Element::Action => text.to_string(),
            Element::Character => format!("**{}**", text),
            Element::Parenthetical => {
                let inner = text.trim_matches(|c| c == '(' || c == ')').trim();
                format!("*({})*", inner)
            }
            Element::Dialogue => format!("> {}", text),
            Element::Transition => format!("`{}`", text),
        };
        s.push_str(&line);
        s.push_str("\n\n");
    }
    s
}

fn esc(s: &str) -> String {
    s.replace('\n', " ").trim().to_string()
}

fn split_front_matter(text: &str) -> (Meta, &str) {
    let mut meta = Meta::default();
    let rest = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return (meta, text);
    };
    // find the closing fence
    let mut idx = 0usize;
    let mut body_start = None;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end();
        idx += line.len();
        if trimmed == "---" {
            body_start = Some(idx);
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let v = v.trim().to_string();
            match k.trim() {
                "title" => meta.title = v,
                "author" => meta.author = v,
                "contact" => meta.contact = v,
                "draft" => meta.draft = v,
                _ => {}
            }
        }
    }
    match body_start {
        Some(i) => (meta, &rest[i..]),
        None => (meta, text),
    }
}

pub fn from_markdown(text: &str) -> Document {
    let (meta, body) = split_front_matter(text);
    let mut doc = Document::from_parts(meta, Vec::new());
    doc.blocks.clear();

    let mut id = 0u64;
    let mut push = |doc: &mut Document, element: Element, text: String| {
        id += 1;
        doc.blocks.push(Block { id, element, text });
    };

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            push(&mut doc, Element::Shot, rest.trim().to_uppercase());
        } else if let Some(rest) = line.strip_prefix("## ") {
            push(&mut doc, Element::SceneHeading, rest.trim().to_uppercase());
        } else if let Some(rest) = line.strip_prefix("# ") {
            // a stray H1: treat as a scene heading rather than losing it
            push(&mut doc, Element::SceneHeading, rest.trim().to_uppercase());
        } else if let Some(rest) = line.strip_prefix("> ") {
            push(&mut doc, Element::Dialogue, rest.trim().to_string());
        } else if line == ">" {
            push(&mut doc, Element::Dialogue, String::new());
        } else if line.len() >= 4 && line.starts_with("**") && line.ends_with("**") {
            let inner = line[2..line.len() - 2].trim();
            push(&mut doc, Element::Character, inner.to_uppercase());
        } else if line.len() >= 2
            && line.starts_with('`')
            && line.ends_with('`')
            && !line[1..line.len() - 1].contains('`')
        {
            let inner = line[1..line.len() - 1].trim();
            push(&mut doc, Element::Transition, inner.to_uppercase());
        } else if line.starts_with("*(") && line.ends_with(")*") {
            let inner = line[1..line.len() - 1].trim();
            push(&mut doc, Element::Parenthetical, inner.to_string());
        } else if line.starts_with('(') && line.ends_with(')') {
            push(&mut doc, Element::Parenthetical, line.to_string());
        } else {
            push(&mut doc, Element::Action, line.to_string());
        }
    }

    doc.ensure_not_empty();
    doc.reseed_ids();
    doc
}

pub fn save(path: &Path, doc: &Document) -> io::Result<()> {
    ensure_dirs()?;
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, to_markdown(doc))?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load(path: &Path) -> io::Result<Document> {
    let text = fs::read_to_string(path)?;
    Ok(from_markdown(&text))
}

pub fn delete(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

/// Hand a path to the desktop (Dolphin, Nautilus, default PDF viewer, ...).
pub fn open_with_desktop(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
