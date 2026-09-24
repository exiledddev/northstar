//! Where scripts live on disk, and how they are written.
//!
//! Library root (created on first run):
//!
//! ```text
//! ~/.local/share/northstar/
//!     scripts/       <- one .md per screenplay
//!     exports/       <- pdf / fdx / fountain / txt output
//!     snapshots/     <- dated copies of a script, one folder per script
//!     settings.conf  <- what you have chosen, key = value
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
//! A scene's index card — its synopsis and colour — rides directly under its
//! heading as an HTML comment, which every markdown viewer hides:
//!
//! ```text
//! ## INT. WAREHOUSE - NIGHT
//! <!-- scene: tint=2 | Maria finds the door that isn't locked. -->
//! ```
//!
//! Blocks are separated by a blank line and never contain a newline, which is
//! what makes parsing a single pass with no ambiguity. Files written by the
//! first Northstar open unchanged; files written by this one open in it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::export;
use crate::model::{Block, Document, Element, Meta};
use crate::settings::Settings;

fn data_home() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/share")
    })
}

pub fn library_root() -> PathBuf {
    data_home().join("northstar")
}

pub fn scripts_dir() -> PathBuf {
    library_root().join("scripts")
}

pub fn exports_dir() -> PathBuf {
    library_root().join("exports")
}

pub fn snapshots_dir() -> PathBuf {
    library_root().join("snapshots")
}

pub fn settings_path() -> PathBuf {
    library_root().join("settings.conf")
}

