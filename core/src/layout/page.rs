//! Pagination (a port of the Android `Paginator`) and page contents.

use super::{Area, BlockLayout, Content, GlyphRun, Group, Ink, PageStyle, Rect, Typesetter};
use crate::fonts::Face;
use crate::model::{Block, Book, Chapter, Kind, Locator, StyledText};

/// Lines `[line_start, line_end)` of a block (one "line" for an image or a table row), `top` below the text area's top.
#[derive(Debug, Clone, Copy)]
pub(super) struct Slice {
    pub(super) layout: usize,
    pub(super) block: usize,
    pub(super) line_start: usize,
    pub(super) line_end: usize,
    pub(super) top: f32,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub chapter: usize,
    pub(super) slices: Vec<Slice>,
}

/// An image to draw: its id in `Book::images` and where.
#[derive(Debug, Clone, PartialEq)]
pub struct PageImage {
    pub id: String,
    pub rect: Rect,
}

/// A table rule: a filled rectangle; `strength` 1 light, 2 medium, 3 heavy (the shell picks the ink's opacity).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rule {
    pub rect: Rect,
    pub strength: u8,
}

/// Everything a shell draws for one page, in page coordinates (logical pixels, y down).
#[derive(Debug, Clone, Default)]
pub struct PageContent {
    pub runs: Vec<GlyphRun>,
    pub images: Vec<PageImage>,
    pub rules: Vec<Rule>,
    pub areas: Vec<Area>,
}

/// A chapter typeset for one page style.
pub struct ChapterLayout {
    pub chapter: usize,
    pub(super) blocks: Vec<BlockLayout>,
    pub pages: Vec<Page>,
    /// The chapter's title, for the running head.
    title: Option<String>,
    position: Position,
    /// The model block index of each block layout.
    block_index: Vec<usize>,
    /// The chapter as one column ("Scroll"): each block layout's top, and the column's height.
    flow: Vec<f32>,
    flow_height: f32,
}

/// Where the chapter's text lies in the book, to tell how far a page is (text lengths, see `Block::text_len`).
#[derive(Default)]
struct Position {
    /// Text before the chapter.
    before: usize,
    /// Text before each block of the chapter.
    blocks: Vec<usize>,
    /// Text of the whole book.
    total: usize,
}

impl ChapterLayout {
    /// Where page `i` starts in the text: the reading position to restore after a re-layout.
    pub fn page_start(&self, i: usize) -> Locator {
        let s = self.pages[i].slices[0];
        let offset = match self.blocks.get(s.layout).map(|b| &b.content) {
            Some(Content::Text(t)) => t.starts.get(s.line_start).copied().unwrap_or(0),
            _ => 0,
        };
        Locator { chapter: self.chapter, block: s.block, offset }
    }

    /// The page that contains `at` (the last one starting at or before it).
    pub fn page_of(&self, at: Locator) -> usize {
        (0..self.pages.len()).rev().find(|&i| self.page_start(i) <= at).unwrap_or(0)
    }

    /// How far into the book page `i` starts, 0 to 1.
    pub fn progress(&self, i: usize) -> f32 {
        let p = &self.position;
        let at = self.page_start(i);
        let block = p.blocks.get(at.block).copied().unwrap_or_else(|| p.blocks.last().copied().unwrap_or(0));
        let block_len = p.blocks.get(at.block + 1).map_or(0, |next| next - block);
        if p.total == 0 { 0.0 } else { (p.before + block + at.offset.min(block_len)) as f32 / p.total as f32 }
    }

    /// The height of page `i`'s content below the top margin.
    pub fn height(&self, i: usize) -> f32 {
        let height = |s: &Slice| {
            self.blocks.get(s.layout).map_or(0.0, |b| match &b.content {
                Content::Text(t) => (s.line_end - s.line_start) as f32 * t.line_height,
                _ => b.height(),
            })
        };
        self.pages[i].slices.iter().map(|s| s.top + height(s)).fold(0.0, f32::max)
    }

