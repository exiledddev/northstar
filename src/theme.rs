//! Themes, tokens and the painting primitives the whole app is drawn with.
//!
//! This is the design system, shared with Tesseract value for value,
//! so the two apps sit side by side on a desktop as one family. Two surface
//! families, and which one a thing gets is the whole visual idea:
//!
//! * **glass** — translucent, sitting on whatever the compositor blurs behind
//!   the window. The ribbon, the script's title band, popovers, index cards,
//!   the gaps between islands. Nothing you stare at.
//! * **solid** — opaque islands with rounded corners. The library rail, the
//!   script page, the scene navigator. Everything you actually work in.
//!
//! Five themes, each with a light and a dark cast. A theme carries a *primary*
//! (the app's accent) and a *secondary* (the gradient the scene stars and their
//! glow are built from).

// The shared kit is kept whole and in step with Tesseract's copy: a
// primitive with no caller in Northstar today is still part of the language.
#![allow(dead_code)]

use std::sync::RwLock;

use crate::model::Element;

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Margin, Mesh, Pos2, Rect, Rounding,
    Shape, Stroke, Vec2,
};

// ---------------------------------------------------------------- identity --

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeId {
    #[default]
    Zen,
    Ember,
    Frostbite,
    Bloodmoon,
    Void,
    /// Northstar's own addition: orange into magenta into violet, on
    /// JetBrains' near-black.
    JetBrains,
}

impl ThemeId {
    pub const ALL: [ThemeId; 6] = [
        ThemeId::Zen,
        ThemeId::Ember,
        ThemeId::Frostbite,
        ThemeId::Bloodmoon,
        ThemeId::Void,
        ThemeId::JetBrains,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ThemeId::Zen => "Zen",
            ThemeId::Ember => "Ember",
            ThemeId::Frostbite => "Frostbite",
            ThemeId::Bloodmoon => "Bloodmoon",
            ThemeId::Void => "Void",
            ThemeId::JetBrains => "JetBrains",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            ThemeId::Zen => "Matcha and copper",
            ThemeId::Ember => "Autumn warmth, gold embers",
            ThemeId::Frostbite => "Winter light, ice blue",
            ThemeId::Bloodmoon => "Deep crimson",
            ThemeId::Void => "Violet dark",
            ThemeId::JetBrains => "Orange, magenta, violet",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ThemeId::Zen => "zen",
            ThemeId::Ember => "ember",
            ThemeId::Frostbite => "frostbite",
            ThemeId::Bloodmoon => "bloodmoon",
            ThemeId::Void => "void",
            ThemeId::JetBrains => "jetbrains",
        }
    }

    pub fn from_slug(s: &str) -> Option<ThemeId> {
        ThemeId::ALL.into_iter().find(|t| t.slug() == s)
    }
}

// ----------------------------------------------------------------- palette --

/// Every colour the app draws with. Copied out of the global on each read, so
/// it is deliberately small and plain.
#[derive(Clone, Copy)]
pub struct Palette {
    pub id: ThemeId,
    pub dark: bool,

    /// What the window is cleared to. Its alpha is set separately from the
    /// blur mode, see [`backdrop_alpha`].
    pub backdrop: Color32,

    // glass — translucent, over the blur
    pub glass: Color32,
    pub glass_hi: Color32,
    pub glass_line: Color32,
    pub glass_edge: Color32,

    // solid — the islands
    pub solid: Color32,
    pub solid_hi: Color32,
    pub sunken: Color32,
    pub raised: Color32,

    pub line: Color32,
    pub line_strong: Color32,

    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    /// Text laid on a primary-filled shape.
    pub ink: Color32,

    pub primary: Color32,
    pub primary_light: Color32,
    pub primary_deep: Color32,
    pub primary_quiet: Color32,
    pub primary_quiet_hi: Color32,
    pub prim_grad: (Color32, Color32),

    pub sec: Color32,
    pub sec_light: Color32,
    pub sec_deep: Color32,
    pub sec_grad: (Color32, Color32),

    pub danger: Color32,
    pub danger_light: Color32,
    pub warn: Color32,
    pub ok: Color32,

