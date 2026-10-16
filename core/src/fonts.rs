//! Fonts: the bundled book face (PT Serif, OFL), fallback fonts for characters it lacks, and shaping with HarfRust.
//!
//! Fonts are numbered for the whole process: ids 0–3 are the bundled faces, later ids are fallback fonts registered
//! when a character first needs one. The typesetter shapes with these exact files and the shells rasterise the same
//! files (`font_source`), so glyph ids and advances agree on every platform.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use harfrust::{Buffer, Direction, Font, Script, ShapeOptions, ShapePlan, ShaperFont};
use read_fonts::TableProvider;
use read_fonts::types::{NameId, Tag};

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

    pub fn id(self) -> FontId {
        FontId(self as u16)
    }

    pub const FAMILY: &'static str = "PT Serif";
}

/// A font known to the process: a bundled face or a registered fallback font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(pub u16);

impl FontId {
    /// The bundled face, or `None` for a fallback font.
    pub fn face(self) -> Option<Face> {
        Face::ALL.get(self.0 as usize).copied()
    }
}

/// Where a font's file is, for a shell to load the same font.
#[derive(Debug, Clone, PartialEq)]
pub enum FontSource {
    Bundled(Face),
    File { path: PathBuf, index: u32 },
}

/// Finds a font file and face index that has a character; set by the shell from the platform's font system.
pub type FallbackProvider = Box<dyn Fn(char) -> Option<(PathBuf, u32)> + Send + Sync>;

struct Entry {
    font: &'static Font,
    source: FontSource,
    family: String,
    color: bool,
}

#[derive(Default)]
struct Registry {
    entries: Vec<Entry>,
    /// Fallback font for a character missing from the bundled faces; `None` when nothing has it.
    by_char: HashMap<char, Option<FontId>>,
    provider: Option<Arc<FallbackProvider>>,
}

/// The bundled faces, outside the registry's lock: nearly every lookup is for them.
fn bundled() -> &'static [Font; 4] {
    static BUNDLED: OnceLock<[Font; 4]> = OnceLock::new();
    BUNDLED.get_or_init(|| Face::ALL.map(|face| Font::new(face.data(), 0).expect("bundled font")))
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let entries = Face::ALL
            .iter()
            .zip(bundled())
            .map(|(&face, font)| Entry { font, source: FontSource::Bundled(face), family: Face::FAMILY.to_owned(), color: false })
            .collect();
        Mutex::new(Registry { entries, ..Registry::default() })
    })
}

/// Sets where fallback fonts come from. Characters already looked up keep their answer.
pub fn set_fallback_provider(provider: FallbackProvider) {
    registry().lock().unwrap().provider = Some(Arc::new(provider));
}

/// The file of `id`, for the shell.
pub fn font_source(id: FontId) -> Option<FontSource> {
    registry().lock().unwrap().entries.get(id.0 as usize).map(|e| e.source.clone())
}

/// The family name stored in the font, for shells that select fonts by name.
pub fn font_family(id: FontId) -> Option<String> {
    registry().lock().unwrap().entries.get(id.0 as usize).map(|e| e.family.clone())
}

/// Whether the font has colour glyphs (emoji): shells must draw them in colour.
pub fn font_is_color(id: FontId) -> bool {
    registry().lock().unwrap().entries.get(id.0 as usize).is_some_and(|e| e.color)
}

fn font(id: FontId) -> &'static Font {
    match id.face() {
        Some(face) => &bundled()[face as usize],
        None => registry().lock().unwrap().entries[id.0 as usize].font,
    }
}

fn covers(font: &Font, ch: char) -> bool {
    font.charmap().map_unicode(ch as u32).is_some()
}

/// The font to draw `ch` with when the text is set in `base`: `base` itself when it has the character, else a
/// fallback font that has it, else `base` (the character becomes a missing-glyph box).
pub fn font_for(base: FontId, ch: char) -> FontId {
    if covers(font(base), ch) || ch.is_control() {
        return base;
    }
    let (cached, provider) = {
        let reg = registry().lock().unwrap();
        (reg.by_char.get(&ch).copied(), reg.provider.clone())
    };
    if let Some(found) = cached {
        return found.unwrap_or(base);
    }
    // Already registered fallback fonts first, then the platform; the lock is not held while the provider runs.
    let known = {
        let reg = registry().lock().unwrap();
        reg.entries.iter().enumerate().skip(Face::ALL.len()).find(|(_, e)| covers(e.font, ch)).map(|(i, _)| FontId(i as u16))
    };
    let found = known.or_else(|| {
        let (path, index) = provider.and_then(|p| p(ch))?;
        let id = register(&path, index)?;
        covers(font(id), ch).then_some(id)
    });
    registry().lock().unwrap().by_char.insert(ch, found);
    found.unwrap_or(base)
}

