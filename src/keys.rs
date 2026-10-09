//! Keyboard shortcuts, and the ones you choose instead.
//!
//! Every command the app's shortcuts reach has its default chords here — the
//! ones Northstar has always had — and a person can give any of them a chord
//! of their own. Only what differs from the defaults is kept, in their
//! settings: on the web those are their account's, so the keys follow them to
//! any browser. Writing keys (Enter, Tab, Backspace, Alt+arrows in the
//! script) are the page's own and are not part of this.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

use crate::model::Element;

/// Something a shortcut does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Command {
    Save,
    NewScript,
    NewScene,
    NewAct,
    Undo,
    Redo,
    Find,
    FindScripts,
    QuickExport,
    ExportWindow,
    NextView,
    FocusMode,
    ToggleLibrary,
    ToggleDetails,
    ToggleScenes,
    Settings,
    Bigger,
    Smaller,
    /// Ctrl+1–7: the element the caret's block is, 1-based.
    Element(u8),
}

use Command::*;

impl Command {
    pub const ALL: [Command; 25] = [
        Save,
        NewScript,
        NewScene,
        NewAct,
        Undo,
        Redo,
        Find,
        FindScripts,
        QuickExport,
        ExportWindow,
        NextView,
        FocusMode,
        ToggleLibrary,
        ToggleDetails,
        ToggleScenes,
        Settings,
        Bigger,
        Smaller,
        Element(1),
        Element(2),
        Element(3),
        Element(4),
        Element(5),
        Element(6),
        Element(7),
    ];

    /// How it is named in a settings file: `key.<slug> = Ctrl+S`.
    pub fn slug(self) -> String {
        match self {
            Save => "save".into(),
            NewScript => "new_script".into(),
            NewScene => "new_scene".into(),
            NewAct => "new_act".into(),
            Undo => "undo".into(),
            Redo => "redo".into(),
            Find => "find".into(),
            FindScripts => "find_scripts".into(),
            QuickExport => "quick_export".into(),
            ExportWindow => "export".into(),
            NextView => "next_view".into(),
            FocusMode => "focus_mode".into(),
            ToggleLibrary => "library".into(),
            ToggleDetails => "details".into(),
            ToggleScenes => "scenes".into(),
            Settings => "settings".into(),
            Bigger => "text_bigger".into(),
            Smaller => "text_smaller".into(),
            Element(n) => format!("element_{n}"),
        }
    }

    pub fn from_slug(s: &str) -> Option<Command> {
        Command::ALL.into_iter().find(|c| c.slug() == s)
    }

    pub fn label(self) -> String {
        match self {
            Save => "Save".into(),
            NewScript => "New script".into(),
            NewScene => "New scene".into(),
            NewAct => "New act".into(),
            Undo => "Undo".into(),
            Redo => "Redo".into(),
            Find => "Find and replace".into(),
            FindScripts => "Search your scripts".into(),
            QuickExport => "Quick Export (PDF)".into(),
            ExportWindow => "Export…".into(),
            NextView => "Next view (Write, Cards, Read)".into(),
            FocusMode => "Focus mode".into(),
            ToggleLibrary => "Show or hide the library".into(),
            ToggleDetails => "Show or hide Details".into(),
            ToggleScenes => "Show or hide Scenes".into(),
            Settings => "Settings".into(),
            Bigger => "Larger page text".into(),
            Smaller => "Smaller page text".into(),
            Element(n) => Element::from_digit(n as usize).map(|e| e.label()).unwrap_or("Element").to_string(),
        }
    }