    pub groups: [Color32; 6],
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Palette {
    pub fn group(&self, hue: usize) -> Color32 {
        self.groups[hue % self.groups.len()]
    }
}

pub fn palette(id: ThemeId, dark: bool) -> Palette {
    match (id, dark) {
        // ------------------------------------------------------------- Zen --
        (ThemeId::Zen, true) => Palette {
            id,
            dark,
            backdrop: c(0x07, 0x0B, 0x09),
            glass: c(0x0E, 0x14, 0x11),
            glass_hi: c(0x16, 0x1F, 0x1A),
            glass_line: c(0x2A, 0x38, 0x31),
            glass_edge: c(0x4A, 0x60, 0x55),
            solid: c(0x0D, 0x12, 0x0F),
            solid_hi: c(0x15, 0x1C, 0x18),
            sunken: c(0x08, 0x0C, 0x0A),
            raised: c(0x1A, 0x23, 0x1E),
            line: c(0x20, 0x2A, 0x25),
            line_strong: c(0x33, 0x41, 0x3A),
            text: c(0xE8, 0xED, 0xE9),
            text_dim: c(0x9D, 0xA9, 0xA2),
            text_faint: c(0x66, 0x71, 0x6B),
            ink: c(0x06, 0x17, 0x0F),
            primary: c(0x7F, 0xB3, 0x94),
            primary_light: c(0xA9, 0xD4, 0xBB),
            primary_deep: c(0x35, 0x60, 0x4D),
            primary_quiet: c(0x0F, 0x1D, 0x18),
            primary_quiet_hi: c(0x15, 0x28, 0x21),
            prim_grad: (c(0x3E, 0x7E, 0x5C), c(0xAE, 0xD9, 0xC0)),
            sec: c(0xC0, 0x85, 0x52),
            sec_light: c(0xE8, 0xB0, 0x74),
            sec_deep: c(0x6B, 0x3A, 0x18),
            sec_grad: (c(0x8A, 0x4B, 0x1E), c(0xEC, 0xBA, 0x84)),
            danger: c(0xE2, 0x53, 0x40),
            danger_light: c(0xFF, 0x7A, 0x63),
            warn: c(0xF0, 0xB4, 0x3C),
            ok: c(0x7F, 0xB3, 0x94),
            groups: [
                c(0x7F, 0xB3, 0x94),
                c(0xC0, 0x85, 0x52),
                c(0x8F, 0xA8, 0xC4),
                c(0xC4, 0x9A, 0xB8),
                c(0xC4, 0x70, 0x5C),
                c(0x9C, 0xC2, 0x7F),
            ],
        },
        (ThemeId::Zen, false) => Palette {
            id,
            dark,
            backdrop: c(0xDD, 0xE5, 0xE0),
            glass: c(0xFA, 0xFD, 0xFB),
            glass_hi: c(0xFF, 0xFF, 0xFF),
            glass_line: c(0xD2, 0xDE, 0xD6),
            glass_edge: c(0xFF, 0xFF, 0xFF),
            solid: c(0xFF, 0xFF, 0xFF),
            solid_hi: c(0xF2, 0xF7, 0xF4),
            sunken: c(0xE7, 0xEE, 0xEA),
            raised: c(0xF7, 0xFA, 0xF8),
            line: c(0xDC, 0xE4, 0xDF),
            line_strong: c(0xB6, 0xC4, 0xBC),
            text: c(0x16, 0x20, 0x1A),
            text_dim: c(0x4E, 0x5A, 0x53),
            text_faint: c(0x7C, 0x88, 0x80),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0x2F, 0x7A, 0x57),
            primary_light: c(0x48, 0xA2, 0x77),
            primary_deep: c(0x1B, 0x4A, 0x34),
            primary_quiet: c(0xE4, 0xF2, 0xEA),
            primary_quiet_hi: c(0xD5, 0xEB, 0xDE),
            prim_grad: (c(0x25, 0x63, 0x46), c(0x63, 0xB9, 0x8D)),
            sec: c(0xA2, 0x66, 0x2F),
            sec_light: c(0xC9, 0x8B, 0x4C),
            sec_deep: c(0x6B, 0x3A, 0x18),
            sec_grad: (c(0x8A, 0x4B, 0x1E), c(0xD7, 0x9B, 0x58)),
            danger: c(0xD0, 0x3A, 0x22),
            danger_light: c(0xE8, 0x5B, 0x3E),
            warn: c(0xA9, 0x7A, 0x1E),
            ok: c(0x2F, 0x7A, 0x57),
            groups: [
                c(0x2F, 0x7A, 0x57),
                c(0xA2, 0x66, 0x2F),
                c(0x44, 0x6C, 0x99),
                c(0x93, 0x4F, 0x83),
                c(0xB2, 0x4B, 0x38),
                c(0x5F, 0x8A, 0x30),
            ],
        },

        // ----------------------------------------------------------- Ember --
        (ThemeId::Ember, true) => Palette {
            id,
            dark,
            backdrop: c(0x0B, 0x07, 0x05),
            glass: c(0x16, 0x0F, 0x0B),
            glass_hi: c(0x20, 0x16, 0x10),
            glass_line: c(0x3A, 0x2A, 0x1F),
            glass_edge: c(0x6A, 0x4C, 0x35),
            solid: c(0x12, 0x0C, 0x09),
            solid_hi: c(0x1B, 0x13, 0x0E),
            sunken: c(0x0C, 0x08, 0x06),
            raised: c(0x23, 0x19, 0x12),
            line: c(0x2C, 0x1F, 0x17),
            line_strong: c(0x48, 0x34, 0x27),
            text: c(0xF0, 0xE8, 0xE2),
            text_dim: c(0xAC, 0x9C, 0x90),
            text_faint: c(0x75, 0x66, 0x5C),
            ink: c(0x1A, 0x0A, 0x03),
            primary: c(0xC9, 0x74, 0x3F),
            primary_light: c(0xE8, 0xA0, 0x6A),
            primary_deep: c(0x5E, 0x2C, 0x18),
            primary_quiet: c(0x21, 0x11, 0x09),
            primary_quiet_hi: c(0x2E, 0x18, 0x0D),
            prim_grad: (c(0x5E, 0x2C, 0x18), c(0xD9, 0x86, 0x4B)),
            sec: c(0xFF, 0x8A, 0x3D),
            sec_light: c(0xFF, 0xD1, 0x66),
            sec_deep: c(0xB4, 0x5A, 0x16),
            sec_grad: (c(0xFF, 0x7A, 0x2E), c(0xFF, 0xD9, 0x7A)),
            danger: c(0xF2, 0x4B, 0x33),
            danger_light: c(0xFF, 0x86, 0x6B),
            warn: c(0xFF, 0xBC, 0x2E),
            ok: c(0x8F, 0xB5, 0x63),
            groups: [
                c(0xC9, 0x74, 0x3F),
                c(0xFF, 0xD1, 0x66),
                c(0xB5, 0x5A, 0x4A),
                c(0x8F, 0xA0, 0x5E),
                c(0xD0, 0x8A, 0x8A),
                c(0x9C, 0x6B, 0xB0),
            ],
        },
        (ThemeId::Ember, false) => Palette {
            id,
            dark,
            backdrop: c(0xE9, 0xDE, 0xD3),
            glass: c(0xFF, 0xFA, 0xF5),
            glass_hi: c(0xFF, 0xFF, 0xFF),
            glass_line: c(0xE0, 0xCE, 0xBE),
            glass_edge: c(0xFF, 0xFF, 0xFF),
            solid: c(0xFF, 0xFD, 0xFB),
            solid_hi: c(0xF7, 0xEF, 0xE8),
            sunken: c(0xEF, 0xE4, 0xDA),
            raised: c(0xFF, 0xF6, 0xEF),
            line: c(0xE5, 0xD6, 0xC9),
            line_strong: c(0xC6, 0xAE, 0x9B),
            text: c(0x24, 0x16, 0x10),
            text_dim: c(0x5E, 0x4A, 0x3D),
            text_faint: c(0x8B, 0x75, 0x66),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0xA9, 0x52, 0x2A),
            primary_light: c(0xC9, 0x74, 0x3F),
            primary_deep: c(0x6E, 0x30, 0x16),
            primary_quiet: c(0xFA, 0xEB, 0xE0),
            primary_quiet_hi: c(0xF3, 0xDD, 0xCB),
            prim_grad: (c(0x6E, 0x30, 0x16), c(0xC9, 0x74, 0x3F)),
            sec: c(0xE2, 0x70, 0x1A),
            sec_light: c(0xF0, 0xA9, 0x3A),
            sec_deep: c(0xA3, 0x4C, 0x10),
            sec_grad: (c(0xD9, 0x63, 0x1A), c(0xF5, 0xB9, 0x42)),
            danger: c(0xB8, 0x38, 0x2C),
            danger_light: c(0xD6, 0x5C, 0x4C),
            warn: c(0xB0, 0x7C, 0x14),
            ok: c(0x53, 0x82, 0x3E),
            groups: [
                c(0xA9, 0x52, 0x2A),
                c(0xC9, 0x91, 0x1E),
                c(0x9E, 0x40, 0x36),
                c(0x6B, 0x7D, 0x36),
                c(0xA6, 0x5A, 0x5A),
                c(0x6E, 0x48, 0x8A),
            ],
        },

