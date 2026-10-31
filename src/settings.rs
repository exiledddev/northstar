//! What the user has decided the app should be like.
//!
//! Written to `settings.conf` next to the scripts as plain `key = value`
//! lines, so it can be read and fixed with a text editor if it ever gets into
//! a state the app will not open. Keys from older versions — the theme, the
//! window blur, the glass and following Tesseract — are read and ignored.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SortBy {
    #[default]
    Recent,
    Title,
    Length,
}

impl SortBy {
    pub const ALL: [SortBy; 3] = [SortBy::Recent, SortBy::Title, SortBy::Length];
    pub fn label(self) -> &'static str {
        match self {
            SortBy::Recent => "Last touched",
            SortBy::Title => "Title",
            SortBy::Length => "Length",
        }
    }
    fn slug(self) -> &'static str {
        match self {
            SortBy::Recent => "recent",
            SortBy::Title => "title",
            SortBy::Length => "length",
        }
    }
    fn from_slug(s: &str) -> SortBy {
        match s {
            "title" => SortBy::Title,
            "length" => SortBy::Length,
            _ => SortBy::Recent,
        }
    }
}

/// What Quick Export (Ctrl+E) does with the PDF once it is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AfterExport {
    /// Open it in the default PDF viewer — what Blackline always did.
    #[default]
    Open,
    /// Show it in the file manager.
    Reveal,
    /// Just say so.
    Nothing,
}

