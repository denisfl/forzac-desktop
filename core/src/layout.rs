//! Typesetting: blocks → lines (Knuth–Plass with Liang hyphenation over HarfRust-shaped words) → pages → positioned
//! glyph runs that a shell draws as is.
//!
//! Rules follow the Android app: 1.5 em first-line indent, justified body text with hyphenation, ragged verse and
//! headings without it, fixed line height for vertical rhythm, no lone line at the top or bottom of a page, a heading
//! kept with the next block, the chapter start lowered by two lines.

use std::rc::Rc;

use crate::fonts::{Face, Glyph, Shaper};
use crate::hyphen::Hyphenator;
use crate::knuth_plass::{self, INFINITY, Item};
use crate::model::{Block, Book, Kind, Locator, Style, StyledText};

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
}

impl PageStyle {
    /// The default page for a window: a column of at most ~34 em (about 65 characters), centred.
    pub fn for_window(width: f32, height: f32, text_size: f32) -> Self {
        let margin_h = (48.0f32).max((width - text_size * 34.0) / 2.0);
        PageStyle { width, height, margin_h, margin_top: 56.0, margin_bottom: 56.0, text_size, line_height: 1.5, justify: true }
    }

    pub fn text_width(&self) -> f32 {
        self.width - 2.0 * self.margin_h
    }

    pub fn text_height(&self) -> f32 {
        self.height - self.margin_top - self.margin_bottom
    }
}

/// Glyphs of one face and size. Positions are glyph origins on the baseline, y down.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRun {
    pub face: Face,
    pub size: f32,
    pub glyphs: Vec<u16>,
    pub positions: Vec<[f32; 2]>,
}

/// Blocks of one group (verse lines, epigraph paragraphs) go without spacing between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Kind(Kind),
    EmptyLine,
}

/// A typeset block: lines of fixed height; glyph positions relative to the block's left edge and the line baseline.
pub struct BlockLayout {
    lines: Vec<Vec<GlyphRun>>,
    /// Byte offset in the block text where each line starts: page start → [`Locator`].
    line_starts: Vec<usize>,
    x: f32,
    line_height: f32,
    baseline: f32,
    group: Group,
    space_before: f32,
    space_after: f32,
    keep_with_next: bool,
}

impl BlockLayout {
    fn height(&self) -> f32 {
        if self.lines.is_empty() { self.line_height } else { self.lines.len() as f32 * self.line_height }
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
    /// A hyphenation point: the hyphen is drawn in this face and size when the line breaks here.
    Hyphen(Face, f32),
}

/// A word, or the part of it in one style, shaped as a whole: kerning inside the word is kept across syllables.
struct Piece {
    start: usize,
    face: Face,
    size: f32,
    rise: f32,
    scale: f32,
    glyphs: Rc<[Glyph]>,
}

struct Segment {
    start: usize,
    end: usize,
    face: Face,
    size: f32,
    rise: f32,
}

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

/// Lays out blocks for one page style. Holds the shaping caches; not thread-safe.
pub struct Typesetter {
    style: PageStyle,
    shaper: Shaper,
    hyphenator: Option<Rc<Hyphenator>>,
    /// Body line height: the unit of vertical spacing.
    pub line_height: f32,
}

impl Typesetter {
    pub fn new(style: PageStyle, hyphenator: Option<Rc<Hyphenator>>) -> Self {
        let line_height = (style.text_size * style.line_height).floor();
        Typesetter { style, shaper: Shaper::new(), hyphenator, line_height }
    }

    pub fn layout(&mut self, block: &Block) -> BlockLayout {
        let w = self.style.text_width();
        let lh = self.line_height;
        let half = lh / 2.0;
        let base = Spec {
            width: w,
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
        };
        let justify = self.style.justify;
        match block {
            Block::EmptyLine => BlockLayout {
                lines: Vec::new(),
                line_starts: Vec::new(),
                x: 0.0,
                line_height: lh,
                baseline: 0.0,
                group: Group::EmptyLine,
                space_before: 0.0,
                space_after: 0.0,
                keep_with_next: false,
            },
            Block::Heading { text, level } => self.make(
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
            ),
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
                self.make(text, spec)
            }
        }
    }

