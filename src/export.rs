//! Turning the document into pages.
//!
//! Everything downstream (page count, plain text, PDF) is built from one
//! composition pass so the page count you see in the status bar is the page
//! count you get in the PDF.

use std::collections::HashMap;
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
    /// The block the line came from; `None` for spacing.
    pub block: Option<u64>,
    /// On the first line of a scene heading, the scene's number.
    pub scene: Option<usize>,
}

impl Line {
    fn blank() -> Self {
        Line {
            indent: 0,
            text: String::new(),
            element: None,
            block: None,
            scene: None,
        }
    }
}

/// Flatten the document into wrapped, indented lines.
pub fn compose(doc: &Document) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut scene_no = 0usize;

    for block in &doc.blocks {
        if block.element == Element::SceneHeading {
            // an empty heading still counts, so numbers match the navigator
            scene_no += 1;
        }
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

        for (k, l) in wrap(&body, e.width_cols()).into_iter().enumerate() {
            out.push(Line {
                indent: e.indent_cols(),
                text: l,
                element: Some(e),
                block: Some(block.id),
                scene: if k == 0 && e == Element::SceneHeading {
                    Some(scene_no)
                } else {
                    None
                },
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
#[allow(dead_code)]
pub fn runtime_minutes(doc: &Document) -> usize {
    page_count(doc)
}

/// Where each printed page begins: (page number, block id), for page 2 on.
/// The block is the first one with a line on that page, so a page that breaks
/// in the middle of a long paragraph is marked just above that paragraph.
pub fn page_starts(doc: &Document) -> Vec<(usize, u64)> {
    let pages = paginate(&compose(doc));
    let mut out = Vec::new();
    for (i, page) in pages.iter().enumerate().skip(1) {
        if let Some(id) = page.iter().find_map(|l| l.block) {
            out.push((i + 1, id));
        }
    }
    out
}

/// For every scene, in order: (heading id, page it starts on, length in
/// eighths of a page). Lengths are measured on the printed layout, and a
/// scene never measures less than an eighth.
pub fn scene_lengths(doc: &Document) -> Vec<(u64, usize, usize)> {
    let pages = paginate(&compose(doc));
    // flatten to (page, line within all printed lines, line)
    let mut starts: Vec<(u64, usize, usize)> = Vec::new(); // (id, page, global line)
    let mut total = 0usize;
    let heading_ids: Vec<u64> = doc
        .blocks
        .iter()
        .filter(|b| b.element == Element::SceneHeading)
        .map(|b| b.id)
        .collect();
    for (pi, page) in pages.iter().enumerate() {
        for l in page {
            if let (Some(id), Some(Element::SceneHeading)) = (l.block, l.element) {
                if !starts.iter().any(|(s, _, _)| *s == id) {
                    starts.push((id, pi + 1, total));
                }
            }
            total += 1;
        }
    }
    let mut out = Vec::new();
    for id in heading_ids {
        match starts.iter().position(|(s, _, _)| *s == id) {
            Some(k) => {
                let (_, page, at) = starts[k];
                let end = starts.get(k + 1).map(|x| x.2).unwrap_or(total);
                let lines = end.saturating_sub(at) as f32;
                let eighths = ((lines / LINES_PER_PAGE as f32) * 8.0).round().max(1.0) as usize;
                out.push((id, page, eighths));
            }
            // an empty heading prints nothing
            None => out.push((id, 0, 0)),
        }
    }
    out
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
            Element::SceneHeading => {
                s.push_str(&format!("\n.{}\n\n", text.to_uppercase()));
                if !b.note.trim().is_empty() {
                    s.push_str(&format!("= {}\n\n", b.note.trim()));
                }
            }
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

#[allow(dead_code)]
pub fn to_pdf(doc: &Document, path: &Path) -> Result<(), String> {
    to_pdf_opts(doc, path, &PdfOptions::default())
}

/// The PDF, optionally with scene numbers in both margins the way a shooting
/// script carries them.
#[allow(dead_code)]
pub fn to_pdf_with(doc: &Document, path: &Path, scene_numbers: bool) -> Result<(), String> {
    to_pdf_opts(
        doc,
        path,
        &PdfOptions {
            scene_numbers,
            ..PdfOptions::default()
        },
    )
}

/// An ink colour for the page, as 0-255 RGB.
pub type Ink = [u8; 3];

/// What goes into a PDF, beyond the script itself.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfOptions {
    pub title_page: bool,
    pub scene_numbers: bool,
    /// Which body pages, by the numbers printed on them (1-based). `None` is
    /// every page.
    pub pages: Option<Vec<usize>>,
    /// Each speaker's ink, by name; their cue and every line they say is
    /// printed in it. `None` prints everything black.
    pub voices: Option<HashMap<String, Ink>>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            title_page: true,
            scene_numbers: false,
            pages: None,
            voices: None,
        }
    }
}

/// One printed body page: the number printed on it, and each of its lines
/// with the ink it is set in (`None` is black).
#[derive(Clone, Debug)]
pub struct PlannedPage {
    pub number: usize,
    pub lines: Vec<(Line, Option<Ink>)>,
}

/// Lay the body out, decide who is speaking on every line — across page
/// breaks, so a speech that runs over keeps its colour — and keep only the
/// pages asked for. Page numbers stay the ones the full script has, the way a
/// production prints revised or selected pages.
pub fn pdf_plan(doc: &Document, opts: &PdfOptions) -> Vec<PlannedPage> {
    let pages = paginate(&compose(doc));
    let mut who: Option<String> = None;
    let mut out = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let number = i + 1;
        let mut lines = Vec::with_capacity(page.len());
        for line in page {
            match line.element {
                Some(Element::Character) => {
                    let n = crate::model::base_character(&line.text);
                    if !n.is_empty() {
                        who = Some(n);
                    }
                }
                Some(Element::Dialogue) | Some(Element::Parenthetical) | None => {}
                _ => who = None,
            }
            let ink = match (&opts.voices, &who) {
                (Some(v), Some(n)) if line.element.is_some() => v.get(n).copied(),
                _ => None,
            };
            lines.push((line.clone(), ink));
        }
        let wanted = opts.pages.as_ref().map(|w| w.contains(&number)).unwrap_or(true);
        if wanted {
            out.push(PlannedPage { number, lines });
        }
    }
    out
}

/// Read a page selection the way a print dialog does: `1-3, 7, 10-` — single
/// pages, ranges, and an open range running to the end. `max` is the number
/// of body pages. The result is sorted and without repeats.
pub fn parse_page_range(text: &str, max: usize) -> Result<Vec<usize>, String> {
    let mut out: Vec<usize> = Vec::new();
    let t = text.trim();
    if t.is_empty() {
        return Err("Name at least one page".into());
    }
    for part in t.split([',', ';', ' ']).map(str::trim).filter(|p| !p.is_empty()) {
        let num = |s: &str| -> Result<usize, String> {
            s.trim()
                .parse::<usize>()
                .map_err(|_| format!("\u{201c}{s}\u{201d} is not a page number"))
        };
        let (lo, hi) = match part.split_once(['-', '\u{2013}']) {
            Some((a, b)) => {
                let lo = if a.trim().is_empty() { 1 } else { num(a)? };
                let hi = if b.trim().is_empty() { max } else { num(b)? };
                (lo, hi)
            }
            None => {
                let n = num(part)?;
                (n, n)
            }
        };
        if lo == 0 || hi == 0 {
            return Err("Pages start at 1".into());
        }
        if lo > hi {
            return Err(format!("{lo}-{hi} runs backwards"));
        }
        if lo > max {
            return Err(format!(
                "There {} only {max} page{}",
                if max == 1 { "is" } else { "are" },
                if max == 1 { "" } else { "s" }
            ));
        }
        for n in lo..=hi.min(max) {
            if !out.contains(&n) {
                out.push(n);
            }
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// The body page a block's first line is printed on (1-based).
pub fn page_of_block(doc: &Document, id: u64) -> Option<usize> {
    paginate(&compose(doc))
        .iter()
        .position(|pg| pg.iter().any(|l| l.block == Some(id)))
        .map(|k| k + 1)
}

/// A page list written back compactly: `1-3, 7, 10-12`.
pub fn describe_pages(pages: &[usize]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < pages.len() {
        let start = pages[i];
        let mut end = start;
        while i + 1 < pages.len() && pages[i + 1] == end + 1 {
            i += 1;
            end = pages[i];
        }
        parts.push(if start == end { format!("{start}") } else { format!("{start}-{end}") });
        i += 1;
    }
    parts.join(", ")
}

pub fn to_pdf_opts(doc: &Document, path: &Path, opts: &PdfOptions) -> Result<(), String> {
    use printpdf::{BuiltinFont, Color, Mm, PdfDocument, Rgb};

    let plan = pdf_plan(doc, opts);
    if plan.is_empty() && !opts.title_page {
        return Err("Nothing to export: no pages chosen".into());
    }

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
    let colour = |ink: Option<Ink>| {
        let [r, g, b] = ink.unwrap_or([0, 0, 0]);
        Color::Rgb(Rgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, None))
    };

    // the first sheet PdfDocument made is used by whichever page comes first
    let mut first = Some((first_page, first_layer));
    let mut next_sheet = |name: String| match first.take() {
        Some(s) => s,
        None => pdf.add_page(Mm(PAGE_W_MM), Mm(PAGE_H_MM), name),
    };

    // --- title page ---
    if opts.title_page {
        let (pg, ly) = next_sheet("Title".into());
        let layer = pdf.get_page(pg).get_layer(ly);
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
    for page in &plan {
        let i = page.number - 1;
        let (page_idx, layer_idx) = next_sheet(format!("Page {}", page.number));
        let layer = pdf.get_page(page_idx).get_layer(layer_idx);
        layer.set_fill_color(colour(None));

        // page number, top right, 0.5" down — omitted on the first body page
        if i > 0 {
            let num = format!("{}.", i + 1);
            let cols = num.chars().count() as f32;
            let x = LEFT_MARGIN_MM + (PAGE_COLS as f32 - cols) * CHAR_W_MM;
            let y = PAGE_H_MM - 12.7 - LINE_H_MM;
            layer.use_text(&num, 12.0, Mm(x), Mm(y), &courier);
        }

        let mut inked: Option<Ink> = None;
        for (row, (line, ink)) in page.lines.iter().enumerate() {
            if line.text.trim().is_empty() {
                continue;
            }
            let x = LEFT_MARGIN_MM + line.indent as f32 * CHAR_W_MM;
            let y = PAGE_H_MM - TOP_MARGIN_MM - (row as f32 + 1.0) * LINE_H_MM;
            let bold = matches!(line.element, Some(Element::SceneHeading));
            if opts.scene_numbers {
                if let Some(n) = line.scene {
                    if inked.is_some() {
                        layer.set_fill_color(colour(None));
                        inked = None;
                    }
                    let num = format!("{n}");
                    let w = num.chars().count() as f32 * CHAR_W_MM;
                    // left: ending half an inch short of the text column
                    layer.use_text(&num, 12.0, Mm(LEFT_MARGIN_MM - 12.7 - w), Mm(y), &courier_bold);
                    // right: just past the end of the text column
                    let right = LEFT_MARGIN_MM + PAGE_COLS as f32 * CHAR_W_MM + 5.0;
                    layer.use_text(&num, 12.0, Mm(right), Mm(y), &courier_bold);
                }
            }
            // only switch ink when it changes, so a black page stays lean
            if *ink != inked {
                layer.set_fill_color(colour(*ink));
                inked = *ink;
            }
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

// ---------- final draft ----------

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Final Draft's own format, so a script can go straight to a production that
/// lives in it. Paragraph types are Final Draft's names for the same elements;
/// the synopsis travels as the scene's summary.
pub fn to_fdx(doc: &Document) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n");
    s.push_str("<FinalDraft DocumentType=\"Script\" Template=\"No\" Version=\"5\">\n");
    s.push_str("  <Content>\n");
    for b in &doc.blocks {
        let text = b.text.trim();
        if text.is_empty() {
            continue;
        }
        let kind = match b.element {
            Element::SceneHeading => "Scene Heading",
            Element::Action => "Action",
            Element::Character => "Character",
            Element::Parenthetical => "Parenthetical",
            Element::Dialogue => "Dialogue",
            Element::Transition => "Transition",
            Element::Shot => "Shot",
        };
        let body = if b.element == Element::Parenthetical {
            format!("({})", text.trim_matches(|c| c == '(' || c == ')').trim())
        } else if b.element.is_upper() {
            text.to_uppercase()
        } else {
            text.to_string()
        };
        s.push_str(&format!("    <Paragraph Type=\"{kind}\">\n"));
        if b.element == Element::SceneHeading && !b.note.trim().is_empty() {
            s.push_str(&format!(
                "      <SceneProperties><Summary><Paragraph><Text>{}</Text></Paragraph></Summary></SceneProperties>\n",
                xml(b.note.trim())
            ));
        }
        s.push_str(&format!("      <Text>{}</Text>\n", xml(&body)));
        s.push_str("    </Paragraph>\n");
    }
    s.push_str("  </Content>\n");
    s.push_str("  <TitlePage>\n    <Content>\n");
    let mut title_line = |text: &str, align: &str| {
        s.push_str(&format!(
            "      <Paragraph Alignment=\"{align}\"><Text>{}</Text></Paragraph>\n",
            xml(text)
        ));
    };
    title_line(&doc.meta.title.trim().to_uppercase(), "Center");
    if !doc.meta.author.trim().is_empty() {
        title_line("written by", "Center");
        title_line(doc.meta.author.trim(), "Center");
    }
    if !doc.meta.draft.trim().is_empty() {
        title_line(doc.meta.draft.trim(), "Center");
    }
    if !doc.meta.contact.trim().is_empty() {
        title_line(doc.meta.contact.trim(), "Left");
    }
    s.push_str("    </Content>\n  </TitlePage>\n");
    s.push_str("</FinalDraft>\n");
    s
}

// ---------- entry points used by the UI ----------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Pdf,
    Text,
    Fountain,
    FinalDraft,
}

impl Format {
    fn ext(self) -> &'static str {
        match self {
            Format::Pdf => "pdf",
            Format::Text => "txt",
            Format::Fountain => "fountain",
            Format::FinalDraft => "fdx",
        }
    }
}

#[allow(dead_code)]
pub fn export(doc: &Document, format: Format) -> Result<PathBuf, String> {
    export_with(doc, format, false)
}

pub fn export_with(doc: &Document, format: Format, scene_numbers: bool) -> Result<PathBuf, String> {
    export_opts(
        doc,
        format,
        &PdfOptions {
            scene_numbers,
            ..PdfOptions::default()
        },
    )
}

/// Export with the PDF's options. A PDF of only some pages says which in its
/// name, so it never overwrites the whole script's PDF.
pub fn export_opts(doc: &Document, format: Format, opts: &PdfOptions) -> Result<PathBuf, String> {
    storage::ensure_dirs().map_err(|e| e.to_string())?;
    let part = match (&opts.pages, format) {
        (Some(p), Format::Pdf) => format!("-pages-{}", describe_pages(p).replace(", ", "_")),
        _ => String::new(),
    };
    let name = format!(
        "{}{part}.{}",
        storage::slugify(&doc.meta.title),
        format.ext()
    );
    let path = storage::exports_dir().join(name);

    match format {
        Format::Pdf => to_pdf_opts(doc, &path, opts)?,
        Format::FinalDraft => std::fs::write(&path, to_fdx(doc)).map_err(|e| e.to_string())?,
        Format::Text => std::fs::write(&path, to_plain_text(doc)).map_err(|e| e.to_string())?,
        Format::Fountain => std::fs::write(&path, to_fountain(doc)).map_err(|e| e.to_string())?,
    }
    Ok(path)
}