    /// The heading it is listed under in Settings → Keyboard.
    pub fn group(self) -> &'static str {
        match self {
            Save | NewScript | NewScene | NewAct | Undo | Redo | Find | FindScripts => "Writing",
            QuickExport | ExportWindow => "Export",
            NextView | FocusMode | ToggleLibrary | ToggleDetails | ToggleScenes | Settings | Bigger | Smaller => {
                "Views and panels"
            }
            Element(_) => "Elements",
        }
    }

    /// The chords it has always had. A browser keeps Ctrl+N and Ctrl+1–7 for
    /// itself, so the web build has Ctrl+Alt+N and Alt+1–7 as well.
    pub fn defaults(self, web: bool) -> Vec<KeyboardShortcut> {
        let c = |m: Modifiers, k: Key| KeyboardShortcut::new(m, k);
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        match self {
            Save => vec![c(cmd, Key::S)],
            NewScript if web => vec![c(cmd | Modifiers::ALT, Key::N), c(cmd, Key::N)],
            NewScript => vec![c(cmd, Key::N)],
            NewScene => vec![c(cmd, Key::Enter)],
            NewAct => vec![c(cmd_shift, Key::Enter)],
            Undo => vec![c(cmd, Key::Z)],
            Redo => vec![c(cmd_shift, Key::Z), c(cmd, Key::Y)],
            Find => vec![c(cmd, Key::F), c(cmd, Key::H)],
            FindScripts => vec![c(cmd_shift, Key::F)],
            QuickExport => vec![c(cmd, Key::E)],
            ExportWindow => vec![c(cmd_shift, Key::E)],
            NextView => vec![c(cmd, Key::G)],
            FocusMode => vec![c(cmd, Key::Period)],
            ToggleLibrary => vec![c(cmd, Key::B)],
            ToggleDetails => vec![c(cmd, Key::I)],
            ToggleScenes => vec![c(cmd_shift, Key::I)],
            Settings => vec![c(cmd, Key::Comma)],
            Bigger => vec![c(cmd, Key::Plus), c(cmd, Key::Equals)],
            Smaller => vec![c(cmd, Key::Minus)],
            Element(n) => {
                let k = DIGITS[(n as usize).clamp(1, 7) - 1];
                if web {
                    vec![c(Modifiers::ALT, k), c(cmd, k)]
                } else {
                    vec![c(cmd, k)]
                }
            }
        }
    }
}

const DIGITS: [Key; 7] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7];

/// A chord as a key press reports it, reduced to what a shortcut means:
/// Ctrl and the Mac's Cmd are one modifier, `command`.
pub fn chord(modifiers: Modifiers, key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(
        Modifiers {
            alt: modifiers.alt,
            shift: modifiers.shift,
            command: modifiers.command || modifiers.ctrl || modifiers.mac_cmd,
            ctrl: false,
            mac_cmd: false,
        },
        key,
    )
}

/// `Ctrl+Shift+Enter`, as people write it.
pub fn text(c: &KeyboardShortcut) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if c.modifiers.command || c.modifiers.ctrl {
        parts.push("Ctrl");
    }
    if c.modifiers.alt {
        parts.push("Alt");
    }
    if c.modifiers.shift {
        parts.push("Shift");
    }
    let key = match c.logical_key {
        Key::Comma => ",",
        Key::Period => ".",
        Key::Plus => "+",
        Key::Equals => "=",
        Key::Minus => "-",
        Key::Slash => "/",
        Key::Backslash => "\\",
        Key::Semicolon => ";",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Backtick => "`",
        Key::Quote => "'",
        k => k.name(),
    };
    parts.push(key);
    parts.join("+")
}

/// Read `Ctrl+Shift+Enter` back. `None` if it is not a chord.
pub fn parse(s: &str) -> Option<KeyboardShortcut> {
    let s = s.trim();
    // the key itself may be "+": "Ctrl++"
    let (mods, key) = match s.strip_suffix("++") {
        Some(head) => (head, "+"),
        None => match s.rsplit_once('+') {
            Some((m, k)) => (m, k),
            None => ("", s),
        },
    };
    let mut m = Modifiers::NONE;
    for part in mods.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" | "command" => m.command = true,
            "alt" | "option" => m.alt = true,
            "shift" => m.shift = true,
            _ => return None,
        }
    }
    let key = match key.trim() {
        "," => Key::Comma,
        "." => Key::Period,
        "+" => Key::Plus,
        "=" => Key::Equals,
        "-" => Key::Minus,
        "/" => Key::Slash,
        "\\" => Key::Backslash,
        ";" => Key::Semicolon,
        "[" => Key::OpenBracket,
        "]" => Key::CloseBracket,
        "`" => Key::Backtick,
        "'" => Key::Quote,
        k => {
            // names as egui writes them ("Enter", "F2", "A"), whatever the case
            let mut c = k.chars();
            let capital = c.next().map(|f| f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase());
            Key::from_name(k)
                .or_else(|| capital.as_deref().and_then(Key::from_name))
                .or_else(|| Key::from_name(&k.to_ascii_uppercase()))?
        }
    };
    Some(KeyboardShortcut::new(m, key))
}

