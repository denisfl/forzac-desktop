//! The bundled book face (PT Serif, OFL) and shaping with HarfRust.
//!
//! The typesetter shapes with these exact files and the shells rasterise the same files, so glyph ids and advances
//! agree on every platform and pages break identically on Linux and macOS.

use std::collections::HashMap;
use std::sync::OnceLock;

use harfrust::{Buffer, Direction, Font, Script, ShapeOptions, ShapePlan, ShaperFont};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Face {
    Regular = 0,
    Italic = 1,
    Bold = 2,
    BoldItalic = 3,
}

impl Face {
    pub const ALL: [Face; 4] = [Face::Regular, Face::Italic, Face::Bold, Face::BoldItalic];

    pub fn of(italic: bool, bold: bool) -> Face {
        match (italic, bold) {
            (false, false) => Face::Regular,
            (true, false) => Face::Italic,
            (false, true) => Face::Bold,
            (true, true) => Face::BoldItalic,
        }
    }

    pub fn is_italic(self) -> bool {
        matches!(self, Face::Italic | Face::BoldItalic)
    }

    pub fn is_bold(self) -> bool {
        matches!(self, Face::Bold | Face::BoldItalic)
    }

    /// The font file, for shells that rasterise it themselves.
    pub fn data(self) -> &'static [u8] {
        match self {
            Face::Regular => include_bytes!("../../assets/fonts/pt-serif/Regular.ttf"),
            Face::Italic => include_bytes!("../../assets/fonts/pt-serif/Italic.ttf"),
            Face::Bold => include_bytes!("../../assets/fonts/pt-serif/Bold.ttf"),
            Face::BoldItalic => include_bytes!("../../assets/fonts/pt-serif/BoldItalic.ttf"),
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Face::Regular => "Regular.ttf",
            Face::Italic => "Italic.ttf",
            Face::Bold => "Bold.ttf",
            Face::BoldItalic => "BoldItalic.ttf",
        }
    }

    pub const FAMILY: &'static str = "PT Serif";
}

fn fonts() -> &'static [Font; 4] {
    static FONTS: OnceLock<[Font; 4]> = OnceLock::new();
    FONTS.get_or_init(|| Face::ALL.map(|f| Font::new(f.data(), 0).expect("bundled font")))
}

/// A glyph in font units; `cluster` is the byte offset of its text in the shaped string.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub cluster: u32,
}

/// Shapes words with a plan cache per face and script, and caches shaped words: books repeat words a lot.
/// Not thread-safe; create one per layout thread.
pub struct Shaper {
    shapers: [ShaperFont<'static, 'static>; 4],
    plans: HashMap<(Face, Option<Script>), ShapePlan>,
    words: HashMap<(Face, String), std::rc::Rc<[Glyph]>>,
    buffer: Option<Buffer>,
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

impl Shaper {
    pub fn new() -> Self {
        let f = fonts();
        Self {
            shapers: [ShaperFont::new(&f[0]), ShaperFont::new(&f[1]), ShaperFont::new(&f[2]), ShaperFont::new(&f[3])],
            plans: HashMap::new(),
            words: HashMap::new(),
            buffer: Some(Buffer::new()),
        }
    }

    pub fn units_per_em(&self, face: Face) -> f32 {
        fonts()[face as usize].units_per_em() as f32
    }

    /// Ascender and descender (negative) in font units: typo metrics when the font asks for them, else hhea.
    pub fn line_metrics(&self, face: Face) -> (f32, f32) {
        let m = fonts()[face as usize].metrics();
        let line = if m.use_typo_metrics { m.typo_line.or(m.hhea_line) } else { m.hhea_line.or(m.typo_line) };
        line.map(|l| (l.ascender.to_f32(), l.descender.to_f32())).unwrap_or((800.0, -200.0))
    }

    pub fn shape(&mut self, face: Face, text: &str) -> std::rc::Rc<[Glyph]> {
        if let Some(g) = self.words.get(&(face, text.to_owned())) {
            return g.clone();
        }
        let mut buffer = self.buffer.take().unwrap_or_default();
        buffer.clear();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        if buffer.direction() == Direction::Invalid {
            buffer.set_direction(Direction::LeftToRight);
        }
        let key = (face, buffer.script());
        let plan = self
            .plans
            .entry(key)
            .or_insert_with(|| ShapePlan::new(&fonts()[face as usize], buffer.direction(), buffer.script(), None, &[]));
        let shaper = &self.shapers[face as usize];
        let _ = harfrust::shape(shaper, &mut buffer, ShapeOptions::new().plan(Some(plan)));
        let glyphs: std::rc::Rc<[Glyph]> = buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(i, p)| Glyph { id: i.glyph_id, advance: p.x_advance, x_offset: p.x_offset, y_offset: p.y_offset, cluster: i.cluster })
            .collect();
        self.buffer = Some(buffer);
        self.words.insert((face, text.to_owned()), glyphs.clone());
        glyphs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cyrillic_with_byte_clusters() {
        let mut s = Shaper::new();
        let g = s.shape(Face::Regular, "Гора");
        assert_eq!(g.len(), 4);
        assert!(g.iter().all(|g| g.id != 0), "no .notdef");
        assert_eq!(g.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 2, 4, 6]);
        assert!(s.units_per_em(Face::Regular) > 0.0);
    }
}