    /// What to draw on page `i`: the text, the running head (the chapter's title, not on its first page) and the foot
    /// (the page in the chapter and the position in the book).
    pub fn render(&self, i: usize, style: &PageStyle, typesetter: &mut Typesetter) -> PageContent {
        let mut out = self.render_body(i, style);
        let size = style.text_size * 0.7;
        let width = style.text_width();
        if i > 0
            && let Some(title) = &self.title
        {
            let y = style.margin_top / 2.0 + size * 0.35;
            out.runs.push(label(typesetter, title, size, style.width / 2.0, y, width));
        }
        let foot = format!("{} / {} · {}%", i + 1, self.pages.len(), (self.progress(i) * 100.0).round() as u32);
        let y = style.height - style.margin_bottom / 2.0 + size * 0.35;
        out.runs.push(label(typesetter, &foot, size, style.width / 2.0, y, width));
        out
    }

    /// Page `i`'s text, images and rules only.
    pub fn render_body(&self, i: usize, style: &PageStyle) -> PageContent {
        let mut out = PageContent::default();
        for s in &self.pages[i].slices {
            if let Some(b) = self.blocks.get(s.layout) {
                draw_block(&mut out, b, s.line_start..s.line_end, style.margin_h + b.x, style.margin_top + s.top);
            }
        }
        out
    }

    /// The height of the chapter as one column ("Scroll").
    pub fn flow_height(&self) -> f32 {
        self.flow_height
    }

    /// The lines of the column whose tops lie between heights `y0` and `y1`, drawn with `y0` at the top. Each line
    /// belongs to one slice, so slices drawn next to each other never draw a line twice (the last one may hang below
    /// its slice, by at most a line, an image or a table row).
    pub fn render_flow(&self, y0: f32, y1: f32, style: &PageStyle) -> PageContent {
        let mut out = PageContent::default();
        for (b, &top) in self.blocks.iter().zip(&self.flow) {
            if top >= y1 || top + b.height() <= y0 {
                continue;
            }
            let lines = match &b.content {
                Content::Text(t) => {
                    let first = ((y0 - top) / t.line_height).ceil().max(0.0) as usize;
                    let last = (((y1 - top) / t.line_height).ceil().max(0.0) as usize).min(t.lines.len());
                    first..last.max(first)
                }
                _ if top < y0 => continue,
                _ => 0..1,
            };
            let first = lines.start;
            let line_top = match &b.content {
                Content::Text(t) => top + first as f32 * t.line_height,
                _ => top,
            };
            draw_block(&mut out, b, lines, style.margin_h + b.x, line_top - y0);
        }
        out
    }

    /// Where in the column the line holding `at` starts.
    pub fn flow_y(&self, at: Locator) -> f32 {
        let Some(i) = self.block_index.iter().rposition(|&b| b <= at.block) else { return 0.0 };
        let top = self.flow[i];
        match &self.blocks[i].content {
            Content::Text(t) if self.block_index[i] == at.block => {
                let line = t.starts.iter().rposition(|&s| s <= at.offset).unwrap_or(0);
                top + line as f32 * t.line_height
            }
            _ => top,
        }
    }

    /// The first line starting at or below height `y` of the column (the last line when `y` is past the end).
    pub fn flow_locator(&self, y: f32) -> Locator {
        let at = |i: usize, offset: usize| Locator { chapter: self.chapter, block: self.block_index[i], offset };
        let mut last = Locator { chapter: self.chapter, ..Locator::default() };
        for (i, (b, &top)) in self.blocks.iter().zip(&self.flow).enumerate() {
            match &b.content {
                Content::Text(t) => {
                    for (line, &start) in t.starts.iter().enumerate() {
                        if top + line as f32 * t.line_height >= y - 0.5 {
                            return at(i, start);
                        }
                        last = at(i, start);
                    }
                }
                Content::Empty => {}
                _ => {
                    if top >= y - 0.5 {
                        return at(i, 0);
                    }
                    last = at(i, 0);
                }
            }
        }
        last
    }
}