        // -------------------------------------------------------- Frostbite --
        (ThemeId::Frostbite, true) => Palette {
            id,
            dark,
            backdrop: c(0x04, 0x07, 0x0B),
            glass: c(0x0B, 0x12, 0x1A),
            glass_hi: c(0x12, 0x1C, 0x26),
            glass_line: c(0x25, 0x36, 0x46),
            glass_edge: c(0x46, 0x66, 0x82),
            solid: c(0x0A, 0x10, 0x17),
            solid_hi: c(0x11, 0x1A, 0x23),
            sunken: c(0x07, 0x0C, 0x11),
            raised: c(0x17, 0x23, 0x2E),
            line: c(0x1D, 0x2B, 0x38),
            line_strong: c(0x31, 0x46, 0x5A),
            text: c(0xE6, 0xEE, 0xF5),
            text_dim: c(0x97, 0xA8, 0xB7),
            text_faint: c(0x63, 0x74, 0x84),
            ink: c(0x04, 0x12, 0x1B),
            primary: c(0x8F, 0xB8, 0xD9),
            primary_light: c(0xC2, 0xDC, 0xF0),
            primary_deep: c(0x2A, 0x42, 0x56),
            primary_quiet: c(0x0E, 0x1B, 0x26),
            primary_quiet_hi: c(0x15, 0x27, 0x35),
            prim_grad: (c(0x2A, 0x42, 0x56), c(0xCF, 0xE6, 0xF5)),
            sec: c(0x35, 0xC6, 0xF4),
            sec_light: c(0x8A, 0xE9, 0xFF),
            sec_deep: c(0x0E, 0x6E, 0x93),
            sec_grad: (c(0x1F, 0xA6, 0xE0), c(0x9D, 0xF0, 0xFF)),
            danger: c(0xF2, 0x43, 0x62),
            danger_light: c(0xFF, 0x76, 0x8E),
            warn: c(0xF5, 0xC8, 0x45),
            ok: c(0x5F, 0xC7, 0xA8),
            groups: [
                c(0x8F, 0xB8, 0xD9),
                c(0x35, 0xC6, 0xF4),
                c(0x9A, 0xB0, 0xE0),
                c(0x74, 0xD4, 0xC4),
                c(0xD9, 0x57, 0x6B),
                c(0xB8, 0xA8, 0xE8),
            ],
        },
        (ThemeId::Frostbite, false) => Palette {
            id,
            dark,
            backdrop: c(0xDB, 0xE6, 0xEF),
            glass: c(0xFB, 0xFD, 0xFF),
            glass_hi: c(0xFF, 0xFF, 0xFF),
            glass_line: c(0xCE, 0xDD, 0xE9),
            glass_edge: c(0xFF, 0xFF, 0xFF),
            solid: c(0xFF, 0xFF, 0xFF),
            solid_hi: c(0xF1, 0xF6, 0xFA),
            sunken: c(0xE4, 0xED, 0xF4),
            raised: c(0xF7, 0xFB, 0xFD),
            line: c(0xD7, 0xE2, 0xEB),
            line_strong: c(0xB0, 0xC2, 0xD1),
            text: c(0x10, 0x20, 0x2C),
            text_dim: c(0x48, 0x60, 0x6F),
            text_faint: c(0x7A, 0x8E, 0x9C),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0x2E, 0x6C, 0x96),
            primary_light: c(0x4A, 0x8F, 0xBE),
            primary_deep: c(0x1A, 0x3F, 0x58),
            primary_quiet: c(0xE3, 0xEF, 0xF7),
            primary_quiet_hi: c(0xD1, 0xE5, 0xF2),
            prim_grad: (c(0x1A, 0x3F, 0x58), c(0x6F, 0xB0, 0xD8)),
            sec: c(0x0E, 0x9B, 0xC8),
            sec_light: c(0x4F, 0xC7, 0xEA),
            sec_deep: c(0x07, 0x5E, 0x7C),
            sec_grad: (c(0x0E, 0x9B, 0xC8), c(0x6F, 0xDB, 0xF5)),
            danger: c(0xC0, 0x3D, 0x55),
            danger_light: c(0xDD, 0x64, 0x79),
            warn: c(0xA8, 0x86, 0x14),
            ok: c(0x1E, 0x8A, 0x6E),
            groups: [
                c(0x2E, 0x6C, 0x96),
                c(0x0E, 0x9B, 0xC8),
                c(0x4C, 0x60, 0xA8),
                c(0x1E, 0x8A, 0x6E),
                c(0xC0, 0x3D, 0x55),
                c(0x76, 0x5C, 0xB8),
            ],
        },

        // -------------------------------------------------------- Bloodmoon --
        (ThemeId::Bloodmoon, true) => Palette {
            id,
            dark,
            backdrop: c(0x0A, 0x04, 0x06),
            glass: c(0x16, 0x0A, 0x0D),
            glass_hi: c(0x20, 0x11, 0x15),
            glass_line: c(0x3A, 0x20, 0x26),
            glass_edge: c(0x6E, 0x3A, 0x44),
            solid: c(0x12, 0x0A, 0x0C),
            solid_hi: c(0x1B, 0x11, 0x14),
            sunken: c(0x0C, 0x07, 0x09),
            raised: c(0x24, 0x14, 0x18),
            line: c(0x2E, 0x1A, 0x1E),
            line_strong: c(0x4A, 0x2A, 0x31),
            text: c(0xF3, 0xE7, 0xE9),
            text_dim: c(0xB0, 0x98, 0x9D),
            text_faint: c(0x7A, 0x64, 0x69),
            ink: c(0x14, 0x04, 0x06),
            primary: c(0xC6, 0x41, 0x4A),
            primary_light: c(0xE4, 0x73, 0x7B),
            primary_deep: c(0x4A, 0x0F, 0x16),
            primary_quiet: c(0x20, 0x0C, 0x10),
            primary_quiet_hi: c(0x2D, 0x11, 0x16),
            prim_grad: (c(0x4A, 0x0F, 0x16), c(0xD9, 0x60, 0x6A)),
            sec: c(0xE0, 0x1F, 0x3D),
            sec_light: c(0xFF, 0x8A, 0x5F),
            sec_deep: c(0x86, 0x09, 0x1F),
            sec_grad: (c(0xB3, 0x12, 0x2E), c(0xFF, 0x9A, 0x6B)),
            danger: c(0xFF, 0x5A, 0x5F),
            danger_light: c(0xFF, 0x8A, 0x8E),
            warn: c(0xF7, 0xAE, 0x33),
            ok: c(0x76, 0xB0, 0x86),
            groups: [
                c(0xC6, 0x41, 0x4A),
                c(0xFF, 0x8A, 0x5F),
                c(0xB0, 0x6A, 0x9A),
                c(0x8A, 0x6E, 0xB8),
                c(0xD9, 0xA8, 0x5C),
                c(0x76, 0xB0, 0x86),
            ],
        },
        (ThemeId::Bloodmoon, false) => Palette {
            id,
            dark,
            backdrop: c(0xEA, 0xDA, 0xDC),
            glass: c(0xFF, 0xFB, 0xFB),
            glass_hi: c(0xFF, 0xFF, 0xFF),
            glass_line: c(0xE2, 0xCC, 0xCF),
            glass_edge: c(0xFF, 0xFF, 0xFF),
            solid: c(0xFF, 0xFF, 0xFF),
            solid_hi: c(0xF8, 0xEF, 0xF0),
            sunken: c(0xEF, 0xE0, 0xE2),
            raised: c(0xFF, 0xF6, 0xF6),
            line: c(0xE6, 0xD2, 0xD5),
            line_strong: c(0xC4, 0xA4, 0xA9),
            text: c(0x26, 0x12, 0x15),
            text_dim: c(0x5E, 0x40, 0x46),
            text_faint: c(0x8A, 0x6A, 0x70),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0xA8, 0x1F, 0x2C),
            primary_light: c(0xC6, 0x41, 0x4A),
            primary_deep: c(0x6E, 0x0A, 0x14),
            primary_quiet: c(0xFA, 0xE7, 0xE9),
            primary_quiet_hi: c(0xF3, 0xD6, 0xD9),
            prim_grad: (c(0x6E, 0x0A, 0x14), c(0xC6, 0x41, 0x4A)),
            sec: c(0xC4, 0x12, 0x30),
            sec_light: c(0xF0, 0x7A, 0x4E),
            sec_deep: c(0x7C, 0x04, 0x18),
            sec_grad: (c(0xC4, 0x12, 0x30), c(0xF5, 0x8B, 0x5C)),
            danger: c(0xB0, 0x18, 0x24),
            danger_light: c(0xD4, 0x44, 0x50),
            warn: c(0xA8, 0x7A, 0x18),
            ok: c(0x30, 0x7A, 0x4E),
            groups: [
                c(0xA8, 0x1F, 0x2C),
                c(0xD9, 0x63, 0x30),
                c(0x94, 0x3C, 0x77),
                c(0x5A, 0x46, 0x9E),
                c(0xA8, 0x7A, 0x18),
                c(0x30, 0x7A, 0x4E),
            ],
        },

        // ------------------------------------------------------------ Void --
        (ThemeId::Void, true) => Palette {
            id,
            dark,
            backdrop: c(0x06, 0x04, 0x0B),
            glass: c(0x12, 0x0D, 0x1C),
            glass_hi: c(0x1A, 0x13, 0x27),
            glass_line: c(0x30, 0x24, 0x46),
            glass_edge: c(0x5C, 0x46, 0x84),
            solid: c(0x0F, 0x0A, 0x18),
            solid_hi: c(0x16, 0x10, 0x22),
            sunken: c(0x0A, 0x07, 0x13),
            raised: c(0x1E, 0x16, 0x30),
            line: c(0x28, 0x1E, 0x3A),
            line_strong: c(0x3F, 0x2F, 0x5A),
            text: c(0xEC, 0xE7, 0xF5),
            text_dim: c(0xA9, 0x9C, 0xC0),
            text_faint: c(0x74, 0x68, 0x89),
            ink: c(0x0B, 0x05, 0x18),
            primary: c(0x9B, 0x6B, 0xE0),
            primary_light: c(0xC7, 0xA6, 0xFF),
            primary_deep: c(0x2E, 0x1A, 0x4D),
            primary_quiet: c(0x18, 0x10, 0x2A),
            primary_quiet_hi: c(0x22, 0x17, 0x3B),
            prim_grad: (c(0x2E, 0x1A, 0x4D), c(0xC7, 0xA6, 0xFF)),
            sec: c(0x7C, 0x3A, 0xED),
            sec_light: c(0xE8, 0x79, 0xF9),
            sec_deep: c(0x4B, 0x1E, 0x96),
            sec_grad: (c(0x7C, 0x3A, 0xED), c(0xEC, 0x8F, 0xFB)),
            danger: c(0xE5, 0x48, 0x4D),
            danger_light: c(0xF0, 0x7A, 0x7E),
            warn: c(0xF7, 0xC0, 0x3B),
            ok: c(0x62, 0xC9, 0xA8),
            groups: [
                c(0x9B, 0x6B, 0xE0),
                c(0xE8, 0x79, 0xF9),
                c(0x6B, 0x8A, 0xE0),
                c(0x62, 0xC9, 0xA8),
                c(0xE5, 0x48, 0x4D),
                c(0xE0, 0xB8, 0x5C),
            ],
        },
        (ThemeId::Void, false) => Palette {
            id,
            dark,
            backdrop: c(0xE2, 0xDC, 0xEF),
            glass: c(0xFC, 0xFA, 0xFF),
            glass_hi: c(0xFF, 0xFF, 0xFF),
            glass_line: c(0xDA, 0xD0, 0xEC),
            glass_edge: c(0xFF, 0xFF, 0xFF),
            solid: c(0xFF, 0xFF, 0xFF),
            solid_hi: c(0xF5, 0xF1, 0xFB),
            sunken: c(0xE9, 0xE3, 0xF3),
            raised: c(0xFA, 0xF7, 0xFE),
            line: c(0xE0, 0xD8, 0xEE),
            line_strong: c(0xBC, 0xAE, 0xD6),
            text: c(0x1A, 0x12, 0x26),
            text_dim: c(0x4E, 0x41, 0x66),
            text_faint: c(0x7C, 0x6E, 0x96),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0x6D, 0x3B, 0xC4),
            primary_light: c(0x8B, 0x5C, 0xE0),
            primary_deep: c(0x3F, 0x1F, 0x7A),
            primary_quiet: c(0xEE, 0xE7, 0xFA),
            primary_quiet_hi: c(0xE1, 0xD6, 0xF6),
            prim_grad: (c(0x3F, 0x1F, 0x7A), c(0x8B, 0x5C, 0xE0)),
            sec: c(0x7C, 0x3A, 0xED),
            sec_light: c(0xC0, 0x5B, 0xE0),
            sec_deep: c(0x4B, 0x1E, 0x96),
            sec_grad: (c(0x7C, 0x3A, 0xED), c(0xD0, 0x6B, 0xE8)),
            danger: c(0xC0, 0x2E, 0x3A),
            danger_light: c(0xDC, 0x56, 0x60),
            warn: c(0xA8, 0x7C, 0x14),
            ok: c(0x1E, 0x8A, 0x6E),
            groups: [
                c(0x6D, 0x3B, 0xC4),
                c(0xB0, 0x3C, 0xC4),
                c(0x3C, 0x5C, 0xC4),
                c(0x1E, 0x8A, 0x6E),
                c(0xC0, 0x2E, 0x3A),
                c(0xA8, 0x7C, 0x14),
            ],
        },

        // ------------------------------------------------------- JetBrains --
        // Taken from the JetBrains artwork: a near-black ground, and arches
        // that run from marigold through pink and magenta into deep violet.
        // The secondary — the energy colour the stars are made of — is the
        // warm end of that run; the primary is its magenta heart.
        (ThemeId::JetBrains, true) => Palette {
            id,
            dark,
            backdrop: c(0x0D, 0x0C, 0x10),
            glass: c(0x17, 0x16, 0x1A),
            glass_hi: c(0x20, 0x1E, 0x25),
            glass_line: c(0x33, 0x2C, 0x3C),
            glass_edge: c(0x62, 0x4C, 0x74),
            solid: c(0x15, 0x14, 0x18),
            solid_hi: c(0x1D, 0x1B, 0x21),
            sunken: c(0x0F, 0x0E, 0x12),
            raised: c(0x24, 0x21, 0x2A),
            line: c(0x2A, 0x26, 0x30),
            line_strong: c(0x43, 0x3A, 0x4D),
            text: c(0xEE, 0xEC, 0xF1),
            text_dim: c(0xA9, 0xA3, 0xB3),
            text_faint: c(0x71, 0x6B, 0x7B),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0xB0, 0x52, 0xC4),
            primary_light: c(0xD4, 0x9A, 0xE0),
            primary_deep: c(0x4E, 0x12, 0x78),
            primary_quiet: c(0x1D, 0x10, 0x25),
            primary_quiet_hi: c(0x2A, 0x15, 0x37),
            prim_grad: (c(0x45, 0x26, 0x8E), c(0xA2, 0x3E, 0xB2)),
            sec: c(0xE8, 0x92, 0x4E),
            sec_light: c(0xF0, 0xBE, 0x62),
            sec_deep: c(0xA8, 0x2E, 0x6A),
            sec_grad: (c(0xC8, 0x4C, 0x8E), c(0xF0, 0xB4, 0x55)),
            danger: c(0xFF, 0x4F, 0x5E),
            danger_light: c(0xFF, 0x82, 0x8B),
            warn: c(0xFD, 0xB3, 0x2A),
            ok: c(0x3D, 0xD6, 0x95),
            groups: [
                c(0xD1, 0x4B, 0xE0),
                c(0xFD, 0xB3, 0x2A),
                c(0x7E, 0x6B, 0xFF),
                c(0xF0, 0x5C, 0xA8),
                c(0x3D, 0xC5, 0xE0),
                c(0x3D, 0xD6, 0x95),
            ],
        },
        (ThemeId::JetBrains, false) => Palette {
            id,
            dark,
            backdrop: c(0xE4, 0xE1, 0xEA),
            glass: c(0xFC, 0xFB, 0xFE),
            glass_hi: c(0xFF, 0xFF, 0xFF),
            glass_line: c(0xDA, 0xD3, 0xE4),
            glass_edge: c(0xFF, 0xFF, 0xFF),
            solid: c(0xFF, 0xFF, 0xFF),
            solid_hi: c(0xF6, 0xF3, 0xF9),
            sunken: c(0xEB, 0xE7, 0xF1),
            raised: c(0xFA, 0xF8, 0xFD),
            line: c(0xE2, 0xDC, 0xEA),
            line_strong: c(0xBF, 0xB4, 0xCD),
            text: c(0x17, 0x13, 0x1F),
            text_dim: c(0x4C, 0x44, 0x58),
            text_faint: c(0x7D, 0x75, 0x89),
            ink: c(0xFF, 0xFF, 0xFF),
            primary: c(0x9A, 0x10, 0xA8),
            primary_light: c(0xB6, 0x3A, 0xC6),
            primary_deep: c(0x4A, 0x0A, 0x6E),
            primary_quiet: c(0xF6, 0xE7, 0xF8),
            primary_quiet_hi: c(0xEE, 0xD5, 0xF2),
            prim_grad: (c(0x4A, 0x2E, 0x96), c(0x96, 0x34, 0xA6)),
            sec: c(0xE0, 0x6E, 0x10),
            sec_light: c(0xF2, 0x98, 0x1C),
            sec_deep: c(0x9A, 0x22, 0x5C),
            sec_grad: (c(0xB8, 0x3A, 0x7C), c(0xE0, 0x96, 0x2E)),
            danger: c(0xC4, 0x28, 0x3A),
            danger_light: c(0xE0, 0x50, 0x5E),
            warn: c(0xB0, 0x7A, 0x0E),
            ok: c(0x17, 0x8A, 0x5E),
            groups: [
                c(0x9A, 0x10, 0xA8),
                c(0xC2, 0x7A, 0x0A),
                c(0x5A, 0x2B, 0xD0),
                c(0xC0, 0x28, 0x7A),
                c(0x1E, 0x8F, 0xA8),
                c(0x17, 0x8A, 0x5E),
            ],
        },
    }
}

