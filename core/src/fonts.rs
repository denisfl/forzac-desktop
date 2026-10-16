//! Fonts: the book families (PT Serif embedded; Literata, Source Serif and PT Sans from the assets directory, all OFL),
//! PT Mono for code,
//! fallback fonts for characters a family lacks, and shaping with HarfRust.
//!
//! Fonts are numbered for the whole process: ids 0–3 are the embedded PT Serif faces, later ids are faces of the other
//! families (registered on first use) and fallback fonts (registered when a character first needs one). The typesetter
//! shapes with these exact files — a variable family's face at its location — and the shells rasterise the same files
//! at the same location (`font_source`), so glyph ids and advances agree on every platform.

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

/// A book font family, as offered in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    #[default]
    PtSerif,
    Literata,
    SourceSerif,
    PtSans,
}

impl Family {
    pub const ALL: [Family; 4] = [Family::PtSerif, Family::Literata, Family::SourceSerif, Family::PtSans];

    /// The name shown to the reader.
    pub fn name(self) -> &'static str {
        match self {
            Family::PtSerif => "PT Serif",
            Family::Literata => "Literata",
            Family::SourceSerif => "Source Serif",
            Family::PtSans => "PT Sans",
        }
    }

    /// The stored value and the directory in `assets/fonts`.
    pub fn key(self) -> &'static str {
        match self {
            Family::PtSerif => "pt-serif",
            Family::Literata => "literata",
            Family::SourceSerif => "source-serif",
            Family::PtSans => "pt-sans",
        }
    }

    pub fn from_key(key: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|f| f.key() == key)
    }

    /// The file of `face` and its weight on the `wght` axis (variable families keep a roman and an italic file).
    fn file(self, face: Face) -> (&'static str, f32) {
        let weight = if face.is_bold() { 700.0 } else { 400.0 };
        match (self, face.is_italic()) {
            (Family::Literata, false) => ("Literata.ttf", weight),
            (Family::Literata, true) => ("Literata-Italic.ttf", weight),
            (Family::SourceSerif, false) => ("SourceSerif4.ttf", weight),
            (Family::SourceSerif, true) => ("SourceSerif4-Italic.ttf", weight),
            _ => (face.file_name(), weight),
        }
    }

    /// The font of `face` in this family; PT Serif's when the family's files cannot be read.
    pub fn face(self, face: Face) -> FontId {
        if self == Family::PtSerif {
            return face.id();
        }
        static FACES: OnceLock<Mutex<HashMap<(Family, Face), FontId>>> = OnceLock::new();
        let faces = FACES.get_or_init(Default::default);
        if let Some(&id) = faces.lock().unwrap().get(&(self, face)) {
            return id;
        }
        let (file, weight) = self.file(face);
        let id = assets()
            .and_then(|dir| register_instance(&dir.join("fonts").join(self.key()).join(file), 0, Some(weight), Some(face)))
            .unwrap_or(face.id());
        faces.lock().unwrap().insert((self, face), id);
        id
    }
}

/// The monospace face for code (PT Mono Regular from the assets); `None` when its file cannot be read. It has one
/// style: code in emphasis or strong stays regular rather than synthesised.
pub fn mono() -> Option<FontId> {
    static MONO: OnceLock<Option<FontId>> = OnceLock::new();
    *MONO.get_or_init(|| register_instance(&assets()?.join("fonts/pt-mono/Regular.ttf"), 0, None, Some(Face::Regular)))
}

static ASSETS: OnceLock<PathBuf> = OnceLock::new();

/// Where the bundled assets are (`fonts/<family>/…`); set once by the shell before typesetting.
pub fn set_assets(dir: PathBuf) {
    let _ = ASSETS.set(dir);
}

fn assets() -> Option<&'static PathBuf> {
    ASSETS.get()
}

/// A font known to the process: an embedded PT Serif face, a face of another family or a fallback font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(pub u16);

impl FontId {
    /// The bundled face, or `None` for a fallback font.
    pub fn face(self) -> Option<Face> {
        Face::ALL.get(self.0 as usize).copied()
    }
}

/// Where a font's file is, for a shell to load the same font: for a variable font, at which location (every axis, by
/// tag, in user space); empty for a static one.
#[derive(Debug, Clone, PartialEq)]
pub enum FontSource {
    Bundled(Face),
    File { path: PathBuf, index: u32, variations: Vec<(String, f32)> },
}

/// Finds a font file and face index that has a character; set by the shell from the platform's font system.
pub type FallbackProvider = Box<dyn Fn(char) -> Option<(PathBuf, u32)> + Send + Sync>;

struct Entry {
    font: &'static Font,
    source: FontSource,
    family: String,
    /// The style within a book family; `None` for a fallback font.
    face: Option<Face>,
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
            .map(|(&face, font)| Entry {
                font,
                source: FontSource::Bundled(face),
                family: Face::FAMILY.to_owned(),
                face: Some(face),
                color: false,
            })
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

/// The style of a book family's face (`None` for a fallback font), for shells that select fonts by style.
pub fn font_style(id: FontId) -> Option<Face> {
    registry().lock().unwrap().entries.get(id.0 as usize).and_then(|e| e.face)
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
        let id = register_instance(&path, index, None, None)?;
        covers(font(id), ch).then_some(id)
    });
    registry().lock().unwrap().by_char.insert(ch, found);
    found.unwrap_or(base)
}

