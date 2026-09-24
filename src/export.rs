//! Turning the document into pages.
//!
//! Everything downstream (page count, plain text, PDF) is built from one
//! composition pass so the page count you see in the status bar is the page
//! count you get in the PDF.

use std::io;
use std::path::{Path, PathBuf};

use crate::model::{wrap, Document, Element, LINES_PER_PAGE, PAGE_COLS};
use crate::storage;

#[derive(Clone, Debug)]
pub struct Line {
    /// Indent in characters from the left text margin.
    pub indent: usize,
    pub text: String,
    pub element: Option<Element>,
}

impl Line {
    fn blank() -> Self {
        Line {
            indent: 0,
            text: String::new(),
            element: None,
        }
    }
}

/// Flatten the document into wrapped, indented lines.
pub fn compose(doc: &Document) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();

    for block in &doc.blocks {
        let text = block.text.trim();
        if text.is_empty() {
            continue;
        }
        let e = block.element;

        if !out.is_empty() {
            for _ in 0..e.blank_lines_before() {
                out.push(Line::blank());
            }
        }

        let body = if e == Element::Parenthetical {
            let inner = text.trim_matches(|c| c == '(' || c == ')').trim();
            format!("({})", inner)
        } else if e.is_upper() {
            text.to_uppercase()
        } else {
            text.to_string()
        };

        for l in wrap(&body, e.width_cols()) {
            out.push(Line {
                indent: e.indent_cols(),
                text: l,
                element: Some(e),
            });
        }
    }

    out
}

/// Split composed lines into pages, keeping a character cue with its dialogue.
pub fn paginate(lines: &[Line]) -> Vec<Vec<Line>> {
    let mut pages: Vec<Vec<Line>> = Vec::new();
    let mut page: Vec<Line> = Vec::new();

    for line in lines {
        // never start a page with blank filler
        if page.is_empty() && line.text.trim().is_empty() {
            continue;
        }
        page.push(line.clone());

        if page.len() >= LINES_PER_PAGE {
            // don't strand a CHARACTER (or its parenthetical) at the foot
            let mut carry: Vec<Line> = Vec::new();
            while matches!(
                page.last().and_then(|l| l.element),
                Some(Element::Character) | Some(Element::Parenthetical)
            ) && page.len() > 4
            {
                carry.insert(0, page.pop().unwrap());
            }
            pages.push(std::mem::take(&mut page));
            page = carry;
        }
    }

    if !page.is_empty() {
        pages.push(page);
    }
    if pages.is_empty() {
        pages.push(Vec::new());
    }
    pages
}

pub fn page_count(doc: &Document) -> usize {
    paginate(&compose(doc)).len()
}

/// Rough runtime: one formatted page ~ one minute of screen time.
pub fn runtime_minutes(doc: &Document) -> usize {
    page_count(doc)
}

// ---------- plain text ----------

pub fn to_plain_text(doc: &Document) -> String {
    let pages = paginate(&compose(doc));
    let mut s = String::new();

    for (i, page) in pages.iter().enumerate() {
        if i > 0 {
            s.push('\u{000C}'); // form feed
            s.push('\n');
            let num = format!("{}.", i + 1);
            let pad = PAGE_COLS.saturating_sub(num.chars().count());
            s.push_str(&" ".repeat(pad));
            s.push_str(&num);
            s.push_str("\n\n");
        }
        for line in page {
            if line.text.trim().is_empty() {
                s.push('\n');
            } else {
                s.push_str(&" ".repeat(line.indent));
                s.push_str(&line.text);
                s.push('\n');
            }
        }
    }
    s
}

// ---------- fountain ----------

/// Fountain, for handing the script to Final Draft, Highland, Beat, afterwriting...
pub fn to_fountain(doc: &Document) -> String {
    let mut s = String::new();
    if !doc.meta.title.trim().is_empty() {
        s.push_str(&format!("Title: {}\n", doc.meta.title.trim()));
    }
    if !doc.meta.author.trim().is_empty() {
        s.push_str(&format!("Credit: written by\nAuthor: {}\n", doc.meta.author.trim()));
    }
    if !doc.meta.draft.trim().is_empty() {
        s.push_str(&format!("Draft date: {}\n", doc.meta.draft.trim()));
    }
    if !doc.meta.contact.trim().is_empty() {
        s.push_str(&format!("Contact: {}\n", doc.meta.contact.trim()));
    }
    if !s.is_empty() {
        s.push('\n');
    }

    for b in &doc.blocks {
        let text = b.text.trim();
        if text.is_empty() {
            continue;
        }
        match b.element {
            Element::SceneHeading => s.push_str(&format!("\n.{}\n\n", text.to_uppercase())),
            Element::Shot => s.push_str(&format!("\n{}\n\n", text.to_uppercase())),
            Element::Action => s.push_str(&format!("{}\n\n", text)),
            Element::Character => s.push_str(&format!("@{}\n", text.to_uppercase())),
            Element::Parenthetical => {
                let inner = text.trim_matches(|c| c == '(' || c == ')').trim();
                s.push_str(&format!("({})\n", inner));
            }
            Element::Dialogue => s.push_str(&format!("{}\n\n", text)),
            Element::Transition => s.push_str(&format!("> {}\n\n", text.to_uppercase())),
        }
    }
    s
}