static CURRENT: RwLock<Option<Palette>> = RwLock::new(None);
/// How solid the glass reads, as f32 bits. Set from Settings and from whether
/// the compositor actually gave us a blur.
static GLASS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0x3F19999A); // 0.6

/// The palette everything paints with.
pub fn pal() -> Palette {
    if let Ok(g) = CURRENT.read() {
        if let Some(p) = *g {
            return p;
        }
    }
    palette(ThemeId::Zen, true)
}

pub fn set_palette(id: ThemeId, dark: bool) {
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(palette(id, dark));
    }
}

/// 0.35 (very see-through) to 1.0 (opaque).
pub fn set_glass_opacity(v: f32) {
    GLASS.store(
        v.clamp(0.2, 1.0).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

pub fn glass_opacity() -> f32 {
    f32::from_bits(GLASS.load(std::sync::atomic::Ordering::Relaxed))
}

// ----------------------------------------------------------------- metrics --

/// The window's own corner radius.
pub const R_WINDOW: f32 = 13.0;
pub const R_SM: f32 = 8.0;
pub const R_MD: f32 = 12.0;
pub const R_CTRL: f32 = 10.0;
pub const R_CARD: f32 = 16.0;
pub const R_ISLAND: f32 = 18.0;
pub const R_PANEL: f32 = 24.0;
pub const R_PILL: f32 = 999.0;

pub const RAIL_W: f32 = 268.0;
pub const OUTLINE_W: f32 = 246.0;
pub const INSPECT_W: f32 = 272.0;
/// Title bar plus ribbon, with room around the ribbon's controls for the light
/// they cast — clip that and the glow gets a straight edge.
pub const TOPBAR_H: f32 = 92.0;
pub const DOC_MAX: f32 = 820.0;
/// The gap of blurred backdrop between the islands. Everything breathes on
/// this one number.
pub const GAP: f32 = 14.0;

pub const T_TITLE: f32 = 26.0;
pub const T_H: f32 = 17.0;
pub const T_BODY: f32 = 14.5;
pub const T_SM: f32 = 13.0;
pub const T_LABEL: f32 = 12.0;
pub const T_CAP: f32 = 11.0;
pub const T_MICRO: f32 = 10.0;

// ------------------------------------------------------------- colour math --

pub fn wash(col: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), alpha)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

pub fn lighten(col: Color32, t: f32) -> Color32 {
    mix(col, Color32::WHITE, t)
}

pub fn darken(col: Color32, t: f32) -> Color32 {
    mix(col, Color32::BLACK, t)
}

/// Readable text colour for something sitting on `bg`.
pub fn on(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 150.0 {
        c(0x10, 0x12, 0x11)
    } else {
        c(0xF4, 0xF7, 0xF5)
    }
}

// ------------------------------------------------------------------ shapes --

/// The outline of a rounded rectangle, as a convex polygon. Corners get a few
/// segments each, which is plenty at the radii we use and keeps the meshes
/// small enough to throw away every frame.
pub fn rounded_poly(rect: Rect, radius: f32) -> Vec<Pos2> {
    let r = radius.min(rect.width() * 0.5).min(rect.height() * 0.5).max(0.0);
    if r <= 0.5 {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ];
    }
    let steps = ((r * 0.9) as usize).clamp(5, 18);
    let mut pts = Vec::with_capacity(steps * 4 + 4);
    let corners = [
        (rect.right() - r, rect.top() + r, -std::f32::consts::FRAC_PI_2),
        (rect.right() - r, rect.bottom() - r, 0.0),
        (rect.left() + r, rect.bottom() - r, std::f32::consts::FRAC_PI_2),
        (rect.left() + r, rect.top() + r, std::f32::consts::PI),
    ];
    for (cx, cy, a0) in corners {
        for i in 0..=steps {
            let a = a0 + std::f32::consts::FRAC_PI_2 * (i as f32 / steps as f32);
            pts.push(Pos2::new(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    pts
}

/// Fill a convex polygon with a linear gradient running along `dir`.
pub fn fill_grad_poly(painter: &egui::Painter, pts: &[Pos2], a: Color32, b: Color32, dir: Vec2) {
    painter.add(grad_poly_shape(pts, a, b, dir));
}

/// The same gradient as a shape, for anything that has to be slotted in
/// underneath content that was laid out after it.
pub fn grad_poly_shape(pts: &[Pos2], a: Color32, b: Color32, dir: Vec2) -> Shape {
    if pts.len() < 3 {
        return Shape::Noop;
    }
    // egui feathers the shapes it tessellates itself; a mesh built by hand gets
    // none, which is what makes a gradient's edge look chewed. One extra ring
    // a pixel out at zero alpha does the same job.
    let dir = if dir.length_sq() < 1e-6 {
        Vec2::new(0.0, 1.0)
    } else {
        dir.normalized()
    };
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for p in pts {
        let t = p.to_vec2().dot(dir);
        lo = lo.min(t);
        hi = hi.max(t);
    }
    let span = (hi - lo).max(0.0001);
    let at = |p: Pos2| -> Color32 { mix(a, b, (p.to_vec2().dot(dir) - lo) / span) };

    let mut centre = Vec2::ZERO;
    for p in pts {
        centre += p.to_vec2();
    }
    let centre = (centre / pts.len() as f32).to_pos2();

    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, at(centre));
    for p in pts {
        mesh.colored_vertex(*p, at(*p));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }

    const FEATHER: f32 = 0.9;
    let base = mesh.vertices.len() as u32;
    for p in pts {
        let out = (*p - centre).normalized() * FEATHER;
        let q = *p + out;
        mesh.colored_vertex(q, wash(at(*p), 0));
    }
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.add_triangle(1 + i, base + i, base + j);
        mesh.add_triangle(1 + i, base + j, 1 + j);
    }
    Shape::mesh(mesh)
}

/// A rounded rectangle filled with a gradient. `dir` defaults to top→bottom.
pub fn grad_rect(painter: &egui::Painter, rect: Rect, radius: f32, a: Color32, b: Color32, dir: Vec2) {
    let pts = rounded_poly(rect, radius);
    fill_grad_poly(painter, &pts, a, b, dir);
}

/// A rhombus filled with a gradient — the theme swatch's marker.
pub fn grad_diamond(painter: &egui::Painter, centre: Pos2, r: f32, a: Color32, b: Color32) {
    let pts = vec![
        Pos2::new(centre.x, centre.y - r),
        Pos2::new(centre.x + r, centre.y),
        Pos2::new(centre.x, centre.y + r),
        Pos2::new(centre.x - r, centre.y),
    ];
    fill_grad_poly(painter, &pts, a, b, Vec2::new(0.6, 1.0));
}

/// An arc that fades out at both ends — the two quarters of outline on a
/// glass orb.
pub fn fading_arc(
    painter: &egui::Painter,
    centre: Pos2,
    radius: f32,
    from: f32,
    to: f32,
    width: f32,
    color: Color32,
) {
    let steps = 22;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let a0 = from + (to - from) * t0;
        let a1 = from + (to - from) * t1;
        // full strength in the middle of the arc, nothing at the ends
        let mid = (t0 + t1) * 0.5;
        let fade = (mid * std::f32::consts::PI).sin().powf(0.7);
        let col = wash(color, (color.a() as f32 * fade) as u8);
        painter.line_segment(
            [
                Pos2::new(centre.x + a0.cos() * radius, centre.y + a0.sin() * radius),
                Pos2::new(centre.x + a1.cos() * radius, centre.y + a1.sin() * radius),
            ],
            Stroke::new(width, col),
        );
    }
}

// ------------------------------------------------------------------- bloom --

/// How finely a bloom is tessellated. Wider blooms get more rings, but the
/// alpha is interpolated *between* them by the rasteriser, so this is about
/// following the falloff curve, not about hiding steps.
fn bloom_rings(spread: f32) -> usize {
    ((spread * 0.40) as usize).clamp(10, 30)
}

/// The curve a glow falls off along.
///
/// The point of a neon surface is that the *object* is bright and the light
/// around it is a wash you barely notice until it is not there. A hot, tight
/// halo reads as a sticker of a glow; this is mostly a long, low tail with
/// just enough near the object to seat it.
fn falloff(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    // Deliberately no spike at t=0: a glow that is at full strength right at
    // the object's edge draws a bright rim around it, which is exactly the
    // look this is trying to get away from.
    let near = (-3.2 * t).exp();
    let far = (1.0 - t).powf(2.6);
    (0.22 * near + 0.78 * far) * 0.82
}

/// A bloom drawn as **one mesh**: concentric rings of vertices whose alpha
/// falls away smoothly. Stacked translucent shapes are what band; a single
/// mesh blends once per pixel and the rasteriser interpolates the alpha
/// between rings, so the result is as smooth as the curve itself.
///
/// `ring` is handed a distance from the shape and returns the outline there.
/// Every ring must carry the same number of points.
fn bloom_mesh<F>(
    painter: &egui::Painter,
    centre: Pos2,
    spread: f32,
    color: Color32,
    strength: f32,
    ring: F,
) where
    F: Fn(f32) -> Vec<Pos2>,
{
    if strength <= 0.002 || spread <= 0.1 {
        return;
    }
    // Low. A glow is atmosphere, not a second object drawn behind the first.
    let peak = (strength.clamp(0.0, 2.0) * 62.0).min(255.0) as u8;
    let rings = bloom_rings(spread);

    let mut mesh = Mesh::default();
    mesh.colored_vertex(centre, wash(color, peak));

    let mut prev: Option<(u32, usize)> = None;
    for k in 0..=rings {
        let t = k as f32 / rings as f32;
        let pts = ring(spread * t);
        if pts.len() < 3 {
            return;
        }
        let col = wash(color, (peak as f32 * falloff(t)) as u8);
        let base = mesh.vertices.len() as u32;
        for q in &pts {
            mesh.colored_vertex(*q, col);
        }
        let n = pts.len() as u32;
        match prev {
            None => {
                for i in 0..n {
                    mesh.add_triangle(0, base + i, base + (i + 1) % n);
                }
            }
            Some((pbase, pcount)) => {
                if pcount != pts.len() {
                    return;
                }
                for i in 0..n {
                    let j = (i + 1) % n;
                    mesh.add_triangle(pbase + i, base + i, base + j);
                    mesh.add_triangle(pbase + i, base + j, pbase + j);
                }
            }
        }
        prev = Some((base, pts.len()));
    }
    painter.add(Shape::mesh(mesh));
}

const BLOOM_SEGMENTS: usize = 44;

/// A round neon bloom.
pub fn glow_circle(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, |grow| {
        let rr = r + grow;
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                Pos2::new(centre.x + a.cos() * rr, centre.y + a.sin() * rr)
            })
            .collect()
    });
}