/// Registers a font file (memory-mapped, not read), or returns the id it already has.
fn register(path: &Path, index: u32) -> Option<FontId> {
    let source = FontSource::File { path: path.to_owned(), index };
    if let Some(i) = registry().lock().unwrap().entries.iter().position(|e| e.source == source) {
        return Some(FontId(i as u16));
    }
    let file = std::fs::File::open(path).ok()?;
    // SAFETY: font files are not modified while the reader runs; a font changed underneath would at worst render
    // wrong glyphs, as with any application that maps its fonts.
    let map = unsafe { memmap2::Mmap::map(&file) }.ok()?;
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(map);
    let font: &'static Font = Box::leak(Box::new(Font::new(data, index)?));
    let tables = font.tables();
    let color = [b"COLR", b"CBDT", b"sbix", b"SVG "].iter().any(|t| tables.data_for_tag(Tag::new(t)).is_some());
    let family = font.names().get(NameId::FAMILY_NAME).english_or_first().map(|l| l.name().chars().collect()).unwrap_or_default();
    let mut reg = registry().lock().unwrap();
    reg.entries.push(Entry { font, source, family, color });
    Some(FontId((reg.entries.len() - 1) as u16))
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

/// A [`ShaperFont`] that can move to a layout thread.
struct ThreadShaper(ShaperFont<'static, 'static>);

// SAFETY: a `ShaperFont` is not `Send` only because it may hold custom font functions (`&dyn FontFuncs`); these are
// created with `ShaperFont::new` and never get any, and the rest is owned data over `&'static Font`, which is `Sync`.
unsafe impl Send for ThreadShaper {}

/// Shapes words with a plan cache per font and script, and caches shaped words: books repeat words a lot.
/// One per layout thread.
pub struct Shaper {
    shapers: HashMap<FontId, ThreadShaper>,
    plans: HashMap<(FontId, Option<Script>), ShapePlan>,
    words: HashMap<(FontId, String), Arc<[Glyph]>>,
    buffer: Option<Buffer>,
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

impl Shaper {
    pub fn new() -> Self {
        Self { shapers: HashMap::new(), plans: HashMap::new(), words: HashMap::new(), buffer: Some(Buffer::new()) }
    }

    pub fn units_per_em(&self, id: FontId) -> f32 {
        font(id).units_per_em() as f32
    }

    /// Ascender and descender (negative) in font units: typo metrics when the font asks for them, else hhea.
    pub fn line_metrics(&self, id: FontId) -> (f32, f32) {
        let m = font(id).metrics();
        let line = if m.use_typo_metrics { m.typo_line.or(m.hhea_line) } else { m.hhea_line.or(m.typo_line) };
        line.map(|l| (l.ascender.to_f32(), l.descender.to_f32())).unwrap_or((800.0, -200.0))
    }

    pub fn shape(&mut self, id: FontId, text: &str) -> Arc<[Glyph]> {
        if let Some(g) = self.words.get(&(id, text.to_owned())) {
            return g.clone();
        }
        let mut buffer = self.buffer.take().unwrap_or_default();
        buffer.clear();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        if buffer.direction() == Direction::Invalid {
            buffer.set_direction(Direction::LeftToRight);
        }
        let f = font(id);
        let plan =
            self.plans.entry((id, buffer.script())).or_insert_with(|| ShapePlan::new(f, buffer.direction(), buffer.script(), None, &[]));
        let shaper = &self.shapers.entry(id).or_insert_with(|| ThreadShaper(ShaperFont::new(f))).0;
        let _ = harfrust::shape(shaper, &mut buffer, ShapeOptions::new().plan(Some(plan)));
        let glyphs: Arc<[Glyph]> = buffer
            .glyph_infos()
            .iter()
            .zip(buffer.glyph_positions())
            .map(|(i, p)| Glyph { id: i.glyph_id, advance: p.x_advance, x_offset: p.x_offset, y_offset: p.y_offset, cluster: i.cluster })
            .collect();
        self.buffer = Some(buffer);
        self.words.insert((id, text.to_owned()), glyphs.clone());
        glyphs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typesetters_move_to_layout_threads() {
        fn send<T: Send>() {}
        send::<Shaper>();
        send::<crate::layout::Typesetter>();
        send::<crate::layout::ChapterLayout>();
    }

    #[test]
    fn shapes_cyrillic_with_byte_clusters() {
        let mut s = Shaper::new();
        let g = s.shape(Face::Regular.id(), "Гора");
        assert_eq!(g.len(), 4);
        assert!(g.iter().all(|g| g.id != 0), "no .notdef");
        assert_eq!(g.iter().map(|g| g.cluster).collect::<Vec<_>>(), [0, 2, 4, 6]);
        assert!(s.units_per_em(Face::Regular.id()) > 0.0);
    }

    #[test]
    fn covered_characters_stay_in_the_book_face() {
        assert_eq!(font_for(Face::Italic.id(), 'ж'), Face::Italic.id());
    }
}