impl AfterExport {
    pub const ALL: [AfterExport; 3] = [AfterExport::Open, AfterExport::Reveal, AfterExport::Nothing];
    pub fn label(self) -> &'static str {
        match self {
            AfterExport::Open => "Open it",
            AfterExport::Reveal => "Show file",
            AfterExport::Nothing => "Nothing",
        }
    }
    fn slug(self) -> &'static str {
        match self {
            AfterExport::Open => "open",
            AfterExport::Reveal => "reveal",
            AfterExport::Nothing => "nothing",
        }
    }
    fn from_slug(s: &str) -> AfterExport {
        match s {
            "reveal" => AfterExport::Reveal,
            "nothing" => AfterExport::Nothing,
            _ => AfterExport::Open,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    // ---- the look ----
    /// The light cast instead of the dark one.
    pub light_mode: bool,
    pub animations: bool,
    pub font_scale: f32,
    pub splash: bool,
    pub starred_first: bool,
    pub sort_by: SortBy,
    /// Idle time before a touched script is written out, in milliseconds.
    pub autosave_ms: u64,

    // ---- the app ----
    pub show_library: bool,
    pub show_scenes: bool,
    pub show_cast: bool,
    /// The page's text size in points, Ctrl+plus / Ctrl+minus.
    pub page_px: f32,
    /// Show where each printed page will break, in the editor.
    pub page_breaks: bool,
    /// Number the scenes in the editor and in the PDF.
    pub scene_numbers: bool,
    /// Offer character names, places and transitions as you type.
    pub smart_type: bool,
    /// Keep the line being written in the middle of the page.
    pub typewriter: bool,
    pub after_export: AfterExport,
    /// Words to write per session; 0 is off.
    pub session_goal: u32,
    /// The script's details — title page and figures — as a panel on the right.
    pub show_details: bool,
    /// A faint band behind each block in Write, in its element's colour.
    pub element_colors: bool,
    /// Each speaking character's name and lines in a colour of their own.
    pub character_colors: bool,
    /// ... and in Reading mode too, not only while writing.
    pub character_colors_reading: bool,
    /// Turns the character colour wheel; Shuffle bumps it.
    pub colour_seed: u32,
    /// Custom: characters wear the colours chosen for them in the script
    /// (anyone without one is dealt one, as in Random). Random: everyone is
    /// dealt one from the wheel.
    pub custom_colors: bool,
    /// Where "View on YouTrack", at the foot of the library, goes.
    pub youtrack_url: String,
    // ---- what a PDF carries; remembered from the Export window ----
    pub pdf_title_page: bool,
    pub pdf_scene_numbers: bool,
    /// Each speaker's cue and lines printed in their colour.
    pub pdf_character_colors: bool,
    /// Keyboard shortcuts chosen instead of the defaults.
    pub keys: crate::keys::Keymap,
}

pub const DEFAULT_YOUTRACK: &str = "https://markedexiled.youtrack.cloud/dashboard?id=177-0";

impl Default for Settings {
    fn default() -> Self {
        Settings {
            light_mode: false,
            animations: true,
            font_scale: 1.0,
            splash: true,
            starred_first: true,
            sort_by: SortBy::Recent,
            autosave_ms: 1200,
            show_library: true,
            show_scenes: true,
            show_cast: false,
            page_px: 16.0,
            page_breaks: true,
            scene_numbers: false,
            smart_type: true,
            typewriter: false,
            after_export: AfterExport::Open,
            session_goal: 0,
            show_details: false,
            element_colors: true,
            character_colors: false,
            character_colors_reading: false,
            colour_seed: 0,
            custom_colors: false,
            youtrack_url: DEFAULT_YOUTRACK.to_string(),
            pdf_title_page: true,
            pdf_scene_numbers: false,
            pdf_character_colors: false,
            keys: crate::keys::Keymap::default(),
        }
    }
}

impl Settings {
    pub fn serialize(&self) -> String {
        let b = |v: bool| if v { "yes" } else { "no" };
        format!(
            "# Blackline settings.\n\
             light_mode = {}\n\
             animations = {}\n\
             font_scale = {:.2}\n\
             splash = {}\n\
             starred_first = {}\n\
             sort_by = {}\n\
             autosave_ms = {}\n\
             show_library = {}\n\
             show_scenes = {}\n\
             show_cast = {}\n\
             page_px = {:.0}\n\
             page_breaks = {}\n\
             scene_numbers = {}\n\
             smart_type = {}\n\
             typewriter = {}\n\
             after_export = {}\n\
             session_goal = {}\n\
             show_details = {}\n\
             element_colors = {}\n\
             character_colors = {}\n\
             character_colors_reading = {}\n\
             colour_seed = {}\n\
             youtrack_url = {}\n\
             pdf_title_page = {}\n\
             pdf_scene_numbers = {}\n\
             pdf_character_colors = {}\n",
            b(self.light_mode),
            b(self.animations),
            self.font_scale,
            b(self.splash),
            b(self.starred_first),
            self.sort_by.slug(),
            self.autosave_ms,
            b(self.show_library),
            b(self.show_scenes),
            b(self.show_cast),
            self.page_px,
            b(self.page_breaks),
            b(self.scene_numbers),
            b(self.smart_type),
            b(self.typewriter),
            self.after_export.slug(),
            self.session_goal,
            b(self.show_details),
            b(self.element_colors),
            b(self.character_colors),
            b(self.character_colors_reading),
            self.colour_seed,
            self.youtrack_url.trim(),
            b(self.pdf_title_page),
            b(self.pdf_scene_numbers),
            b(self.pdf_character_colors),
        ) + if self.custom_colors {
            // only when chosen, so a file that never chose stays as it was
            "character_colors_mode = custom\n"
        } else {
            ""
        } + &self.keys.serialize()
    }

    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        let yes = |v: &str| matches!(v, "yes" | "true" | "1" | "on");
        // Scene numbers used to be one switch for the editor and the PDF;
        // a file from then keeps its PDF as it was.
        let mut pdf_numbers_said = false;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "light_mode" => s.light_mode = yes(v),
                "animations" => s.animations = yes(v),
                "font_scale" => s.font_scale = v.parse::<f32>().unwrap_or(1.0).clamp(0.8, 1.4),
                "splash" => s.splash = yes(v),
                "starred_first" => s.starred_first = yes(v),
                "sort_by" => s.sort_by = SortBy::from_slug(v),
                "autosave_ms" => {
                    s.autosave_ms = v.parse::<u64>().unwrap_or(1200).clamp(150, 10_000)
                }
                "show_library" => s.show_library = yes(v),
                "show_scenes" => s.show_scenes = yes(v),
                "show_cast" => s.show_cast = yes(v),
                "page_px" => s.page_px = v.parse::<f32>().unwrap_or(16.0).clamp(11.0, 26.0),
                "page_breaks" => s.page_breaks = yes(v),
                "scene_numbers" => s.scene_numbers = yes(v),
                "smart_type" => s.smart_type = yes(v),
                "typewriter" => s.typewriter = yes(v),
                "after_export" => s.after_export = AfterExport::from_slug(v),
                "session_goal" => s.session_goal = v.parse::<u32>().unwrap_or(0).min(20_000),
                "show_details" => s.show_details = yes(v),
                "element_colors" => s.element_colors = yes(v),
                "character_colors" => s.character_colors = yes(v),
                "character_colors_reading" => s.character_colors_reading = yes(v),
                "colour_seed" => s.colour_seed = v.parse::<u32>().unwrap_or(0),
                "character_colors_mode" => s.custom_colors = v == "custom",
                k if k.starts_with("key.") => {
                    s.keys.parse_line(k, v);
                }
                "pdf_title_page" => s.pdf_title_page = yes(v),
                "pdf_scene_numbers" => {
                    s.pdf_scene_numbers = yes(v);
                    pdf_numbers_said = true;
                }
                "pdf_character_colors" => s.pdf_character_colors = yes(v),
                "youtrack_url" => {
                    s.youtrack_url = if v.is_empty() {
                        DEFAULT_YOUTRACK.to_string()
                    } else {
                        v.to_string()
                    }
                }
                _ => {}
            }
        }
        if !pdf_numbers_said {
            s.pdf_scene_numbers = s.scene_numbers;
        }
        s
    }
}