/// A neon bloom around a rounded rectangle.
pub fn glow_rect(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    let steps = 9usize;
    bloom_mesh(painter, rect.center(), spread, color, strength, move |grow| {
        let r = rect.expand(grow);
        let rad = (radius + grow).min(r.width() * 0.5).min(r.height() * 0.5);
        let quarter = std::f32::consts::FRAC_PI_2;
        let corners = [
            (Pos2::new(r.right() - rad, r.top() + rad), -quarter),
            (Pos2::new(r.right() - rad, r.bottom() - rad), 0.0),
            (Pos2::new(r.left() + rad, r.bottom() - rad), quarter),
            (Pos2::new(r.left() + rad, r.top() + rad), std::f32::consts::PI),
        ];
        let mut pts = Vec::with_capacity(steps * 4 + 4);
        for (c, a0) in corners {
            for i in 0..=steps {
                let a = a0 + quarter * (i as f32 / steps as f32);
                pts.push(Pos2::new(c.x + a.cos() * rad, c.y + a.sin() * rad));
            }
        }
        pts
    });
}

/// A bloom the shape of the rhombus it comes off, so the marker itself is the
/// light source rather than something with a lamp behind it. The halo rounds
/// off as it spreads, the way a real one does.
pub fn glow_diamond(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, move |grow| {
        let rr = r + grow;
        // a superellipse that starts as a diamond and relaxes towards a circle
        let n = 1.0 + 1.15 * (grow / spread.max(0.001)).clamp(0.0, 1.0);
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                let (c, s) = (a.cos(), a.sin());
                let d = (c.abs().powf(n) + s.abs().powf(n)).powf(1.0 / n);
                let k = rr / d.max(0.001);
                Pos2::new(centre.x + c * k, centre.y + s * k)
            })
            .collect()
    });
}
/// The four points of Northstar's star, clockwise from the top, with the
/// waist between each pair. `waist` is the inner radius as a fraction of `r`.
pub fn star_points(centre: Pos2, r: f32, waist: f32) -> [Pos2; 8] {
    let w = r * waist * std::f32::consts::FRAC_1_SQRT_2;
    [
        Pos2::new(centre.x, centre.y - r),
        Pos2::new(centre.x + w, centre.y - w),
        Pos2::new(centre.x + r, centre.y),
        Pos2::new(centre.x + w, centre.y + w),
        Pos2::new(centre.x, centre.y + r),
        Pos2::new(centre.x - w, centre.y + w),
        Pos2::new(centre.x - r, centre.y),
        Pos2::new(centre.x - w, centre.y - w),
    ]
}