/// Lines `lines` of block `b` (one "line" for an image or a table row) with the first at `top`, the column at `left`.
fn draw_block(out: &mut PageContent, b: &BlockLayout, lines: std::ops::Range<usize>, left: f32, top: f32) {
    match &b.content {
        Content::Text(t) => {
            for line in lines.clone() {
                let y = top + (line - lines.start) as f32 * t.line_height + t.baseline;
                push_line(out, &t.lines[line], left, y);
            }
        }
        Content::Empty => {}
        Content::Image { id, w, h } => out.images.push(PageImage { id: id.clone(), rect: Rect { x: left, y: top, w: *w, h: *h } }),
        Content::Row { cells, rules, height, width } => {
            for cell in cells {
                for (n, line) in cell.text.lines.iter().enumerate() {
                    let y = top + cell.y + n as f32 * cell.text.line_height + cell.text.baseline;
                    push_line(out, line, left + cell.x, y);
                }
            }
            for (strength, y) in [(rules.0, top), (rules.1, top + height)] {
                if strength > 0 {
                    let h = [0.0, 0.5, 1.0, 1.5][strength as usize];
                    out.rules.push(Rule { rect: Rect { x: left, y: y - h / 2.0, w: *width, h }, strength });
                }
            }
        }
    }
}

/// A line of secondary text centred on `center` at baseline `y`, shortened with "…" to `max_width`.
fn label(typesetter: &mut Typesetter, text: &str, size: f32, center: f32, y: f32, max_width: f32) -> GlyphRun {
    let font = typesetter.font_of(Face::Regular);
    let scale = size / typesetter.shaper.units_per_em(font);
    let mut text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut glyphs = typesetter.shaper.shape(font, &text);
    let width = |g: &[crate::fonts::Glyph]| g.iter().map(|g| g.advance as f32 * scale).sum::<f32>();
    while width(&glyphs) > max_width && text.chars().count() > 1 {
        let keep = text.chars().count() * 9 / 10;
        text = text.chars().take(keep).collect::<String>().trim_end().to_owned() + "…";
        glyphs = typesetter.shaper.shape(font, &text);
    }
    let mut x = center - width(&glyphs) / 2.0;
    let mut run = GlyphRun { font, size, ink: Ink::Secondary, glyphs: Vec::new(), positions: Vec::new() };
    for g in glyphs.iter() {
        run.glyphs.push(g.id as u16);
        run.positions.push([x + g.x_offset as f32 * scale, y - g.y_offset as f32 * scale]);
        x += g.advance as f32 * scale;
    }
    run
}

/// Note `id` typeset as one tall page at `style`'s width: its title (a subtitle) and text, the chapter start not
/// lowered. Draw it with `render_body`; images are the book's.
pub fn layout_note(book: &Book, id: &str, style: &PageStyle, typesetter: &mut Typesetter) -> Option<ChapterLayout> {
    let note = book.notes.get(id)?;
    let mut blocks = Vec::new();
    if let Some(title) = note.title.as_ref().filter(|t| !t.trim().is_empty()) {
        blocks.push(Block::Paragraph { text: StyledText { text: title.clone(), styles: Vec::new() }, kind: Kind::Subtitle });
    }
    blocks.extend(note.blocks.iter().cloned());
    let images = blocks
        .iter()
        .filter_map(|b| if let Block::Image { id } = b { book.images.get_key_value(id) } else { None })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let note_book = Book { language: book.language.clone(), images, chapters: vec![Chapter { title: None, blocks }], ..Book::default() };
    Some(paginate_from(&note_book, 0, style, typesetter, 0.0))
}

fn push_line(out: &mut PageContent, line: &super::Line, x: f32, y: f32) {
    out.runs.extend(line.runs.iter().map(|r| r.offset(x, y)));
    out.areas.extend(line.areas.iter().map(|a| Area { rect: a.rect.offset(x, y), target: a.target.clone() }));
}

/// Paginates a chapter: a port of the Android `Paginator`, which needs only line counts and heights from its layouts.
pub fn paginate(book: &Book, chapter: usize, style: &PageStyle, typesetter: &mut Typesetter) -> ChapterLayout {
    let lowered = typesetter.line_height * 2.0; // the chapter start is lowered, as in a printed book
    paginate_from(book, chapter, style, typesetter, lowered)
}

