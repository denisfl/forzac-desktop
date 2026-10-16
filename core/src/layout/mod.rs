//! Typesetting: blocks → lines (Knuth–Plass with Liang hyphenation over HarfRust-shaped words) → pages → page contents
//! (positioned glyph runs, images, table rules, active areas) that a shell draws as is.
//!
//! Rules follow the Android app: 1.5 em first-line indent, justified body text with hyphenation, ragged verse and
//! headings without it, fixed line height for vertical rhythm, no lone line at the top or bottom of a page, a heading
//! kept with the next block, the chapter start lowered by two lines, images and table rows never split.

mod page;
mod table;

use std::collections::HashMap;
use std::sync::Arc;

pub use page::{ChapterLayout, Page, PageContent, PageImage, Rule, layout_note, paginate};

use crate::fonts::{self, Face, Family, FontId, Glyph, Shaper};
use crate::hyphen::Hyphenator;
use crate::knuth_plass::{self, INFINITY, Item};
use crate::model::{Block, Book, Kind, Style, StyledText};

/// Page geometry and text parameters, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageStyle {
    pub width: f32,
    pub height: f32,
    pub margin_h: f32,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub text_size: f32,
    /// Line height of body text in em.
    pub line_height: f32,
    pub justify: bool,
    pub family: Family,
}

impl PageStyle {
    /// The default page for a window: a column of at most 34 em (about 65 characters), centred, in PT Serif with
    /// the normal line spacing (the default settings, see `settings`).
    pub fn for_window(width: f32, height: f32, text_size: f32) -> Self {
        Self::column(width, height, text_size, 34.0, 48.0)
    }

    /// A page with the text column at most `measure` em wide and side margins of at least `min_margin`.
    pub fn column(width: f32, height: f32, text_size: f32, measure: f32, min_margin: f32) -> Self {
        let margin_h = min_margin.max((width - text_size * measure) / 2.0);
        PageStyle {
            width,
            height,
            margin_h,
            margin_top: 56.0,
            margin_bottom: 56.0,
            text_size,
            line_height: 1.62,
            justify: true,
            family: Family::PtSerif,
        }
    }

    pub fn text_width(&self) -> f32 {
        self.width - 2.0 * self.margin_h
    }

    pub fn text_height(&self) -> f32 {
        self.height - self.margin_top - self.margin_bottom
    }
}

/// Which theme colour a run is drawn in; the shell maps it, so typesetting does not depend on the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Ink {
    Text = 0,
    Link = 1,
    Secondary = 2,
}

/// What an active area on a page leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A note id in `Book::notes`.
    Note(String),
    /// A key in `Book::links`.
    Link(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    fn offset(self, dx: f32, dy: f32) -> Rect {
        Rect { x: self.x + dx, y: self.y + dy, ..self }
    }
}

/// Glyphs of one font, size and ink. Positions are glyph origins on the baseline, y down.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRun {
    pub font: FontId,
    pub size: f32,
    pub ink: Ink,
    pub glyphs: Vec<u16>,
    pub positions: Vec<[f32; 2]>,
}

impl GlyphRun {
    fn offset(&self, dx: f32, dy: f32) -> GlyphRun {
        GlyphRun { positions: self.positions.iter().map(|p| [p[0] + dx, p[1] + dy]).collect(), ..self.clone() }
    }
}

/// A note marker or a link on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    pub rect: Rect,
    pub target: Target,
}

/// Blocks of one group (verse lines, epigraph paragraphs, rows of one table) go without spacing between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Kind(Kind),
    EmptyLine,
    /// Rows of the table at this address.
    Table(usize),
    /// Never shares spacing with a neighbour (images).
    Alone,
}

/// One line: its runs and active areas relative to the line's left edge and baseline.
#[derive(Debug, Clone, Default)]
struct Line {
    runs: Vec<GlyphRun>,
    areas: Vec<Area>,
}

/// Lines of a paragraph set at one width.
struct TextLayout {
    lines: Vec<Line>,
    /// Byte offset in the block text where each line starts: page start → `Locator`.
    starts: Vec<usize>,
    line_height: f32,
    baseline: f32,
}

impl TextLayout {
    fn height(&self) -> f32 {
        self.lines.len() as f32 * self.line_height
    }
}