/// A four-pointed star — Northstar's marker, where Tesseract has its rhombus —
/// filled with a gradient.
///
/// A star is not convex, so it is laid down as four kites (tip, waist, centre,
/// waist), each convex, each feathered, all sharing one gradient so the seams
/// never show.
pub fn grad_star(painter: &egui::Painter, centre: Pos2, r: f32, a: Color32, b: Color32) {
    for s in star_shapes(centre, r, 0.5, a, b) {
        painter.add(s);
    }
}

/// The same star as shapes, for slotting in under content laid out later.
pub fn star_shapes(centre: Pos2, r: f32, waist: f32, a: Color32, b: Color32) -> Vec<Shape> {
    let pts = star_points(centre, r, waist);
    // one gradient over the whole star, not four copies of it
    let dir = Vec2::new(0.6, 1.0).normalized();
    let (lo, hi) = (
        (centre.to_vec2() - Vec2::splat(r)).dot(dir),
        (centre.to_vec2() + Vec2::splat(r)).dot(dir),
    );
    let at = |p: Pos2| mix(a, b, (p.to_vec2().dot(dir) - lo) / (hi - lo).max(0.0001));
    (0..4)
        .map(|k| {
            let tip = pts[k * 2];
            let right = pts[k * 2 + 1];
            let left = pts[(k * 2 + 7) % 8];
            let kite = [tip, right, centre, left];
            let (ca, cb) = (at(tip), at(centre));
            // run each kite's own gradient between the colours its ends take
            // in the shared one
            let kd = centre - tip;
            let kd = if kd.length_sq() < 1e-6 { dir } else { kd };
            grad_poly_shape(&kite, ca, cb, kd)
        })
        .collect()
}

/// A bloom the shape of the four-pointed star it comes off, relaxing into a
/// circle as it spreads — the star is the light source, exactly as the
/// rhombus is in Tesseract.
pub fn glow_star(
    painter: &egui::Painter,
    centre: Pos2,
    r: f32,
    color: Color32,
    spread: f32,
    strength: f32,
) {
    bloom_mesh(painter, centre, spread, color, strength, move |grow| {
        let rr = r + grow;
        // n < 1 is a concave, four-pointed curve; by n = 2 it is a circle
        let n = 0.72 + 1.43 * (grow / spread.max(0.001)).clamp(0.0, 1.0);
        (0..BLOOM_SEGMENTS)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / BLOOM_SEGMENTS as f32;
                let (c, s) = (a.cos(), a.sin());
                let d = (c.abs().powf(n) + s.abs().powf(n)).powf(1.0 / n);
                let k = rr / d.max(0.001);
                Pos2::new(centre.x + c * k, centre.y + s * k)
            })
            .collect()
    });
}

/// The body of a pane of glass.
///
/// White-on-white is not frosted glass, it is nothing at all — so on a light
/// page the pane is tinted *down* towards the sunken tone instead of up
/// towards white, and it is the sheen and the rim that put the light back.
pub fn glass_body(opacity: f32) -> Color32 {
    let p = pal();
    let o = opacity.clamp(0.0, 1.0);
    if p.dark {
        wash(p.backdrop, (o * 205.0) as u8)
    } else {
        wash(p.sunken, (o * 232.0) as u8)
    }
}