/// Tesseract's own settings, for following its theme.
pub fn tesseract_settings_path() -> PathBuf {
    data_home().join("tesseract").join("settings.conf")
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
    pub starred: bool,
    pub pages: usize,
    pub scenes: usize,
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
            .find(|l| !l.is_empty() && !l.starts_with("<!--"))
            .unwrap_or("")
            .trim_start_matches(['#', '*', '>', '`', ' '])
            .chars()
            .take(70)
            .collect::<String>();
        let doc = from_markdown(&text);
        out.push(Entry {
            path,
            title,
            modified,
            preview,
            starred: meta.starred,
            pages: export::page_count(&doc),
            scenes: doc.scene_count(),
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
    if doc.meta.starred {
        s.push_str("starred: yes\n");
    }
    s.push_str("---\n\n");

    for b in &doc.blocks {
        let text = b.text.trim();
        if text.is_empty() {
            // an empty heading still carries its card, if it has one
            if !(b.element == Element::SceneHeading && has_card(b)) {
                continue;
            }
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
        s.push_str(line.trim_end());
        if b.element == Element::SceneHeading && has_card(b) {
            s.push('\n');
            s.push_str(&card_comment(b));
        }
        s.push_str("\n\n");
    }
    s
}

fn has_card(b: &Block) -> bool {
    !b.note.trim().is_empty() || b.tint.is_some()
}

fn card_comment(b: &Block) -> String {
    // "-->" would close the comment early; nothing else needs escaping
    let note = b.note.replace('\n', " ").replace("-->", "- ->");
    let note = note.trim();
    match b.tint {
        Some(t) => format!("<!-- scene: tint={t} | {note} -->"),
        None => format!("<!-- scene: {note} -->"),
    }
}

/// Read a card comment back: `(tint, synopsis)`, or `None` if the line is some
/// other comment.
fn parse_card(line: &str) -> Option<(Option<usize>, String)> {
    let inner = line.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    let rest = inner.strip_prefix("scene:")?.trim();
    if let Some(after) = rest.strip_prefix("tint=") {
        let (num, note) = match after.split_once('|') {
            Some((n, note)) => (n.trim(), note.trim()),
            None => (after.trim(), ""),
        };
        Some((num.parse::<usize>().ok(), note.to_string()))
    } else {
        Some((None, rest.to_string()))
    }
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
                "starred" => meta.starred = matches!(v.as_str(), "yes" | "true" | "1"),
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
        doc.blocks.push(Block::new(id, element, &text));
    };

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((tint, note)) = parse_card(line) {
            // a card belongs to the heading right above it; anywhere else it
            // has nothing to describe and is dropped
            if let Some(last) = doc.blocks.last_mut() {
                if last.element == Element::SceneHeading {
                    last.tint = tint;
                    last.note = note;
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            push(&mut doc, Element::Shot, rest.trim().to_uppercase());
        } else if let Some(rest) = line.strip_prefix("## ") {
            push(&mut doc, Element::SceneHeading, rest.trim().to_uppercase());
        } else if line == "##" {
            push(&mut doc, Element::SceneHeading, String::new());
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

// ---------- snapshots ----------

/// The folder a script's snapshots are kept in, named after the file.
pub fn snapshot_folder(script: &Path) -> PathBuf {
    let stem = script
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    snapshots_dir().join(stem)
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    /// When it was taken, as it reads in the file name: 2026-09-24 14:05:09.
    pub when: String,
    pub pages: usize,
}

/// Put a dated copy of the script aside.
pub fn take_snapshot(script: &Path, doc: &Document) -> io::Result<PathBuf> {
    let dir = snapshot_folder(script);
    fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let mut path = dir.join(format!("{stamp}.md"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stamp}-{n}.md"));
        n += 1;
    }
    fs::write(&path, to_markdown(doc))?;
    Ok(path)
}

/// A script's snapshots, newest first.
pub fn list_snapshots(script: &Path) -> Vec<Snapshot> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(snapshot_folder(script)) else {
        return out;
    };
    for e in rd.flatten() {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let when = match stem.split_once('_') {
            Some((d, t)) => format!("{d} {}", t.replace('-', ":")),
            None => stem.clone(),
        };
        let pages = load(&path).map(|d| export::page_count(&d)).unwrap_or(0);
        out.push(Snapshot { path, when, pages });
    }
    out.sort_by(|a, b| b.path.cmp(&a.path));
    out
}

/// When a script's file is renamed, its snapshots follow it.
pub fn follow_rename(from: &Path, to: &Path) {
    let old = snapshot_folder(from);
    if old.is_dir() {
        let new = snapshot_folder(to);
        if !new.exists() {
            let _ = fs::rename(old, new);
        }
    }
}

// ---------- settings ----------

pub fn read_settings() -> Settings {
    fs::read_to_string(settings_path())
        .map(|t| Settings::parse(&t))
        .unwrap_or_default()
}

pub fn write_settings(s: &Settings) -> io::Result<()> {
    fs::create_dir_all(library_root())?;
    fs::write(settings_path(), s.serialize())
}

/// Tesseract's settings, if Tesseract is installed and has been run. Read
/// with the same parser — the keys the two apps share are spelled the same.
pub fn read_tesseract_settings() -> Option<(Settings, SystemTime)> {
    let path = tesseract_settings_path();
    let when = fs::metadata(&path).and_then(|m| m.modified()).ok()?;
    let text = fs::read_to_string(&path).ok()?;
    // Tesseract's own default theme is Zen; a file that never names one means it
    Some((Settings::parse(&format!("theme = zen\n{text}")), when))
}

// ---------- the desktop ----------

/// Hand a path to the desktop (Dolphin, Nautilus, default PDF viewer, ...).
pub fn open_with_desktop(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

/// Open the file manager with `path` selected, where the desktop speaks the
/// freedesktop FileManager1 interface. KDE's Dolphin, Nautilus and Nemo all do.
/// Falls back to simply opening the containing folder.
pub fn reveal_in_file_manager(path: &Path) {
    let uri = format!("file://{}", path.display());
    let spoke = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{uri}"))
        .arg("string:")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !spoke {
        let dir = path.parent().unwrap_or(path);
        open_with_desktop(dir);
    }
}

/// Ask the desktop for a file to import. KDE's own dialog where there is one,
/// GNOME's otherwise. Blocks, so call it off the UI thread. `None` if the
/// dialog was cancelled or neither tool is installed.
pub fn pick_file_to_import() -> Result<Option<PathBuf>, String> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let tries: [(&str, Vec<String>); 2] = [
        (
            "kdialog",
            vec![
                "--title".into(),
                "Import a screenplay".into(),
                "--getopenfilename".into(),
                home.display().to_string(),
                "Screenplays (*.fountain *.spmd *.fdx *.md *.txt)".into(),
            ],
        ),
        (
            "zenity",
            vec![
                "--file-selection".into(),
                "--title=Import a screenplay".into(),
                "--file-filter=Screenplays | *.fountain *.spmd *.fdx *.md *.txt".into(),
            ],
        ),
    ];
    for (tool, args) in tries {
        match std::process::Command::new(tool).args(&args).output() {
            Ok(out) => {
                let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return Ok(if out.status.success() && !picked.is_empty() {
                    Some(PathBuf::from(picked))
                } else {
                    None
                });
            }
            Err(_) => continue,
        }
    }
    Err("Neither kdialog nor zenity is installed — drop the file onto the window instead.".into())
}

/// Read a screenplay from another format into a document. Fountain, Final
/// Draft and Northstar's own markdown are understood; anything else is read
/// as Fountain, which degrades to plain action.
pub fn import_file(path: &Path) -> Result<Document, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut doc = match ext.as_str() {
        "md" | "markdown" => from_markdown(&text),
        "fdx" => crate::fountain::parse_fdx(&text),
        _ => crate::fountain::parse(&text),
    };
    if doc.meta.title.trim().is_empty() {
        doc.meta.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Imported Script")
            .replace(['-', '_'], " ");
    }
    doc.normalize();
    Ok(doc)
}