    fn make(&mut self, text: &StyledText, spec: Spec) -> BlockLayout {
        let size = self.style.text_size * spec.scale;
        let segments = segments(text, size, spec.italic, spec.bold);
        let indent = if spec.indent { self.style.text_size * 1.5 } else { 0.0 };
        let (items, meta, pieces) = self.items(&text.text, &segments, indent, spec.justify, spec.hyphenate);
        let breaks = knuth_plass::break_lines(&items, |_| spec.width);
        let (lines, line_starts) = self.lines(&items, &meta, &pieces, &breaks, &spec);

        let line_height = (size * self.style.line_height * spec.line_scale).floor();
        let face = segments.first().map(|s| s.face).unwrap_or(Face::Regular);
        let (asc, desc) = self.shaper.line_metrics(face);
        let units = size / self.shaper.units_per_em(face);
        let baseline = (line_height - (asc - desc) * units) / 2.0 + asc * units;
        BlockLayout {
            lines,
            line_starts,
            x: spec.x,
            line_height,
            baseline,
            group: spec.group,
            space_before: spec.space_before,
            space_after: spec.space_after,
            keep_with_next: spec.keep_with_next,
        }
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
                let seg = segment_at(segments, i);
                let space = self.width_of(seg.face, " ", seg.size);
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
            self.word(text, segments, i, end, hyphenate, if justify { 50.0 } else { 300.0 }, &mut items, &mut meta, &mut pieces);
            i = end;
        }
        // End of paragraph: the last line is not stretched.
        items.push(Item::Penalty { width: 0.0, penalty: INFINITY, flagged: false });
        items.push(Item::Glue { width: 0.0, stretch: 100_000.0, shrink: 0.0 });
        items.push(Item::Penalty { width: 0.0, penalty: -INFINITY, flagged: false });
        meta.extend([Meta::Plain, Meta::Plain, Meta::Plain]);
        (items, meta, pieces)
    }

    #[allow(clippy::too_many_arguments)]
    fn word(
        &mut self,
        text: &str,
        segments: &[Segment],
        start: usize,
        end: usize,
        hyphenate: bool,
        penalty: f32,
        items: &mut Vec<Item>,
        meta: &mut Vec<Meta>,
        pieces: &mut Vec<Piece>,
    ) {
        let first_piece = pieces.len();
        for seg in segments.iter().filter(|s| s.start < end && s.end > start) {
            let (a, b) = (seg.start.max(start), seg.end.min(end));
            let glyphs = self.shaper.shape(seg.face, &text[a..b]);
            let scale = seg.size / self.shaper.units_per_em(seg.face);
            pieces.push(Piece { start: a, face: seg.face, size: seg.size, rise: seg.rise, scale, glyphs });
        }
        let word_pieces = &pieces[first_piece..];

        // Break points inside the word: (byte offset, hyphenation?).
        let mut cuts: Vec<(usize, bool)> = Vec::new();
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
                    cuts.extend(h.hyphenate(&letters).into_iter().map(|p| (chars[run_start + p].0, true)));
                }
            } else {
                if matches!(chars[k].1, '-' | '—') && k > 0 && k + 1 < chars.len() {
                    cuts.push((chars[k + 1].0, false));
                }
                k += 1;
            }
        }
        // Only where a glyph cluster starts: never inside a ligature.
        cuts.retain(|(at, _)| word_pieces.iter().any(|p| p.glyphs.iter().any(|g| p.start + g.cluster as usize == *at)));
        cuts.sort_by_key(|c| c.0);

        let mut at = start;
        for (pos, hyphen) in cuts.into_iter().chain([(end, false)]) {
            items.push(Item::Box { width: span_width(word_pieces, at, pos) });
            meta.push(Meta::Text(at, pos));
            if pos < end {
                if hyphen {
                    let piece = word_pieces.iter().rev().find(|p| p.start < pos).unwrap_or(&word_pieces[0]);
                    let (face, size) = (piece.face, piece.size);
                    items.push(Item::Penalty { width: self.width_of(face, "-", size), penalty, flagged: true });
                    meta.push(Meta::Hyphen(face, size));
                } else {
                    items.push(Item::Penalty { width: 0.0, penalty: 0.0, flagged: false });
                    meta.push(Meta::Plain);
                }
            }
            at = pos;
        }
    }

    fn width_of(&mut self, face: Face, s: &str, size: f32) -> f32 {
        let scale = size / self.shaper.units_per_em(face);
        self.shaper.shape(face, s).iter().map(|g| g.advance as f32 * scale).sum()
    }

    fn lines(
        &mut self,
        items: &[Item],
        meta: &[Meta],
        pieces: &[Piece],
        breaks: &[knuth_plass::Break],
        spec: &Spec,
    ) -> (Vec<Vec<GlyphRun>>, Vec<usize>) {
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
            let mut runs: Vec<GlyphRun> = Vec::new();
            let mut x = 0.0;
            let mut line_start = None;
            for k in s..br.index {
                match (items[k], meta[k]) {
                    (Item::Box { width }, Meta::Text(a, b)) => {
                        line_start.get_or_insert(a);
                        draw(&mut runs, pieces, a, b, x);
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
            if let (Item::Penalty { width, flagged: true, .. }, Meta::Hyphen(face, size)) = (items[br.index], meta[br.index]) {
                let scale = size / self.shaper.units_per_em(face);
                for g in self.shaper.shape(face, "-").iter() {
                    push_glyph(&mut runs, face, size, g.id, x + g.x_offset as f32 * scale, -(g.y_offset as f32) * scale);
                }
                x += width;
            }
            let shift = match spec.align {
                Align::Left => 0.0,
                Align::Center => (spec.width - x) / 2.0,
                Align::Right => spec.width - x,
            };
            if shift != 0.0 {
                runs.iter_mut().flat_map(|r| r.positions.iter_mut()).for_each(|p| p[0] += shift);
            }
            lines.push(runs);
            starts.push(line_start.unwrap_or(prev_end));
            from = br.index + 1;
        }
        (lines, starts)
    }
}

fn segment_at(segments: &[Segment], at: usize) -> &Segment {
    segments.iter().find(|s| at >= s.start && at < s.end).unwrap_or_else(|| segments.last().unwrap())
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
            let has = |st: Style| text.styles.iter().any(|r| r.style == st && r.start <= s && r.end >= e);
            let small = has(Style::Superscript) || has(Style::NoteRef) || has(Style::Subscript);
            let rise = if has(Style::Superscript) || has(Style::NoteRef) {
                size * 0.35
            } else if has(Style::Subscript) {
                -size * 0.15
            } else {
                0.0
            };
            Segment {
                start: s,
                end: e,
                face: Face::of(italic || has(Style::Emphasis), bold || has(Style::Strong)),
                size: if small { size * 0.7 } else { size },
                rise,
            }
        })
        .collect();
    if out.is_empty() {
        out.push(Segment { start: 0, end: 0, face: Face::of(italic, bold), size, rise: 0.0 });
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

fn draw(runs: &mut Vec<GlyphRun>, pieces: &[Piece], a: usize, b: usize, mut x: f32) {
    for p in pieces {
        for g in p.glyphs.iter() {
            if (a..b).contains(&(p.start + g.cluster as usize)) {
                push_glyph(runs, p.face, p.size, g.id, x + g.x_offset as f32 * p.scale, -p.rise - g.y_offset as f32 * p.scale);
                x += g.advance as f32 * p.scale;
            }
        }
    }
}

fn push_glyph(runs: &mut Vec<GlyphRun>, face: Face, size: f32, id: u32, x: f32, y: f32) {
    match runs.last_mut() {
        Some(r) if r.face == face && r.size == size => {
            r.glyphs.push(id as u16);
            r.positions.push([x, y]);
        }
        _ => runs.push(GlyphRun { face, size, glyphs: vec![id as u16], positions: vec![[x, y]] }),
    }
}

/// Lines `[line_start, line_end)` of a block, `top` below the top of the text area.
#[derive(Debug, Clone, Copy)]
pub struct Slice {
    layout: usize,
    block: usize,
    line_start: usize,
    line_end: usize,
    top: f32,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub chapter: usize,
    slices: Vec<Slice>,
}

/// A chapter typeset for one page style.
pub struct ChapterLayout {
    pub chapter: usize,
    blocks: Vec<BlockLayout>,
    pub pages: Vec<Page>,
}

impl ChapterLayout {
    /// Where page `i` starts in the text: the reading position to restore after a re-layout.
    pub fn page_start(&self, i: usize) -> Locator {
        let s = self.pages[i].slices[0];
        Locator { chapter: self.chapter, block: s.block, offset: self.blocks[s.layout].line_starts.get(s.line_start).copied().unwrap_or(0) }
    }

    /// The page that contains `at` (the last one starting at or before it).
    pub fn page_of(&self, at: Locator) -> usize {
        (0..self.pages.len()).rev().find(|&i| self.page_start(i) <= at).unwrap_or(0)
    }

    /// Glyph runs of page `i` in page coordinates, with the page number at the bottom.
    pub fn render(&self, i: usize, style: &PageStyle, typesetter: &mut Typesetter) -> Vec<GlyphRun> {
        let mut out: Vec<GlyphRun> = Vec::new();
        for s in &self.pages[i].slices {
            let b = &self.blocks[s.layout];
            for line in s.line_start..s.line_end {
                let dx = style.margin_h + b.x;
                let dy = style.margin_top + s.top + (line - s.line_start) as f32 * b.line_height + b.baseline;
                for run in &b.lines[line] {
                    out.push(GlyphRun { positions: run.positions.iter().map(|p| [p[0] + dx, p[1] + dy]).collect(), ..run.clone() });
                }
            }
        }
        // Page number.
        let label = format!("{} / {}", i + 1, self.pages.len());
        let size = style.text_size * 0.7;
        let scale = size / typesetter.shaper.units_per_em(Face::Regular);
        let glyphs = typesetter.shaper.shape(Face::Regular, &label);
        let width: f32 = glyphs.iter().map(|g| g.advance as f32 * scale).sum();
        let mut x = (style.width - width) / 2.0;
        let y = style.height - style.margin_bottom / 2.0 + size * 0.35;
        let mut run = GlyphRun { face: Face::Regular, size, glyphs: Vec::new(), positions: Vec::new() };
        for g in glyphs.iter() {
            run.glyphs.push(g.id as u16);
            run.positions.push([x, y]);
            x += g.advance as f32 * scale;
        }
        out.push(run);
        out
    }
}

/// Paginates a chapter: a port of the Android `Paginator`, which needs only line counts and heights from its layouts.
pub fn paginate(book: &Book, chapter: usize, style: &PageStyle, typesetter: &mut Typesetter) -> ChapterLayout {
    let model = &book.chapters[chapter].blocks;
    let blocks: Vec<BlockLayout> = model.iter().map(|b| typesetter.layout(b)).collect();
    let page_height = style.text_height();
    let lh = typesetter.line_height;
    let mut pages = Vec::new();
    let mut slices: Vec<Slice> = Vec::new();
    let mut y = lh * 2.0; // the chapter start is lowered, as in a printed book
    let mut prev: Option<usize> = None;

    let mut new_page = |slices: &mut Vec<Slice>, y: &mut f32, prev: &mut Option<usize>| {
        if !slices.is_empty() {
            pages.push(Page { chapter, slices: std::mem::take(slices) });
        }
        *y = 0.0;
        *prev = None;
    };

    for (index, block) in blocks.iter().enumerate() {
        if block.lines.is_empty() {
            // An empty line is not needed at the top of a page.
            if y > 0.0 && y + block.height() <= page_height {
                y += block.height();
            }
            prev = Some(index);
            continue;
        }
        if y > 0.0 {
            y += match prev.map(|p| &blocks[p]) {
                None => block.space_before,
                Some(p) if p.group == block.group => 0.0,
                Some(p) => p.space_after.max(block.space_before),
            };
        }
        if block.keep_with_next && y > 0.0 {
            let next = blocks.get(index + 1);
            let keep = block.height() + lh + next.map_or(0.0, |n| n.lines.len().min(2) as f32 * n.line_height);
            if y + keep > page_height {
                new_page(&mut slices, &mut y, &mut prev);
            }
        }
        let lines = block.lines.len();
        let mut line = 0;
        while line < lines {
            let mut end = line;
            while end < lines && (end - line + 1) as f32 * block.line_height <= page_height - y {
                end += 1;
            }
            if end < lines {
                if lines - end == 1 && end - line >= 2 {
                    end -= 1; // no lone last line on the next page
                }
                let orphan = line == 0 && end == 1 && lines > 1; // no lone first line at the bottom
                if end == line || orphan {
                    if y > 0.0 {
                        new_page(&mut slices, &mut y, &mut prev);
                        continue;
                    }
                    end = end.max(line + 1); // taller than a page: draw as is
                }
            }
            slices.push(Slice { layout: index, block: index, line_start: line, line_end: end, top: y });
            y += (end - line) as f32 * block.line_height;
            line = end;
            if line < lines {
                new_page(&mut slices, &mut y, &mut prev);
            }
        }
        prev = Some(index);
    }
    new_page(&mut slices, &mut y, &mut prev);
    if pages.is_empty() {
        pages.push(Page { chapter, slices: vec![Slice { layout: 0, block: 0, line_start: 0, line_end: 0, top: 0.0 }] });
    }
    ChapterLayout { chapter, blocks, pages }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Chapter, Kind};

    fn book(paragraphs: usize) -> Book {
        let text =
            "Слезы навернулись у нее на глаза. Видно, ей нравилась эта распущенность, видно, ее рассердило, что я хотел пристыдить ее.";
        let blocks = (0..paragraphs)
            .map(|_| Block::Paragraph { text: StyledText { text: text.into(), styles: vec![] }, kind: Kind::Body })
            .collect();
        Book { title: "T".into(), authors: vec![], language: Some("ru".into()), chapters: vec![Chapter { title: None, blocks }] }
    }

    #[test]
    fn justified_lines_fill_the_column() {
        let style = PageStyle::for_window(600.0, 900.0, 20.0);
        let mut t = Typesetter::new(style, Hyphenator::for_language(Some("ru")).map(Rc::new));
        let b = &book(1).chapters[0].blocks[0];
        let layout = t.layout(b);
        assert!(layout.lines.len() >= 3);
        // Every line but the last ends at the right edge of the column (a hyphen included).
        let right = |line: &Vec<GlyphRun>| line.iter().flat_map(|r| r.positions.iter()).map(|p| p[0]).fold(0.0f32, f32::max);
        for line in &layout.lines[..layout.lines.len() - 1] {
            assert!(right(line) > style.text_width() - 30.0 && right(line) <= style.text_width(), "line ends at {}", right(line));
        }
    }

    #[test]
    fn pages_hold_whole_chapter_and_find_locators() {
        let style = PageStyle::for_window(600.0, 500.0, 20.0);
        let b = book(30);
        let mut t = Typesetter::new(style, Hyphenator::for_language(Some("ru")).map(Rc::new));
        let ch = paginate(&b, 0, &style, &mut t);
        assert!(ch.pages.len() > 3);
        let lines: usize = ch.pages.iter().flat_map(|p| &p.slices).map(|s| s.line_end - s.line_start).sum();
        assert_eq!(lines, ch.blocks.iter().map(|b| b.lines.len()).sum::<usize>());
        for i in 0..ch.pages.len() {
            assert_eq!(ch.page_of(ch.page_start(i)), i);
        }
        assert!(!ch.render(1, &style, &mut t).is_empty());
    }
}