/// What every control does when you reach for it.
///
/// The surface takes a *breath* of the accent — it does not become a different
/// colour — and only the outline goes bright. Anything more and a row of
/// buttons turns into a row of lamps.
pub fn hover_surface(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    accent: Color32,
    t: f32,
    on: bool,
) {
    let p = pal();
    let t = t.clamp(0.0, 1.0);
    if t <= 0.005 && !on {
        return;
    }
    let lit = t.max(if on { 1.0 } else { 0.0 });
    // The ground goes *down*, not up — but not to plain black. It sinks to a
    // well about as dark as the page itself, carrying the accent's own hue, so
    // the thing standing in it (a rhombus, a glyph) reads against something
    // that still belongs to the theme.
    let well = if p.dark {
        darken(accent, 0.88)
    } else {
        mix(p.sunken, darken(accent, 0.25), 0.35)
    };
    painter.rect_filled(
        rect,
        Rounding::same(radius),
        wash(well, ((if p.dark { 225.0 } else { 190.0 }) * lit) as u8),
    );
    painter.rect_stroke(
        rect.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(1.0_f32, wash(accent, (30.0 + 190.0 * lit) as u8)),
    );
}

/// A glassmorphic surface: a pane of frosted glass with light caught along its
/// top edge.
///
/// Three layers, in the order real glass gives you them — a dark translucent
/// body so text stays readable over whatever is behind, a sheen falling down
/// from the top, and a lit rim. Anything that floats above the page uses this:
/// alerts, questions, popovers, the index cards.
pub fn glass_surface(painter: &egui::Painter, rect: Rect, radius: f32, opacity: f32) {
    let p = pal();
    let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let o = opacity.clamp(0.0, 1.0);
    let pts = rounded_poly(rect, radius);

    painter.rect_filled(rect, Rounding::same(radius), glass_body(o));
    let sheen = Color32::WHITE;
    painter.add(grad_poly_shape(
        &pts,
        wash(sheen, (o * if p.dark { 22.0 } else { 130.0 }) as u8),
        wash(sheen, 0),
        Vec2::new(0.0, 1.0),
    ));

    painter.rect_stroke(
        rect.shrink(0.5),
        Rounding::same(radius),
        Stroke::new(
            1.0_f32,
            wash(sheen, (o * if p.dark { 26.0 } else { 140.0 }) as u8),
        ),
    );
}

/// Glass: a translucent plate with a soft lift down it, so it reads as a pane
/// of something rather than a flat rectangle.
pub fn glass_plate(painter: &egui::Painter, rect: Rect, radius: f32, opacity: f32) {
    let p = pal();
    // A pill radius is bigger than the plate; left unclamped it sends the edge
    // highlight off in the wrong direction and draws a line across the window.
    let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let base = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
    painter.rect_filled(rect, Rounding::same(radius), wash(p.glass, base));
    grad_rect(
        painter,
        rect.shrink(0.5),
        radius,
        wash(p.glass_hi, (base as f32 * 0.34) as u8),
        Color32::TRANSPARENT,
        Vec2::new(0.0, 1.0),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(radius),
        Stroke::new(1.0_f32, wash(p.glass_line, 110)),
    );
}

/// A solid island: opaque, rounded, hairlined, with a drop shadow so it lifts
/// off the blurred backdrop.
pub fn island_frame(radius: f32) -> egui::Frame {
    let p = pal();
    // No outline. An island is told apart from the backdrop by being opaque
    // and by the gap around it, never by a line drawn on the seam.
    egui::Frame::none()
        .fill(p.solid)
        .rounding(Rounding::same(radius))
        // Low and soft. A heavy drop shadow under a pane on glass reads as a
        // smudge, not as depth.
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 3.0),
            blur: 16.0,
            spread: 0.0,
            color: Color32::from_black_alpha(if p.dark { 48 } else { 22 }),
        })
}

/// Letter-spaced text, laid out as one run.
///
/// Placing glyphs one at a time — measuring each and adding the tracking by
/// hand — snaps every glyph to a whole pixel on its own and drops the font's
/// kerning, and the gaps between letters come out uneven. egui can space a
/// run itself; laid out once, the spacing is even at any size. Returns the
/// width drawn.
pub fn tracked_text(
    painter: &egui::Painter,
    left_centre: Pos2,
    text: &str,
    font: egui::FontId,
    color: Color32,
    tracking: f32,
) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: tracking,
            ..Default::default()
        },
    );
    let galley = painter.ctx().fonts(|f| f.layout_job(job));
    let size = galley.rect.size();
    painter.galley(
        Pos2::new(left_centre.x, left_centre.y - size.y * 0.5),
        galley,
        color,
    );
    // the tracking after the last letter is not part of the word
    (size.x - tracking).max(0.0)
}

/// A soft lift under something that floats: several shadows, each wider and
/// fainter than the last, rather than one hard one or a coloured glow — the
/// way light really falls off under a card held above a page. `lift` 0..1.
pub fn lift_shadow(painter: &egui::Painter, rect: Rect, radius: f32, lift: f32) {
    let p = pal();
    let lift = lift.clamp(0.0, 1.0);
    if lift <= 0.01 {
        return;
    }
    let rounding = Rounding::same(radius.min(rect.width() * 0.5).min(rect.height() * 0.5));
    let base = if p.dark { 1.0 } else { 0.32 };
    for (dy, blur, alpha) in [(1.0_f32, 2.0_f32, 60.0_f32), (4.0, 10.0, 38.0), (12.0, 28.0, 26.0)] {
        painter.add(
            egui::epaint::Shadow {
                offset: Vec2::new(0.0, dy * (0.5 + 0.5 * lift)),
                blur: blur * (0.6 + 0.4 * lift),
                spread: 0.0,
                color: Color32::from_black_alpha((alpha * base * lift) as u8),
            }
            .as_shape(rect, rounding),
        );
    }
}

// ------------------------------------------------------------------ visuals --

pub fn apply(ctx: &egui::Context) {
    let p = pal();
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    v.override_text_color = Some(p.text);
    v.panel_fill = Color32::TRANSPARENT;
    v.window_fill = p.solid;
    v.extreme_bg_color = p.sunken;
    v.faint_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0_f32, p.line);
    v.window_rounding = Rounding::same(R_PANEL);
    v.menu_rounding = Rounding::same(R_MD);
    v.selection.bg_fill = wash(p.primary, if p.dark { 70 } else { 60 });
    v.selection.stroke = Stroke::new(1.0_f32, p.primary);
    v.hyperlink_color = p.primary_light;
    v.popup_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 16.0),
        blur: 42.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if p.dark { 190 } else { 60 }),
    };
    v.window_shadow = v.popup_shadow;
    v.text_cursor.stroke = Stroke::new(2.0_f32, p.primary_light);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.solid;
    w.noninteractive.weak_bg_fill = p.solid;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_dim);
    w.noninteractive.rounding = Rounding::same(R_CTRL);

    w.inactive.bg_fill = p.raised;
    w.inactive.weak_bg_fill = p.solid_hi;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.inactive.rounding = Rounding::same(R_CTRL);

    w.hovered.bg_fill = p.solid_hi;
    w.hovered.weak_bg_fill = p.solid_hi;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, p.primary_deep);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.hovered.rounding = Rounding::same(R_CTRL);
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.primary_quiet_hi;
    w.active.weak_bg_fill = p.primary_quiet_hi;
    w.active.bg_stroke = Stroke::new(1.0_f32, p.primary);
    w.active.fg_stroke = Stroke::new(1.0_f32, p.primary_light);
    w.active.rounding = Rounding::same(R_CTRL);

    w.open.bg_fill = p.raised;
    w.open.weak_bg_fill = p.raised;
    w.open.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.open.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.open.rounding = Rounding::same(R_CTRL);

    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(11.0, 6.0);
        s.spacing.window_margin = Margin::same(18.0);
        s.spacing.menu_margin = Margin::same(6.0);
        s.spacing.scroll.bar_width = 8.0;
        s.spacing.scroll.floating = true;
        s.visuals.clip_rect_margin = 2.0;
    });
}


// ------------------------------------------------------------ script colours --


/// The accent an element wears in the gutter tag and the element bar. Taken
/// from the theme, never fixed, so every element follows the palette.
///
/// Seven elements, seven hues a theme already owns: the scene heading takes
/// the energy colour, a cue the accent, and the rest are spread across the
/// theme's group tints so no two elements that sit next to each other on a
/// page share a colour. Action — most of any script — stays neutral.
pub fn element_color(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::SceneHeading => p.sec_light,
        Element::Action => p.text_faint,
        Element::Character => p.primary_light,
        Element::Parenthetical => p.group(3),
        Element::Dialogue => p.group(2),
        Element::Transition => p.group(4),
        Element::Shot => p.group(5),
    }
}