/// A cell of a table row, its top-left corner relative to the row.
struct Cell {
    text: TextLayout,
    x: f32,
    y: f32,
}

enum Content {
    Text(TextLayout),
    Empty,
    Image {
        id: String,
        w: f32,
        h: f32,
    },
    /// Rule strengths: 0 none, 1 light between rows, 2 medium under the header, 3 heavy above and below the table.
    Row {
        cells: Vec<Cell>,
        rules: (u8, u8),
        height: f32,
        width: f32,
    },
}

/// A typeset block.
pub struct BlockLayout {
    content: Content,
    x: f32,
    group: Group,
    space_before: f32,
    space_after: f32,
    keep_with_next: bool,
}

impl BlockLayout {
    fn height(&self) -> f32 {
        match &self.content {
            Content::Text(t) => t.height(),
            Content::Empty => 0.0,
            Content::Image { h, .. } => *h,
            Content::Row { height, .. } => *height,
        }
    }

    /// Images and table rows are never split between pages.
    fn atomic(&self) -> bool {
        matches!(self.content, Content::Image { .. } | Content::Row { .. })
    }

    fn line_count(&self) -> usize {
        match &self.content {
            Content::Text(t) => t.lines.len(),
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

/// What an item of the paragraph stands for, beyond its width.
#[derive(Clone, Copy)]
enum Meta {
    Plain,
    Indent,
    /// Text `[start, end)` of the paragraph.
    Text(usize, usize),
    /// A hyphenation point: the hyphen is drawn in the style of this piece when the line breaks here.
    Hyphen(usize),
}

/// A word, or the part of it in one style and one font, shaped as a whole: kerning inside the word is kept across
/// syllables.
struct Piece {
    start: usize,
    font: FontId,
    size: f32,
    rise: f32,
    scale: f32,
    /// Ascender and descender in pixels, for the active area of a marker or link.
    ascent: f32,
    descent: f32,
    segment: usize,
    glyphs: Arc<[Glyph]>,
}

/// A run of one style: face, size, raise, ink and target.
struct Segment {
    start: usize,
    end: usize,
    face: Face,
    size: f32,
    rise: f32,
    ink: Ink,
    target: Option<Target>,
}

#[derive(Clone, Copy)]
struct Spec {
    width: f32,
    x: f32,
    align: Align,
    justify: bool,
    hyphenate: bool,
    group: Group,
    space_before: f32,
    space_after: f32,
    indent: bool,
    keep_with_next: bool,
    italic: bool,
    bold: bool,
    scale: f32,
    line_scale: f32,
}

/// Lays out blocks for one page style. Holds the shaping caches; one per thread.
pub struct Typesetter {
    style: PageStyle,
    shaper: Shaper,
    hyphenator: Option<Arc<Hyphenator>>,
    /// Column geometry per table, by the table's address: computed once per table.
    tables: HashMap<usize, Arc<table::Geometry>>,
    /// Body line height: the unit of vertical spacing.
    pub line_height: f32,
    /// The fonts of the four faces in the style's family.
    faces: [FontId; 4],
}

impl Typesetter {
    pub fn new(style: PageStyle, hyphenator: Option<Arc<Hyphenator>>) -> Self {
        let line_height = (style.text_size * style.line_height).floor();
        let faces = Face::ALL.map(|f| style.family.face(f));
        Typesetter { style, shaper: Shaper::new(), hyphenator, tables: HashMap::new(), line_height, faces }
    }

    /// The font of `face` in the style's family.
    fn font_of(&self, face: Face) -> FontId {
        self.faces[face as usize]
    }

    pub fn style(&self) -> &PageStyle {
        &self.style
    }

    pub fn hyphenator(&self) -> Option<Arc<Hyphenator>> {
        self.hyphenator.clone()
    }

    fn base_spec(&self) -> Spec {
        Spec {
            width: self.style.text_width(),
            x: 0.0,
            align: Align::Left,
            justify: false,
            hyphenate: true,
            group: Group::Kind(Kind::Body),
            space_before: 0.0,
            space_after: 0.0,
            indent: false,
            keep_with_next: false,
            italic: false,
            bold: false,
            scale: 1.0,
            line_scale: 1.0,
        }
    }

    /// `None` for a block that takes no room: an image that is missing or cannot be read.
    pub fn layout(&mut self, block: &Block, book: &Book) -> Option<BlockLayout> {
        let w = self.style.text_width();
        let lh = self.line_height;
        let half = lh / 2.0;
        let base = self.base_spec();
        let justify = self.style.justify;
        match block {
            Block::EmptyLine => Some(BlockLayout {
                content: Content::Empty,
                x: 0.0,
                group: Group::EmptyLine,
                space_before: 0.0,
                space_after: 0.0,
                keep_with_next: false,
            }),
            Block::Image { id } => self.image(id, book),
            Block::TableRow { table, row } => Some(self.table_row(table, *row)),
            Block::Heading { text, level } => Some(self.make(
                text,
                Spec {
                    align: Align::Center,
                    hyphenate: false,
                    group: Group::Kind(Kind::Subtitle),
                    space_before: lh,
                    space_after: lh,
                    keep_with_next: true,
                    bold: true,
                    scale: if *level <= 1 { 1.35 } else { 1.15 },
                    line_scale: 1.15 / self.style.line_height,
                    ..base
                },
            )),
            Block::Paragraph { text, kind } => {
                let spec = match kind {
                    Kind::Body => Spec { justify, indent: true, ..base },
                    Kind::Cite => Spec {
                        width: w * 0.92,
                        x: w * 0.08,
                        justify,
                        indent: true,
                        group: Group::Kind(Kind::Cite),
                        space_before: half,
                        space_after: half,
                        ..base
                    },
                    Kind::Epigraph => Spec {
                        width: w * 0.7,
                        x: w * 0.3,
                        group: Group::Kind(Kind::Epigraph),
                        space_before: half,
                        italic: true,
                        scale: 0.92,
                        ..base
                    },
                    Kind::TextAuthor => {
                        Spec { align: Align::Right, group: Group::Kind(Kind::Epigraph), space_after: lh, scale: 0.92, ..base }
                    }
                    Kind::Verse => Spec {
                        width: w * 0.88,
                        x: w * 0.12,
                        hyphenate: false,
                        group: Group::Kind(Kind::Verse),
                        space_before: half,
                        space_after: half,
                        ..base
                    },
                    Kind::Subtitle => Spec {
                        align: Align::Center,
                        hyphenate: false,
                        group: Group::Kind(Kind::Subtitle),
                        space_before: lh,
                        space_after: half,
                        keep_with_next: true,
                        bold: true,
                        ..base
                    },
                    Kind::Code => Spec {
                        hyphenate: false,
                        group: Group::Kind(Kind::Code),
                        space_before: half,
                        space_after: half,
                        scale: 0.85,
                        ..base
                    },
                };
                Some(self.make(text, spec))
            }
        }
    }

    /// One logical pixel per image pixel, scaled down to the column and the page; never split.
    fn image(&mut self, id: &str, book: &Book) -> Option<BlockLayout> {
        let image = book.images.get(id)?;
        let size = imagesize::blob_size(&image.bytes).ok()?;
        let (iw, ih) = (size.width as f32, size.height as f32);
        if iw <= 0.0 || ih <= 0.0 {
            return None;
        }
        let scale = 1f32.min(self.style.text_width() / iw).min(self.style.text_height() / ih);
        let (w, h) = (iw * scale, ih * scale);
        let half = self.line_height / 2.0;
        Some(BlockLayout {
            content: Content::Image { id: id.to_owned(), w, h },
            x: (self.style.text_width() - w) / 2.0,
            group: Group::Alone,
            space_before: half,
            space_after: half,
            keep_with_next: false,
        })
    }

    fn make(&mut self, text: &StyledText, spec: Spec) -> BlockLayout {
        let layout = self.text(text, &spec);
        BlockLayout {
            content: Content::Text(layout),
            x: spec.x,
            group: spec.group,
            space_before: spec.space_before,
            space_after: spec.space_after,
            keep_with_next: spec.keep_with_next,
        }
    }

    /// A paragraph set into lines of `spec.width`.
    fn text(&mut self, text: &StyledText, spec: &Spec) -> TextLayout {
        let size = self.style.text_size * spec.scale;
        let segments = segments(text, size, spec.italic, spec.bold);
        let indent = if spec.indent { self.style.text_size * 1.5 } else { 0.0 };
        let (items, meta, pieces) = self.items(&text.text, &segments, indent, spec.justify, spec.hyphenate);
        let breaks = knuth_plass::break_lines(&items, |_| spec.width);
        let (lines, starts) = self.lines(&items, &meta, &pieces, &segments, &breaks, spec);

        let line_height = (size * self.style.line_height * spec.line_scale).floor();
        let face = self.font_of(segments.first().map(|s| s.face).unwrap_or(Face::Regular));
        let (asc, desc) = self.shaper.line_metrics(face);
        let units = size / self.shaper.units_per_em(face);
        let baseline = (line_height - (asc - desc) * units) / 2.0 + asc * units;
        TextLayout { lines, starts, line_height, baseline }
    }

    /// The paragraph as boxes (syllables), glue (spaces) and penalties (hyphenation points, breaks after dashes).
    fn items(
        &mut self,
        text: &str,
        segments: &[Segment],
        indent: f32,
        justify: bool,
        hyphenate: bool,
    ) -> (Vec<Item>, Vec<Meta>, Vec<Piece>) {
        let mut items = Vec::new();
        let mut meta = Vec::new();
        let mut pieces = Vec::new();
        if indent > 0.0 {
            items.push(Item::Box { width: indent });
            meta.push(Meta::Indent);
        }
        let mut i = 0;
        while i < text.len() {
            let c = text[i..].chars().next().unwrap();
            if c == '\n' {
                items.push(Item::Glue { width: 0.0, stretch: 100_000.0, shrink: 0.0 });
                items.push(Item::Penalty { width: 0.0, penalty: -INFINITY, flagged: false });
                meta.extend([Meta::Plain, Meta::Plain]);
                i += 1;
                continue;
            }
            if c == ' ' {
                let seg = &segments[segment_at(segments, i)];
                let space = self.width_of(self.font_of(seg.face), " ", seg.size);
                if justify {
                    items.push(Item::Glue { width: space, stretch: space * 0.5, shrink: space * 0.33 });
                    meta.push(Meta::Plain);
                } else {
                    // Ragged right (Knuth): the stretch lives in the break, so lines keep natural spacing.
                    items.push(Item::Glue { width: 0.0, stretch: space * 3.0, shrink: 0.0 });
                    items.push(Item::Penalty { width: 0.0, penalty: 0.0, flagged: false });
                    items.push(Item::Glue { width: space, stretch: -space * 3.0, shrink: 0.0 });
                    meta.extend([Meta::Plain, Meta::Plain, Meta::Plain]);
                }
                i += 1;
                continue;
            }
            let end = text[i..].find([' ', '\n']).map_or(text.len(), |e| i + e);
            let penalty = if justify { 50.0 } else { 300.0 };
            self.word(text, segments, (i, end), hyphenate, penalty, &mut items, &mut meta, &mut pieces);
            i = end;
        }
        // End of paragraph: the last line is not stretched.
        items.push(Item::Penalty { width: 0.0, penalty: INFINITY, flagged: false });
        items.push(Item::Glue { width: 0.0, stretch: 100_000.0, shrink: 0.0 });
        items.push(Item::Penalty { width: 0.0, penalty: -INFINITY, flagged: false });
        meta.extend([Meta::Plain, Meta::Plain, Meta::Plain]);
        (items, meta, pieces)
    }

    /// Pieces of the word `[start, end)`: one per style segment and font. Characters the book face lacks are shaped
    /// with a fallback font (`fonts::font_for`).
    fn word_pieces(&mut self, text: &str, segments: &[Segment], (start, end): (usize, usize), pieces: &mut Vec<Piece>) {
        for (index, seg) in segments.iter().enumerate().filter(|(_, s)| s.start < end && s.end > start) {
            let (a, b) = (seg.start.max(start), seg.end.min(end));
            let base = self.font_of(seg.face);
            // Nearly always the book face has every character: no glyph 0 (.notdef), and the shape is cached.
            let whole = self.piece(text, a, b, base, seg, index);
            if whole.glyphs.iter().all(|g| g.id != 0) {
                pieces.push(whole);
                continue;
            }
            let mut run_start = a;
            let mut run_font = None;
            for (offset, ch) in text[a..b].char_indices().chain([(b - a, '\0')]) {
                let at = a + offset;
                let font = if at == b { None } else { Some(fonts::font_for(base, ch)) };
                if font != run_font {
                    if let Some(f) = run_font.filter(|_| at > run_start) {
                        pieces.push(self.piece(text, run_start, at, f, seg, index));
                    }
                    run_start = at;
                    run_font = font;
                }
            }
        }
    }

    fn piece(&mut self, text: &str, a: usize, b: usize, font: FontId, seg: &Segment, segment: usize) -> Piece {
        let glyphs = self.shaper.shape(font, &text[a..b]);
        let scale = seg.size / self.shaper.units_per_em(font);
        let (asc, desc) = self.shaper.line_metrics(font);
        Piece { start: a, font, size: seg.size, rise: seg.rise, scale, ascent: asc * scale, descent: -desc * scale, segment, glyphs }
    }

    #[allow(clippy::too_many_arguments)]
    fn word(
        &mut self,
        text: &str,
        segments: &[Segment],
        (start, end): (usize, usize),
        hyphenate: bool,
        penalty: f32,
        items: &mut Vec<Item>,
        meta: &mut Vec<Meta>,
        pieces: &mut Vec<Piece>,
    ) {
        let first_piece = pieces.len();
        self.word_pieces(text, segments, (start, end), pieces);
        let word_pieces = &pieces[first_piece..];

        // Break points inside the word.
        let mut cuts: Vec<(usize, Cut)> = Vec::new();
        let chars: Vec<(usize, char)> = text[start..end].char_indices().map(|(o, c)| (start + o, c)).collect();
        let mut k = 0;
        while k < chars.len() {
            if chars[k].1.is_alphabetic() {
                let run_start = k;
                while k < chars.len() && chars[k].1.is_alphabetic() {
                    k += 1;
                }
                if let (true, Some(h)) = (hyphenate && k - run_start >= 5, &self.hyphenator) {
                    let letters: Vec<char> = chars[run_start..k].iter().map(|(_, c)| *c).collect();
                    cuts.extend(h.hyphenate(&letters).into_iter().map(|p| (chars[run_start + p].0, Cut::Hyphen)));
                }
            } else {
                if matches!(chars[k].1, '-' | '—') && k > 0 && k + 1 < chars.len() {
                    cuts.push((chars[k + 1].0, Cut::Break));
                }
                k += 1;
            }
        }
        // Chinese and Japanese have no spaces: a line may break between characters, but not before closing
        // punctuation or after opening punctuation (kinsoku).
        for pair in chars.windows(2) {
            let (a, b) = (pair[0].1, pair[1].1);
            if (is_ideographic(a) || is_ideographic(b)) && !NO_LINE_START.contains(b) && !NO_LINE_END.contains(a) {
                cuts.push((pair[1].0, Cut::Ideographic));
            }
        }
        // Only where a glyph cluster starts: never inside a ligature.
        cuts.retain(|(at, _)| word_pieces.iter().any(|p| p.glyphs.iter().any(|g| p.start + g.cluster as usize == *at)));
        cuts.sort_by_key(|c| c.0);
        cuts.dedup_by_key(|c| c.0);

        let mut at = start;
        for (pos, cut) in cuts.into_iter().chain([(end, Cut::Break)]) {
            items.push(Item::Box { width: span_width(word_pieces, at, pos) });
            meta.push(Meta::Text(at, pos));
            if pos < end {
                if cut == Cut::Ideographic {
                    // A little stretch between characters, so justified lines without spaces can fill the column.
                    let size = segments[segment_at(segments, pos)].size;
                    items.push(Item::Glue { width: 0.0, stretch: size * 0.1, shrink: 0.0 });
                    meta.push(Meta::Plain);
                } else if cut == Cut::Hyphen {
                    let index = first_piece + word_pieces.iter().rposition(|p| p.start < pos).unwrap_or(0);
                    let piece = &pieces[index];
                    let width = self.width_of(self.font_of(segments[piece.segment].face), "-", piece.size);
                    items.push(Item::Penalty { width, penalty, flagged: true });
                    meta.push(Meta::Hyphen(index));
                } else {
                    items.push(Item::Penalty { width: 0.0, penalty: 0.0, flagged: false });
                    meta.push(Meta::Plain);
                }
            }
            at = pos;
        }
    }

    fn width_of(&mut self, font: FontId, s: &str, size: f32) -> f32 {
        let scale = size / self.shaper.units_per_em(font);
        self.shaper.shape(font, s).iter().map(|g| g.advance as f32 * scale).sum()
    }

    fn lines(
        &mut self,
        items: &[Item],
        meta: &[Meta],
        pieces: &[Piece],
        segments: &[Segment],
        breaks: &[knuth_plass::Break],
        spec: &Spec,
    ) -> (Vec<Line>, Vec<usize>) {
        let mut lines = Vec::new();
        let mut starts = Vec::new();
        let mut from = 0;
        let mut prev_end = 0;
        for (n, br) in breaks.iter().enumerate() {
            let last = n + 1 == breaks.len();
            let mut s = from;
            while s < br.index && !matches!(items[s], Item::Box { .. }) {
                s += 1;
            }
            let mut line = Line::default();
            let mut x = 0.0;
            let mut line_start = None;
            for k in s..br.index {
                match (items[k], meta[k]) {
                    (Item::Box { width }, Meta::Text(a, b)) => {
                        line_start.get_or_insert(a);
                        draw(&mut line, pieces, segments, a, b, x);
                        x += width;
                        prev_end = b;
                    }
                    (Item::Box { width }, _) => x += width,
                    (Item::Glue { width, stretch, shrink }, _) => {
                        // The last line is not stretched, but one the breaker shrank must be drawn shrunk.
                        x += width
                            + if br.ratio < 0.0 {
                                br.ratio * shrink
                            } else if spec.justify && !last {
                                br.ratio * stretch
                            } else {
                                0.0
                            };
                    }
                    (Item::Penalty { .. }, _) => {}
                }
            }
            if let (Item::Penalty { width, flagged: true, .. }, Meta::Hyphen(index)) = (items[br.index], meta[br.index]) {
                let piece = &pieces[index];
                let seg = &segments[piece.segment];
                let font = self.font_of(seg.face);
                let scale = piece.size / self.shaper.units_per_em(font);
                for g in self.shaper.shape(font, "-").iter() {
                    let at = [x + g.x_offset as f32 * scale, -piece.rise - g.y_offset as f32 * scale];
                    push_glyph(&mut line.runs, font, piece.size, seg.ink, g.id, at);
                }
                x += width;
            }
            let shift = match spec.align {
                Align::Left => 0.0,
                Align::Center => (spec.width - x) / 2.0,
                Align::Right => spec.width - x,
            };
            if shift != 0.0 {
                line.runs.iter_mut().flat_map(|r| r.positions.iter_mut()).for_each(|p| p[0] += shift);
                line.areas.iter_mut().for_each(|a| a.rect.x += shift);
            }
            lines.push(line);
            starts.push(line_start.unwrap_or(prev_end));
            from = br.index + 1;
        }
        (lines, starts)
    }
}

/// A break point inside a word.
#[derive(Clone, Copy, PartialEq)]
enum Cut {
    Hyphen,
    /// After a hard hyphen or a dash.
    Break,
    /// Between Chinese or Japanese characters.
    Ideographic,
}

/// Characters that may not start a line (closing punctuation, small kana, the prolonged sound mark).
const NO_LINE_START: &str =
    "、。，．・：；？！ー）」』】〕〉》｝］〙〗〟’”ゝゞヽヾ々ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶ‐゠〜?!),.:;]}";
/// Characters that may not end a line (opening punctuation).
const NO_LINE_END: &str = "（「『【〔〈《｛［〘〖〝‘“([{";

/// Han ideographs, kana and CJK punctuation, which are set without spaces.
fn is_ideographic(c: char) -> bool {
    matches!(c as u32, 0x2E80..=0x2FDF | 0x3000..=0x30FF | 0x3100..=0x31FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF | 0x20000..=0x3FFFF)
}

fn segment_at(segments: &[Segment], at: usize) -> usize {
    segments.iter().position(|s| at >= s.start && at < s.end).unwrap_or(segments.len() - 1)
}

/// Runs of one style: cut at every style range edge.
fn segments(text: &StyledText, size: f32, italic: bool, bold: bool) -> Vec<Segment> {
    let len = text.text.len();
    let mut cuts = vec![0, len];
    for r in &text.styles {
        cuts.push(r.start.min(len));
        cuts.push(r.end.min(len));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut out: Vec<Segment> = cuts
        .windows(2)
        .map(|w| {
            let (s, e) = (w[0], w[1]);
            let range = |st: Style| text.styles.iter().find(|r| r.style == st && r.start <= s && r.end >= e);
            let has = |st: Style| range(st).is_some();
            let small = has(Style::Superscript) || has(Style::NoteRef) || has(Style::Subscript);
            let rise = if has(Style::Superscript) || has(Style::NoteRef) {
                size * 0.35
            } else if has(Style::Subscript) {
                -size * 0.15
            } else {
                0.0
            };
            let note = range(Style::NoteRef).and_then(|r| r.target.clone()).map(Target::Note);
            let link = range(Style::Link).and_then(|r| r.target.clone()).map(Target::Link);
            Segment {
                start: s,
                end: e,
                face: Face::of(italic || has(Style::Emphasis), bold || has(Style::Strong)),
                size: if small { size * 0.7 } else { size },
                rise,
                ink: if link.is_some() { Ink::Link } else { Ink::Text },
                target: note.or(link),
            }
        })
        .collect();
    if out.is_empty() {
        out.push(Segment { start: 0, end: 0, face: Face::of(italic, bold), size, rise: 0.0, ink: Ink::Text, target: None });
    }
    out
}

/// Width of text `[a, b)` of a word: advances of the glyphs whose clusters start there.
fn span_width(pieces: &[Piece], a: usize, b: usize) -> f32 {
    pieces
        .iter()
        .flat_map(|p| p.glyphs.iter().map(move |g| (p, g)))
        .filter(|(p, g)| (a..b).contains(&(p.start + g.cluster as usize)))
        .map(|(p, g)| g.advance as f32 * p.scale)
        .sum()
}

/// Glyphs of text `[a, b)` at `x`; note markers and links also extend the line's active areas.
fn draw(line: &mut Line, pieces: &[Piece], segments: &[Segment], a: usize, b: usize, mut x: f32) {
    for p in pieces {
        let seg = &segments[p.segment];
        for g in p.glyphs.iter() {
            if !(a..b).contains(&(p.start + g.cluster as usize)) {
                continue;
            }
            push_glyph(
                &mut line.runs,
                p.font,
                p.size,
                seg.ink,
                g.id,
                [x + g.x_offset as f32 * p.scale, -p.rise - g.y_offset as f32 * p.scale],
            );
            let advance = g.advance as f32 * p.scale;
            if let Some(target) = &seg.target {
                let rect = Rect { x, y: -p.rise - p.ascent, w: advance, h: p.ascent + p.descent };
                match line.areas.last_mut() {
                    // The words of one link are one area: across the (possibly stretched) spaces between them.
                    Some(area) if area.target == *target && area.rect.x + area.rect.w >= x - 2.0 * p.size => {
                        let bottom = (area.rect.y + area.rect.h).max(rect.y + rect.h);
                        area.rect.y = area.rect.y.min(rect.y);
                        area.rect.h = bottom - area.rect.y;
                        area.rect.w = x + advance - area.rect.x;
                    }
                    _ => line.areas.push(Area { rect, target: target.clone() }),
                }
            }
            x += advance;
        }
    }
}

fn push_glyph(runs: &mut Vec<GlyphRun>, font: FontId, size: f32, ink: Ink, id: u32, at: [f32; 2]) {
    match runs.last_mut() {
        Some(r) if r.font == font && r.size == size && r.ink == ink => {
            r.glyphs.push(id as u16);
            r.positions.push(at);
        }
        _ => runs.push(GlyphRun { font, size, ink, glyphs: vec![id as u16], positions: vec![at] }),
    }
}

#[cfg(test)]
mod tests;