fn paginate_from(book: &Book, chapter: usize, style: &PageStyle, typesetter: &mut Typesetter, start: f32) -> ChapterLayout {
    let model = &book.chapters[chapter].blocks;
    // Layouts paired with their block index in the chapter: blocks that take no room have no layout.
    let laid: Vec<(usize, BlockLayout)> =
        model.iter().enumerate().filter_map(|(i, b)| typesetter.layout(b, book).map(|l| (i, l))).collect();
    let blocks: Vec<&BlockLayout> = laid.iter().map(|(_, l)| l).collect();
    let page_height = style.text_height();
    let lh = typesetter.line_height;
    let mut pages = Vec::new();
    let mut slices: Vec<Slice> = Vec::new();
    let mut y = start;
    let mut prev: Option<usize> = None;

    let mut new_page = |slices: &mut Vec<Slice>, y: &mut f32, prev: &mut Option<usize>| {
        if !slices.is_empty() {
            pages.push(Page { chapter, slices: std::mem::take(slices) });
        }
        *y = 0.0;
        *prev = None;
    };
    let gap = |prev: Option<&BlockLayout>, block: &BlockLayout| match prev {
        None => block.space_before,
        Some(p) if p.group == block.group && block.group != Group::Alone => 0.0,
        Some(p) => p.space_after.max(block.space_before),
    };

    for (index, &block) in blocks.iter().enumerate() {
        if matches!(block.content, Content::Empty) {
            // An empty line is not needed at the top of a page.
            if y > 0.0 && y + lh <= page_height {
                y += lh;
            }
            prev = Some(index);
            continue;
        }
        if y > 0.0 {
            y += gap(prev.map(|p| blocks[p]), block);
        }
        if block.atomic() {
            // A header row goes with the next row; an image or row taller than a page is drawn as is.
            let next = blocks.get(index + 1).filter(|n| block.keep_with_next && n.atomic());
            let need = block.height() + next.map_or(0.0, |n| n.height());
            if y > 0.0 && y + need > page_height {
                new_page(&mut slices, &mut y, &mut prev);
            }
            slices.push(Slice { layout: index, block: laid[index].0, line_start: 0, line_end: 1, top: y });
            y += block.height();
            prev = Some(index);
            continue;
        }
        let Content::Text(text) = &block.content else { continue };
        if block.keep_with_next && y > 0.0 {
            // A heading stays with two lines of what follows (or with the first row of a table).
            let next = blocks.get(index + 1).map_or(0.0, |n| match &n.content {
                Content::Text(t) => t.lines.len().min(2) as f32 * t.line_height,
                _ => n.height(),
            });
            if y + block.height() + lh + next > page_height {
                new_page(&mut slices, &mut y, &mut prev);
            }
        }
        let lines = block.line_count();
        let mut line = 0;
        while line < lines {
            let mut end = line;
            while end < lines && (end - line + 1) as f32 * text.line_height <= page_height - y {
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
            slices.push(Slice { layout: index, block: laid[index].0, line_start: line, line_end: end, top: y });
            y += (end - line) as f32 * text.line_height;
            line = end;
            if line < lines {
                new_page(&mut slices, &mut y, &mut prev);
            }
        }
        prev = Some(index);
    }
    new_page(&mut slices, &mut y, &mut prev);
    if pages.is_empty() {
        pages.push(Page { chapter, slices: vec![Slice { layout: usize::MAX, block: 0, line_start: 0, line_end: 0, top: 0.0 }] });
    }
    let mut position = Position::default();
    for (c, ch) in book.chapters.iter().enumerate() {
        let lengths = ch.blocks.iter().map(Block::text_len);
        if c == chapter {
            for len in lengths {
                position.blocks.push(position.total - position.before);
                position.total += len;
            }
            position.blocks.push(position.total - position.before);
        } else {
            position.total += lengths.sum::<usize>();
            if c < chapter {
                position.before = position.total;
            }
        }
    }
    // The column ("Scroll"): the same gaps, no page breaks.
    let mut flow = Vec::with_capacity(blocks.len());
    let mut y = start;
    let mut prev: Option<&BlockLayout> = None;
    for &block in &blocks {
        if matches!(block.content, Content::Empty) {
            flow.push(y);
            y += lh;
            prev = Some(block);
            continue;
        }
        y += gap(prev, block);
        flow.push(y);
        y += block.height();
        prev = Some(block);
    }
    let flow_height = y + lh * 2.0;

    let title = book.chapters[chapter].title.clone();
    let block_index = laid.iter().map(|(i, _)| *i).collect();
    ChapterLayout { chapter, blocks: laid.into_iter().map(|(_, l)| l).collect(), pages, title, position, block_index, flow, flow_height }
}