/// The band behind a block in the editor, that tells one element from the
/// next at a glance. Faint enough to read through, never on the printed page.
pub fn element_band(e: Element) -> Color32 {
    let p = pal();
    match e {
        // action is most of any script: it carries no band at all
        Element::Action => Color32::TRANSPARENT,
        _ => tint(element_color(e), if p.dark { 0.035 } else { 0.05 }),
    }
}

/// `col` laid over whatever is under it at exactly `k` of its strength.
///
/// `wash` goes through egui's unmultiplied constructor, which premultiplies
/// in linear light — so a very low alpha on a bright colour comes out several
/// times stronger than the number says (alpha 6 on a pale colour lands near
/// 15%). For the faintest tints, the ones that must stay a whisper, the
/// colour is premultiplied here, in the same space it is blended in.
pub fn tint(col: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let f = |v: u8| (v as f32 * k).round() as u8;
    Color32::from_rgba_premultiplied(f(col.r()), f(col.g()), f(col.b()), (255.0 * k).round() as u8)
}

/// A colour of its own for each speaking character, in order of first
/// appearance — a golden-ratio walk round the hue circle, so neighbours are
/// far apart and a new character never repaints the ones before it. `seed`
/// turns the whole wheel, for Settings' Shuffle.
pub fn character_colors(names: &[String], seed: u32) -> std::collections::HashMap<String, Color32> {
    let p = pal();
    let (s, l) = if p.dark { (0.62, 0.72) } else { (0.66, 0.36) };
    voices_at(names, seed, s, l)
}

/// The same colours for paper: each speaker keeps their hue — so they are
/// recognisably the colour they wear in the app — at a depth that reads as
/// ink on white, whatever theme the app is in.
pub fn character_inks(names: &[String], seed: u32) -> std::collections::HashMap<String, [u8; 3]> {
    voices_at(names, seed, 0.70, 0.34)
        .into_iter()
        .map(|(n, c)| (n, [c.r(), c.g(), c.b()]))
        .collect()
}

fn voices_at(names: &[String], seed: u32, s: f32, l: f32) -> std::collections::HashMap<String, Color32> {
    let start = (seed as f32 * 0.137_5).fract();
    names
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let h = (start + k as f32 * 0.618_034).fract();
            (n.clone(), hsl(h, s, l))
        })
        .collect()
}

/// Hue, saturation, lightness, all 0..1, to a colour.
pub fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.fract() + 1.0).fract() * 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    let to = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    c3(to(r), to(g), to(b))
}

fn c3(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// The ink an element's text is set in on the page. Bodies sit a step down
/// from full-strength text, as they do in Tesseract; the cues and headings
/// that structure the page stay at full strength.
pub fn element_ink(e: Element) -> Color32 {
    let p = pal();
    match e {
        Element::SceneHeading | Element::Shot | Element::Character => p.text,
        Element::Dialogue => mix(p.text, p.text_dim, 0.15),
        Element::Action => mix(p.text, p.text_dim, 0.35),
        Element::Parenthetical | Element::Transition => p.text_dim,
    }
}

// -------------------------------------------------------------------- fonts --

/// Outfit ships with the app: the same geometric sans Tesseract is set in, so
/// the two read as one family on every machine. Four weights, so `strong`
/// really is heavier rather than just brighter.
const OUTFIT_REGULAR: &[u8] = include_bytes!("../assets/fonts/Outfit-Regular.ttf");
const OUTFIT_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Outfit-Medium.ttf");
const OUTFIT_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Outfit-SemiBold.ttf");
const OUTFIT_BOLD: &[u8] = include_bytes!("../assets/fonts/Outfit-Bold.ttf");

/// Courier Prime, the screenwriting standard, ships with the app too: the
/// page is the one thing whose geometry has to be exact, and a fallback
/// monospace with different metrics would change where lines wrap.
const PAGE_REGULAR: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Regular.ttf");
const PAGE_BOLD: &[u8] = include_bytes!("../assets/fonts/CourierPrime-Bold.ttf");

/// The name shown in Settings for the page font.
pub const PAGE_FONT_NAME: &str = "Courier Prime";

/// The heavier cuts, reachable from `FontId::new(size, family())`.
pub fn medium() -> FontFamily {
    FontFamily::Name("outfit-medium".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("outfit-semibold".into())
}
pub fn bold() -> FontFamily {
    FontFamily::Name("outfit-bold".into())
}
/// The screenplay page.
pub fn page() -> FontFamily {
    FontFamily::Name("page".into())
}
pub fn page_bold() -> FontFamily {
    FontFamily::Name("page-bold".into())
}

const MONO_CANDIDATES: &[&str] = &[
    "JetBrainsMono-Regular.ttf",
    "CascadiaCode-Regular.ttf",
    "FiraCode-Regular.ttf",
    "RobotoMono-Regular.ttf",
    "UbuntuMono-R.ttf",
    "LiberationMono-Regular.ttf",
    "DejaVuSansMono.ttf",
];

fn font_dirs() -> Vec<std::path::PathBuf> {
    let mut d = vec![
        std::path::PathBuf::from("/usr/share/fonts"),
        std::path::PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(home) = dirs::home_dir() {
        d.push(home.join(".local/share/fonts"));
        d.push(home.join(".fonts"));
    }
    d
}

fn find_font(candidates: &[&str]) -> Option<(String, Vec<u8>)> {
    let mut stack = font_dirs();
    let mut seen = 0usize;
    let mut best: Option<(usize, std::path::PathBuf)> = None;
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > 20_000 {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Some(rank) = candidates.iter().position(|c| c.eq_ignore_ascii_case(name)) {
                if best.as_ref().map(|(r, _)| rank < *r).unwrap_or(true) {
                    best = Some((rank, path.clone()));
                }
                if rank == 0 {
                    break;
                }
            }
        }
    }
    let (_, path) = best?;
    let bytes = std::fs::read(&path).ok()?;
    Some((path.file_stem()?.to_str()?.to_string(), bytes))
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    for (key, bytes) in [
        ("Outfit", OUTFIT_REGULAR),
        ("OutfitMedium", OUTFIT_MEDIUM),
        ("OutfitSemiBold", OUTFIT_SEMIBOLD),
        ("OutfitBold", OUTFIT_BOLD),
        ("CourierPrime", PAGE_REGULAR),
        ("CourierPrimeBold", PAGE_BOLD),
    ] {
        fonts
            .font_data
            .insert(key.to_string(), FontData::from_static(bytes));
    }

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Outfit".to_string());

    for (family, key) in [
        (medium(), "OutfitMedium"),
        (semibold(), "OutfitSemiBold"),
        (bold(), "OutfitBold"),
    ] {
        // each heavy cut falls back to the normal stack for glyphs it lacks
        let mut stack = vec![key.to_string()];
        stack.extend(
            fonts
                .families
                .get(&FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        );
        fonts.families.insert(family, stack);
    }

    if let Some((name, bytes)) = find_font(MONO_CANDIDATES) {
        fonts
            .font_data
            .insert(name.clone(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, name);
    }

    // the page: Courier Prime first, then the monospace stack for anything it
    // does not carry
    let mono = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    let mut page_stack = vec!["CourierPrime".to_string()];
    page_stack.extend(mono.iter().cloned());
    fonts.families.insert(page(), page_stack.clone());
    let mut bold_stack = vec!["CourierPrimeBold".to_string()];
    bold_stack.extend(page_stack);
    fonts.families.insert(page_bold(), bold_stack);

    ctx.set_fonts(fonts);
}

pub fn font(size: f32) -> egui::FontId {
    egui::FontId::proportional(size)
}
pub fn font_med(size: f32) -> egui::FontId {
    egui::FontId::new(size, medium())
}
pub fn font_semi(size: f32) -> egui::FontId {
    egui::FontId::new(size, semibold())
}
pub fn font_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, bold())
}
pub fn font_mono(size: f32) -> egui::FontId {
    egui::FontId::monospace(size)
}
pub fn font_page(size: f32) -> egui::FontId {
    egui::FontId::new(size, page())
}
pub fn font_page_bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, page_bold())
}
