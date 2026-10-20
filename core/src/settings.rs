//! The reader's settings: the agreed presets (shared with the Android app), their defaults, how they are stored in the
//! reading state, and the page style they make for a window. Shells keep only the themes' colours.

use std::io;

use crate::fonts::Family;
use crate::layout::PageStyle;
use crate::state::Store;

/// Theme names, in the order of the shells' palettes.
pub const THEMES: [&str; 6] = ["Light", "Sepia", "Dark", "Dawn", "Wave", "Forest"];
const LIGHT: usize = 0;
const DARK: usize = 2;

/// Text sizes in logical pixels: steps, not a slider.
pub const TEXT_SIZES: [f32; 7] = [16.0, 18.0, 20.0, 22.0, 24.0, 27.0, 30.0];
pub const DEFAULT_SIZE: usize = 2;

/// The narrowest text column, in em, of a page in a two-page spread (~55–60 characters): a full-screen 13" MacBook
/// (1440 px) holds two at the default size.
pub const MIN_SPREAD_COLUMN: f32 = 28.0;

/// Line spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spacing {
    Compact,
    #[default]
    Normal,
    Loose,
}

impl Spacing {
    pub const ALL: [Spacing; 3] = [Spacing::Compact, Spacing::Normal, Spacing::Loose];

    /// Line height in em.
    pub fn em(self) -> f32 {
        match self {
            Spacing::Compact => 1.45,
            Spacing::Normal => 1.62,
            Spacing::Loose => 1.85,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Spacing::Compact => "compact",
            Spacing::Normal => "normal",
            Spacing::Loose => "loose",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Justified,
    Left,
}

impl Align {
    pub const ALL: [Align; 2] = [Align::Justified, Align::Left];

    fn key(self) -> &'static str {
        match self {
            Align::Justified => "justified",
            Align::Left => "left",
        }
    }
}

/// How wide the text column may grow and the least side margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Margins {
    Narrow,
    #[default]
    Medium,
    Wide,
}

impl Margins {
    pub const ALL: [Margins; 3] = [Margins::Narrow, Margins::Medium, Margins::Wide];

    /// The widest column in em and the least side margin in pixels.
    fn column(self) -> (f32, f32) {
        match self {
            Margins::Narrow => (38.0, 24.0),
            Margins::Medium => (34.0, 48.0),
            Margins::Wide => (30.0, 72.0),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Margins::Narrow => "narrow",
            Margins::Medium => "medium",
            Margins::Wide => "wide",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Turning {
    #[default]
    Slide,
    Scroll,
}

impl Turning {
    pub const ALL: [Turning; 2] = [Turning::Slide, Turning::Scroll];

    fn key(self) -> &'static str {
        match self {
            Turning::Slide => "slide",
            Turning::Scroll => "scroll",
        }
    }
}

/// One setting, a control of the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Theme,
    Font,
    Size,
    Spacing,
    Align,
    Margins,
    Turning,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// An index in `THEMES`; `None` follows the system's light or dark appearance.
    pub theme: Option<usize>,
    /// An index in `TEXT_SIZES`.
    pub size: usize,
    pub family: Family,
    pub spacing: Spacing,
    pub align: Align,
    pub margins: Margins,
    pub turning: Turning,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: None,
            size: DEFAULT_SIZE,
            family: Family::default(),
            spacing: Spacing::default(),
            align: Align::default(),
            margins: Margins::default(),
            turning: Turning::default(),
        }
    }
}

/// The value of `key` among `all`, by its stored key; `None` for an unknown or missing value.
fn find<T: Copy>(all: &[T], key: Option<&str>, key_of: impl Fn(T) -> &'static str) -> Option<T> {
    let key = key?;
    all.iter().copied().find(|v| key_of(*v) == key)
}

impl Settings {
    /// The settings remembered in `store`; defaults for anything missing or unknown.
    pub fn load(store: &Store) -> Settings {
        let d = Settings::default();
        Settings {
            theme: store.setting("theme").and_then(|t| THEMES.iter().position(|n| *n == t)),
            size: store.setting("text-size").and_then(|px| TEXT_SIZES.iter().position(|s| s.to_string() == px)).unwrap_or(d.size),
            family: store.setting("font").and_then(Family::from_key).unwrap_or(d.family),
            spacing: find(&Spacing::ALL, store.setting("spacing"), Spacing::key).unwrap_or(d.spacing),
            align: find(&Align::ALL, store.setting("align"), Align::key).unwrap_or(d.align),
            margins: find(&Margins::ALL, store.setting("margins"), Margins::key).unwrap_or(d.margins),
            turning: find(&Turning::ALL, store.setting("turning"), Turning::key).unwrap_or(d.turning),
        }
    }