/// Why a chord cannot be a shortcut here, if it cannot.
pub fn refused(c: &KeyboardShortcut, web: bool) -> Option<&'static str> {
    let m = c.modifiers;
    let k = c.logical_key;
    let function = matches!(
        k,
        Key::F1 | Key::F2 | Key::F3 | Key::F4 | Key::F5 | Key::F6 | Key::F7 | Key::F8 | Key::F9 | Key::F10 | Key::F11 | Key::F12
    );
    if !(m.command || m.ctrl || m.alt || function) {
        return Some("Add Ctrl or Alt: a plain key is for typing.");
    }
    if matches!(k, Key::Escape | Key::Tab | Key::Backspace | Key::Delete) {
        return Some("That key belongs to the page.");
    }
    if m.alt && !(m.command || m.ctrl) && matches!(k, Key::ArrowUp | Key::ArrowDown) {
        return Some("Alt+Up and Down move a block in the script.");
    }
    if (m.command || m.ctrl) && !m.alt && !m.shift && matches!(k, Key::C | Key::V | Key::X | Key::A) {
        return Some("That is copy, cut, paste or select all.");
    }
    if web {
        // what a browser takes for itself and never hands to a page
        let ctrl = m.command || m.ctrl;
        let browser = ctrl
            && !m.alt
            && (matches!(k, Key::N | Key::T | Key::W | Key::Tab | Key::PageUp | Key::PageDown)
                || (!m.shift && DIGITS.contains(&k))
                || (!m.shift && matches!(k, Key::Num8 | Key::Num9)));
        if browser || matches!(k, Key::F5 | Key::F11 | Key::F12) {
            return Some("Your browser keeps that one for itself.");
        }
    }
    None
}

/// The chords someone chose instead of the defaults; empty is all defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keymap {
    custom: Vec<(Command, KeyboardShortcut)>,
}

impl Keymap {
    /// What runs `cmd`: the chord chosen for it, or its defaults.
    pub fn chords(&self, cmd: Command, web: bool) -> Vec<KeyboardShortcut> {
        match self.custom.iter().find(|(c, _)| *c == cmd) {
            Some((_, k)) => vec![*k],
            None => cmd.defaults(web),
        }
    }

    /// The chord shown for `cmd`: its first.
    pub fn label(&self, cmd: Command, web: bool) -> String {
        self.chords(cmd, web).first().map(text).unwrap_or_default()
    }

    pub fn is_custom(&self, cmd: Command) -> bool {
        self.custom.iter().any(|(c, _)| *c == cmd)
    }

    pub fn is_default(&self) -> bool {
        self.custom.is_empty()
    }

    /// Give `cmd` a chord of its own.
    pub fn set(&mut self, cmd: Command, chord: KeyboardShortcut, web: bool) {
        self.custom.retain(|(c, _)| *c != cmd);
        // choosing a default back is the same as resetting
        if cmd.defaults(web).first() != Some(&chord) {
            self.custom.push((cmd, chord));
        }
    }

    pub fn reset(&mut self, cmd: Command) {
        self.custom.retain(|(c, _)| *c != cmd);
    }

    pub fn reset_all(&mut self) {
        self.custom.clear();
    }

    /// The other command a chord already runs, among `commands`.
    pub fn taken_by(&self, chord: &KeyboardShortcut, except: Command, commands: &[Command], web: bool) -> Option<Command> {
        commands
            .iter()
            .copied()
            .filter(|c| *c != except)
            .find(|c| self.chords(*c, web).contains(chord))
    }

    /// Every chord to listen for, among `commands`, the most specific first.
    /// egui lets a chord with an extra Shift or Alt through to the plainer
    /// one, so Ctrl+Shift+Z must be asked for before Ctrl+Z.
    pub fn listen(&self, commands: &[Command], web: bool) -> Vec<(Command, KeyboardShortcut)> {
        let mut out: Vec<(Command, KeyboardShortcut)> = commands
            .iter()
            .flat_map(|c| self.chords(*c, web).into_iter().map(move |k| (*c, k)))
            .collect();
        let weight = |k: &KeyboardShortcut| k.modifiers.alt as u8 + k.modifiers.shift as u8;
        // stable: equally specific chords keep the order they are listed in
        out.sort_by_key(|(_, k)| std::cmp::Reverse(weight(k)));
        out
    }

    /// The lines a settings file carries: only the chords chosen.
    pub fn serialize(&self) -> String {
        self.custom
            .iter()
            .map(|(c, k)| format!("key.{} = {}\n", c.slug(), text(k)))
            .collect()
    }

    /// Read a settings line `key.<slug> = <chord>`; false if it is not one.
    pub fn parse_line(&mut self, key: &str, value: &str) -> bool {
        let Some(slug) = key.strip_prefix("key.") else {
            return false;
        };
        if let (Some(cmd), Some(chord)) = (Command::from_slug(slug.trim()), parse(value)) {
            self.custom.retain(|(c, _)| *c != cmd);
            self.custom.push((cmd, chord));
        }
        true
    }
}