// ---------- pdf ----------

const PAGE_W_MM: f32 = 215.9; // 8.5"
const PAGE_H_MM: f32 = 279.4; // 11"
const LEFT_MARGIN_MM: f32 = 38.1; // 1.5"
const TOP_MARGIN_MM: f32 = 25.4; // 1"
const CHAR_W_MM: f32 = 2.54; // 10 cpi
const LINE_H_MM: f32 = 25.4 / 6.0; // 12pt single spaced = 1/6"

pub fn to_pdf(doc: &Document, path: &Path) -> Result<(), String> {
    use printpdf::{BuiltinFont, Mm, PdfDocument};

    let title = if doc.meta.title.trim().is_empty() {
        "Untitled Script"
    } else {
        doc.meta.title.trim()
    };

    let (pdf, first_page, first_layer) =
        PdfDocument::new(title, Mm(PAGE_W_MM), Mm(PAGE_H_MM), "Script");
    let courier = pdf
        .add_builtin_font(BuiltinFont::Courier)
        .map_err(|e| e.to_string())?;
    let courier_bold = pdf
        .add_builtin_font(BuiltinFont::CourierBold)
        .map_err(|e| e.to_string())?;

    // --- title page ---
    {
        let layer = pdf.get_page(first_page).get_layer(first_layer);
        let put = |text: &str, line: f32, bold: bool| {
            let cols = text.chars().count() as f32;
            let x = LEFT_MARGIN_MM + (PAGE_COLS as f32 - cols).max(0.0) / 2.0 * CHAR_W_MM;
            let y = PAGE_H_MM - TOP_MARGIN_MM - line * LINE_H_MM;
            layer.use_text(
                text,
                12.0,
                Mm(x),
                Mm(y),
                if bold { &courier_bold } else { &courier },
            );
        };
        put(&title.to_uppercase(), 20.0, true);
        if !doc.meta.author.trim().is_empty() {
            put("written by", 24.0, false);
            put(doc.meta.author.trim(), 26.0, false);
        }
        if !doc.meta.draft.trim().is_empty() {
            put(doc.meta.draft.trim(), 44.0, false);
        }
        if !doc.meta.contact.trim().is_empty() {
            let y = PAGE_H_MM - TOP_MARGIN_MM - 48.0 * LINE_H_MM;
            layer.use_text(doc.meta.contact.trim(), 12.0, Mm(LEFT_MARGIN_MM), Mm(y), &courier);
        }
    }

    // --- body ---
    let pages = paginate(&compose(doc));
    for (i, page) in pages.iter().enumerate() {
        let (page_idx, layer_idx) =
            pdf.add_page(Mm(PAGE_W_MM), Mm(PAGE_H_MM), format!("Page {}", i + 1));
        let layer = pdf.get_page(page_idx).get_layer(layer_idx);

        // page number, top right, 0.5" down — omitted on the first body page
        if i > 0 {
            let num = format!("{}.", i + 1);
            let cols = num.chars().count() as f32;
            let x = LEFT_MARGIN_MM + (PAGE_COLS as f32 - cols) * CHAR_W_MM;
            let y = PAGE_H_MM - 12.7 - LINE_H_MM;
            layer.use_text(&num, 12.0, Mm(x), Mm(y), &courier);
        }

        for (row, line) in page.iter().enumerate() {
            if line.text.trim().is_empty() {
                continue;
            }
            let x = LEFT_MARGIN_MM + line.indent as f32 * CHAR_W_MM;
            let y = PAGE_H_MM - TOP_MARGIN_MM - (row as f32 + 1.0) * LINE_H_MM;
            let bold = matches!(line.element, Some(Element::SceneHeading));
            layer.use_text(
                &line.text,
                12.0,
                Mm(x),
                Mm(y),
                if bold { &courier_bold } else { &courier },
            );
        }
    }

    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut writer = io::BufWriter::new(file);
    pdf.save(&mut writer).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- entry points used by the UI ----------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Text,
    Fountain,
}

impl Format {
    pub fn label(self) -> &'static str {
        match self {
            Format::Pdf => "PDF",
            Format::Text => "Plain text",
            Format::Fountain => "Fountain",
        }
    }
    fn ext(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Text => "txt",
            Format::Fountain => "fountain",
        }
    }
}

pub fn export(doc: &Document, format: Format) -> Result<PathBuf, String> {
    storage::ensure_dirs().map_err(|e| e.to_string())?;
    let name = format!(
        "{}.{}",
        storage::slugify(&doc.meta.title),
        format.ext()
    );
    let path = storage::exports_dir().join(name);

    match format {
        Format::Pdf => to_pdf(doc, &path)?,
        Format::Text => std::fs::write(&path, to_plain_text(doc)).map_err(|e| e.to_string())?,
        Format::Fountain => std::fs::write(&path, to_fountain(doc)).map_err(|e| e.to_string())?,
    }
    Ok(path)
}
