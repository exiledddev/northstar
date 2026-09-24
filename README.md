# Northstar

A screenwriting app for Linux. It behaves like a notes app — open it, type, it
saves itself — but every paragraph is a typed screenplay element, laid out to
standard master-scene geometry and exportable as a print-ready PDF.

Scripts are stored as plain markdown in `~/.local/share/northstar/scripts`, so
your work stays greppable, diffable, and yours.

```
sudo apt install build-essential pkg-config libgl1-mesa-dev libx11-dev \
  libxcursor-dev libxrandr-dev libxi-dev libxkbcommon-dev \
  libxkbcommon-x11-dev libwayland-dev

./install.sh
```

`install.sh` builds with `cargo build --release`, drops the binary in
`~/.local/bin`, and registers a desktop entry and icon so it shows up in the
KDE application launcher. Rust itself should come from
[rustup](https://rustup.rs) — the version in Debian's repositories is usually
too old for the GUI stack.

---

## Writing in it

The whole editor is keyboard-driven. Each block of text has an element type,
and the type decides the indent, the width, and whether it is forced to caps.

| Key | Does |
|---|---|
| `Enter` | New block. The type is chosen for you: Character → Dialogue, Dialogue → Action, Scene Heading → Action, Transition → Scene Heading |
| `Shift+Enter` | New block of the *same* type |
| `Tab` / `Shift+Tab` | Cycle the current block's type forward / back |
| `Ctrl+1…7` | Set type directly (Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot) |
| `Backspace` at the head of a block | Merge into the block above, or delete it if empty |
| `↑` / `↓` at the edges of a block | Move between blocks |
| `Alt+↑` / `Alt+↓` | Move the whole block up or down |
| `Ctrl+S` | Save now (also renames the file to match the title) |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / redo, document-wide |
| `Ctrl+N` | New script |
| `Ctrl+E` | Export PDF and open it |
| `Ctrl+B` / `Ctrl+I` | Toggle the library and details panels |
| `Ctrl+±` | Page text bigger / smaller |

Typing is saved automatically about a second after you stop. The dot next to
the title means there are unsaved changes.

Paste is smart: if you paste several lines at once, each line is guessed into
the right element (`INT.`/`EXT.` → scene heading, `(…)` → parenthetical,
`… TO:` → transition, short all-caps line → character, a line right after a
character → dialogue).

## Format

Layout follows the standard the Google Docs template on johnhaller.com
approximates — 12pt Courier at 10 characters per inch, 1.5″ left margin, 1″
right, 1″ top and bottom, which gives a 60-character text column and 55 lines
to a page.

| Element | Indent from page left | Width | Caps |
|---|---|---|---|
| Scene heading | 1.5″ | 6.0″ | yes |
| Action | 1.5″ | 6.0″ | no |
| Character | 3.7″ | 3.8″ | yes |
| Parenthetical | 3.1″ | 2.8″ | no |
| Dialogue | 2.5″ | 3.5″ | no |
| Transition | 6.0″ | 1.5″ | yes |
| Shot | 1.5″ | 6.0″ | yes |

The page count in the status bar comes from the same layout pass the PDF
exporter uses, so it isn't an estimate — it is the real page count, and one
page ≈ one minute of screen time.

## Files

```
~/.local/share/northstar/
    scripts/     one .md per screenplay
    exports/     pdf / txt / fountain output
```

Each script is CommonMark with YAML front matter. Every element maps to exactly
one markdown construct, so the round trip is lossless and the file still reads
properly in any markdown viewer:

```markdown
---
title: The Long Way Down
author: A. Writer
contact:
draft: First Draft
---

## INT. WAREHOUSE - NIGHT

Rain hammers the corrugated roof.

**MARIA (V.O.)**

*(barely audible)*

> Three, four... there you are.

### ANGLE ON THE DOOR

`SMASH CUT TO:`
```

`##` scene heading · `###` shot · plain paragraph action · `**bold**` character
· `*(italic parens)*` parenthetical · `>` dialogue · `` `code` `` transition.

Drop a hand-written file into `scripts/` following those rules and it opens
correctly. Anything unrecognised becomes action rather than being dropped.

Export gives you PDF (title page, page numbers, proper Courier geometry),
plain text, or Fountain if you want to hand the script to Final Draft,
Highland, or Beat.

## Layout of the source

| File | Contents |
|---|---|
| `src/model.rs` | Elements, indents, blocks, word wrap, paste guessing |
| `src/storage.rs` | The library folder, markdown read/write |
| `src/export.rs` | Line composition, pagination, PDF/text/Fountain |
| `src/editor.rs` | The page: block rendering and every structural keystroke |
| `src/app.rs` | Shell — library, top bar, details panel, autosave, undo |
| `src/theme.rs` | Palette, egui styling, system font discovery |
| `src/caret.rs` | The only place that touches egui's text-cursor internals |

`cargo test` runs the behavioural suite: markdown round-trip, wrap widths,
industry indents, pagination, PDF generation, paste guessing.

The page font is whichever it finds first: Courier Prime, Nimbus Mono,
Liberation Mono, DejaVu Sans Mono, then egui's built-in. For the real thing:

```
sudo apt install fonts-courier-prime
```

## Notes

Pinned to `eframe`/`egui` 0.29. If you move to a newer egui and the text cursor
API has shifted, `src/caret.rs` is the only file that needs attention — it is
four functions long and everything in it fails soft.

MIT.