/// Registers a font file (memory-mapped, not read), or returns the id it already has. A variable font is registered
/// at `weight` on its `wght` axis and the default of every other axis; `face` is its style in a book family.
fn register_instance(path: &Path, index: u32, weight: Option<f32>, face: Option<Face>) -> Option<FontId> {
    let file = std::fs::File::open(path).ok()?;
    // SAFETY: font files are not modified while the reader runs; a font changed underneath would at worst render
    // wrong glyphs, as with any application that maps its fonts.
    let map = unsafe { memmap2::Mmap::map(&file) }.ok()?;
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(map);
    let base = Font::new(data, index)?;
    // Every axis explicitly, so the shell renders exactly this location (no automatic optical size).
    let variations: Vec<(String, f32)> = base
        .tables()
        .fvar()
        .ok()
        .and_then(|fvar| fvar.axes().ok().map(|axes| axes.to_vec()))
        .unwrap_or_default()
        .iter()
        .map(|axis| {
            let tag = axis.axis_tag();
            let value =
                if tag == Tag::new(b"wght") { weight.unwrap_or(axis.default_value().to_f32()) } else { axis.default_value().to_f32() };
            (tag.to_string(), value)
        })
        .collect();
    let source = FontSource::File { path: path.to_owned(), index, variations: variations.clone() };
    if let Some(i) = registry().lock().unwrap().entries.iter().position(|e| e.source == source) {
        return Some(FontId(i as u16));
    }
    let font = if variations.is_empty() {
        base
    } else {
        base.instance_builder()
            .variations(variations.iter().map(|(t, v)| (Tag::new_checked(t.as_bytes()).unwrap_or(Tag::new(b"wght")), *v)))
            .build()
    };
    let font: &'static Font = Box::leak(Box::new(font));
    let tables = font.tables();
    let color = [b"COLR", b"CBDT", b"sbix", b"SVG "].iter().any(|t| tables.data_for_tag(Tag::new(t)).is_some());
    let family = font.names().get(NameId::FAMILY_NAME).english_or_first().map(|l| l.name().chars().collect()).unwrap_or_default();
    let mut reg = registry().lock().unwrap();
    reg.entries.push(Entry { font, source, family, face, color });
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
    /// Fonts by id, so registered fonts do not take the registry's lock on every lookup.
    fonts: HashMap<FontId, &'static Font>,
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
        Self { fonts: HashMap::new(), shapers: HashMap::new(), plans: HashMap::new(), words: HashMap::new(), buffer: Some(Buffer::new()) }
    }

    fn font(&mut self, id: FontId) -> &'static Font {
        self.fonts.entry(id).or_insert_with(|| font(id))
    }

    pub fn units_per_em(&mut self, id: FontId) -> f32 {
        self.font(id).units_per_em() as f32
    }

    /// Ascender and descender (negative) in font units: typo metrics when the font asks for them, else hhea.
    pub fn line_metrics(&mut self, id: FontId) -> (f32, f32) {
        let m = self.font(id).metrics();
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
        let f = self.font(id);
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
    fn families_shape_with_their_own_fonts_and_weights() {
        set_assets(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")));
        let mut s = Shaper::new();
        let width = |s: &mut Shaper, id: FontId| {
            let scale = 20.0 / s.units_per_em(id);
            s.shape(id, "Широкая строка Wide line").iter().map(|g| g.advance as f32 * scale).sum::<f32>()
        };
        let pt_serif = width(&mut s, Family::PtSerif.face(Face::Regular));
        for family in [Family::Literata, Family::SourceSerif, Family::PtSans] {
            let (regular, bold) = (family.face(Face::Regular), family.face(Face::Bold));
            assert!(regular.face().is_none() && regular != bold, "{family:?} has fonts of its own");
            assert_eq!(font_style(bold), Some(Face::Bold));
            let (r, b) = (width(&mut s, regular), width(&mut s, bold));
            assert!(b > r, "{family:?}: bold {b} wider than regular {r}"); // PT Sans Bold is barely wider
            assert!((r - pt_serif).abs() > 1.0, "{family:?} is not PT Serif");
            assert!(s.shape(regular, "Гора").iter().all(|g| g.id != 0), "{family:?} has Cyrillic");
        }
        // A variable family's face is the file at a location, every axis named.
        let Some(FontSource::File { variations, .. }) = font_source(Family::Literata.face(Face::Bold)) else { panic!() };
        assert!(
            variations.iter().any(|(t, v)| t == "wght" && *v == 700.0) && variations.iter().any(|(t, _)| t == "opsz"),
            "{variations:?}"
        );
        assert_eq!(Family::Literata.face(Face::Bold), Family::Literata.face(Face::Bold), "registered once");
    }

    #[test]
    fn covered_characters_stay_in_the_book_face() {
        assert_eq!(font_for(Face::Italic.id(), 'ж'), Face::Italic.id());
    }
}