    /// Remembers the settings in `store` (one write). A theme following the system is not written.
    pub fn save(&self, store: &mut Store) -> io::Result<()> {
        let mut values = vec![
            ("text-size", TEXT_SIZES[self.size].to_string()),
            ("font", self.family.key().to_owned()),
            ("spacing", self.spacing.key().to_owned()),
            ("align", self.align.key().to_owned()),
            ("margins", self.margins.key().to_owned()),
            ("turning", self.turning.key().to_owned()),
        ];
        if let Some(t) = self.theme {
            values.push(("theme", THEMES[t].to_owned()));
        }
        store.set_settings(&values)
    }

    /// The theme shown: the chosen one, else Light or Dark as the system prefers.
    pub fn theme_index(&self, system_dark: bool) -> usize {
        self.theme.unwrap_or(if system_dark { DARK } else { LIGHT })
    }

    pub fn text_size(&self) -> f32 {
        TEXT_SIZES[self.size.min(TEXT_SIZES.len() - 1)]
    }

    /// Whether a window of `width` × `height` logical pixels shows two pages side by side: "Slide", wider than tall,
    /// and each half holds a column of `MIN_SPREAD_COLUMN` with the least side margins.
    pub fn spread(&self, width: f32, height: f32) -> bool {
        let (_, min_margin) = self.margins.column();
        self.turning == Turning::Slide && width > height && width / 2.0 - 2.0 * min_margin >= MIN_SPREAD_COLUMN * self.text_size()
    }

    /// The page for a window of `width` × `height` logical pixels: half the window in a spread.
    pub fn page_style(&self, width: f32, height: f32) -> PageStyle {
        let (measure, min_margin) = self.margins.column();
        let width = if self.spread(width, height) { width / 2.0 } else { width };
        PageStyle {
            line_height: self.spacing.em(),
            justify: self.align == Align::Justified,
            family: self.family,
            ..PageStyle::column(width, height, self.text_size(), measure, min_margin)
        }
    }

    /// A note in a panel `width` wide: the design's 17 px with 27 px lines next to an 18 px page, no margins (the
    /// panel has its own).
    pub fn note_style(&self, width: f32) -> PageStyle {
        let base = self.page_style(width, 100_000.0);
        PageStyle {
            text_size: base.text_size * 17.0 / 18.0,
            line_height: 27.0 / 17.0,
            margin_h: 0.0,
            margin_top: 0.0,
            margin_bottom: 4.0,
            ..base
        }
    }

    /// A table in the view at `zoom` times the text size: the page's family and line spacing, no margins.
    pub fn table_style(&self, zoom: f32) -> PageStyle {
        let base = self.page_style(100_000.0, 100_000.0);
        PageStyle { text_size: base.text_size * zoom, margin_h: 0.0, margin_top: 0.0, margin_bottom: 0.0, ..base }
    }

    /// Switches `setting` to the next (`delta` 1) or previous (−1) value. Sizes stop at the ends; the others go round.
    pub fn step(&mut self, setting: Setting, delta: i32, system_dark: bool) {
        fn round<T: Copy + PartialEq>(all: &[T], value: T, delta: i32) -> T {
            let i = all.iter().position(|v| *v == value).unwrap_or(0) as i32;
            all[(i + delta).rem_euclid(all.len() as i32) as usize]
        }
        match setting {
            Setting::Theme => {
                let i = self.theme_index(system_dark) as i32;
                self.theme = Some((i + delta).rem_euclid(THEMES.len() as i32) as usize);
            }
            Setting::Font => self.family = round(&Family::ALL, self.family, delta),
            Setting::Size => self.size = (self.size as i32 + delta).clamp(0, TEXT_SIZES.len() as i32 - 1) as usize,
            Setting::Spacing => self.spacing = round(&Spacing::ALL, self.spacing, delta),
            Setting::Align => self.align = round(&Align::ALL, self.align, delta),
            Setting::Margins => self.margins = round(&Margins::ALL, self.margins, delta),
            Setting::Turning => self.turning = round(&Turning::ALL, self.turning, delta),
        }
    }
}

#[cfg(test)]
mod tests;
