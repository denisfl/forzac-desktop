//! Pagination (a port of the Android `Paginator`) and page contents.

use super::{Area, BlockLayout, BookPages, Content, GlyphRun, Group, Ink, PageStyle, Rect, Stroke, Target, Typesetter};
use crate::fonts::Face;
use crate::model::{Block, Book, Chapter, Locator};

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

/// A word drawn on a line (one piece of a hyphenated word): where it is, and the whole word's text `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    pub start: Locator,
    pub end: Locator,
    pub rect: Rect,
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
    pub strokes: Vec<Stroke>,
}

/// A chapter typeset for one page style.
pub struct ChapterLayout {
    pub chapter: usize,
    pub(super) blocks: Vec<BlockLayout>,
    pub pages: Vec<Page>,
    /// The chapter's title, for the running head.
    title: Option<String>,
    /// The book's title, for the running head of a spread's left page.
    book_title: Option<String>,
    position: Position,
    /// The model block index of each block layout.
    block_index: Vec<usize>,
    /// The chapter as one column ("Scroll"): each block layout's top, and the column's height.
    flow: Vec<f32>,
    flow_height: f32,
    /// Whether wide images and tables are areas (`view_area`): not in notes and previews, whose blocks are not the
    /// chapter's.
    viewable: bool,
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

    /// Where page `i` ends: the next page's start, or the next chapter's start after the last page.
    pub fn page_end(&self, i: usize) -> Locator {
        if i + 1 < self.pages.len() { self.page_start(i + 1) } else { Locator { chapter: self.chapter + 1, ..Locator::default() } }
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

    /// What to draw on page `i`: the text, the running head (the chapter's title, not on its first page; on a spread's
    /// left page the book's title) and the foot (the page in the chapter and the position in the book).
    pub fn render(&self, i: usize, style: &PageStyle, typesetter: &mut Typesetter) -> PageContent {
        self.render_numbered(i, style, typesetter, None)
    }

    /// The same, the foot as the page of the book and the book's pages ("37 of 212") when `pages` were counted for
    /// `style`.
    pub fn render_numbered(&self, i: usize, style: &PageStyle, typesetter: &mut Typesetter, pages: Option<&BookPages>) -> PageContent {
        let mut out = self.render_body(i, style);
        let size = style.text_size * 0.7;
        let width = style.text_width();
        // Every chapter starts on a spread's left page: its even pages are left pages.
        let head = if style.spread && i.is_multiple_of(2) { &self.book_title } else { &self.title };
        if i > 0
            && let Some(title) = head
        {
            let y = style.margin_top / 2.0 + size * 0.35;
            out.runs.push(label(typesetter, title, Ink::Secondary, size, style.width / 2.0, y, width));
        }
        let foot = match pages.filter(|p| p.style() == style) {
            Some(p) => crate::i18n::fill(
                crate::i18n::tr("{n} of {m}"),
                &[("n", &(p.of_chapter_page(self.chapter, i) + 1).to_string()), ("m", &p.total().to_string())],
            ),
            None => format!("{} / {} · {}%", i + 1, self.pages.len(), (self.progress(i) * 100.0).round() as u32),
        };
        let y = style.height - style.margin_bottom / 2.0 + size * 0.35;
        out.runs.push(label(typesetter, &foot, Ink::Secondary, size, style.width / 2.0, y, width));
        out
    }

    /// Page `i`'s text, images and rules only.
    pub fn render_body(&self, i: usize, style: &PageStyle) -> PageContent {
        let mut out = PageContent::default();
        for s in &self.pages[i].slices {
            if let Some(b) = self.blocks.get(s.layout) {
                let (left, top, at) = (style.margin_h + b.x, style.margin_top + s.top, out.areas.len());
                draw_block(&mut out, b, s.line_start..s.line_end, left, top);
                self.view_area(&mut out, at, b, s.block, left, top, style);
            }
        }
        out
    }

    /// The area of an image at least a third of the column wide or of a table, which open in the view: an image's
    /// rectangle; for a table one area around its rows drawn here, inserted at `at` (before the first row's cell areas)
    /// and grown by later rows. `block` is the model block of `b`, drawn at `left`, `top`.
    #[allow(clippy::too_many_arguments)]
    fn view_area(&self, out: &mut PageContent, at: usize, b: &BlockLayout, block: usize, left: f32, top: f32, style: &PageStyle) {
        if !self.viewable {
            return;
        }
        match &b.content {
            Content::Image { id, w, h } if *w >= style.text_width() / 3.0 => {
                out.areas.insert(at, Area { rect: Rect { x: left, y: top, w: *w, h: *h }, target: Target::Image(id.clone()) })
            }
            Content::Row { row, height, width, .. } => {
                let Some(first) = block.checked_sub(*row) else { return };
                let target = Target::Table(Locator { chapter: self.chapter, block: first, offset: 0 });
                let rect = Rect { x: left, y: top, w: *width, h: *height };
                match out.areas.iter_mut().find(|a| a.target == target) {
                    Some(area) => area.rect = area.rect.union(rect),
                    None => out.areas.insert(at, Area { rect, target }),
                }
            }
            _ => {}
        }
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
        for (i, lines, top) in self.flow_lines(y0, y1) {
            let b = &self.blocks[i];
            let (left, at) = (style.margin_h + b.x, out.areas.len());
            draw_block(&mut out, b, lines, left, top);
            self.view_area(&mut out, at, b, self.block_index[i], left, top, style);
        }
        out
    }

    /// The block layouts and their lines whose tops lie between column heights `y0` and `y1`, with the first line's top
    /// below `y0`.
    fn flow_lines(&self, y0: f32, y1: f32) -> Vec<(usize, std::ops::Range<usize>, f32)> {
        let mut out = Vec::new();
        for (i, (b, &top)) in self.blocks.iter().zip(&self.flow).enumerate() {
            if top >= y1 || top + b.height() <= y0 {
                continue;
            }
            let (lines, line_top) = match &b.content {
                Content::Text(t) => {
                    let first = ((y0 - top) / t.line_height).ceil().max(0.0) as usize;
                    let last = (((y1 - top) / t.line_height).ceil().max(0.0) as usize).min(t.lines.len());
                    (first..last.max(first), top + first as f32 * t.line_height)
                }
                _ if top < y0 => continue,
                _ => (0..1, top),
            };
            out.push((i, lines, line_top - y0));
        }
        out
    }

    /// Rectangles over the text `[from, to)` (a search match) on page `i`, in page coordinates: one per line.
    pub fn marks(&self, i: usize, style: &PageStyle, from: Locator, to: Locator) -> Vec<Rect> {
        let mut out = Vec::new();
        if from.chapter == self.chapter {
            for s in &self.pages[i].slices {
                if let Some(b) = self.blocks.get(s.layout) {
                    mark_block(&mut out, b, s.block, s.line_start..s.line_end, style.margin_h + b.x, style.margin_top + s.top, from, to);
                }
            }
        }
        out
    }

    /// The same between column heights `y0` and `y1`, in the coordinates `render_flow` draws in.
    pub fn flow_marks(&self, y0: f32, y1: f32, style: &PageStyle, from: Locator, to: Locator) -> Vec<Rect> {
        let mut out = Vec::new();
        if from.chapter == self.chapter {
            for (i, lines, top) in self.flow_lines(y0, y1) {
                let b = &self.blocks[i];
                mark_block(&mut out, b, self.block_index[i], lines, style.margin_h + b.x, top, from, to);
            }
        }
        out
    }

    /// The words drawn on page `i` of `book` (this layout's book), in reading order, in page coordinates.
    pub fn words(&self, i: usize, style: &PageStyle, book: &Book) -> Vec<Word> {
        let mut out = Vec::new();
        let blocks = &book.chapters[self.chapter].blocks;
        for s in &self.pages[i].slices {
            if let (Some(b), Some(block)) = (self.blocks.get(s.layout), blocks.get(s.block)) {
                word_block(
                    &mut out,
                    b,
                    block,
                    Locator { chapter: self.chapter, block: s.block, offset: 0 },
                    s.line_start..s.line_end,
                    style.margin_h + b.x,
                    style.margin_top + s.top,
                );
            }
        }
        out
    }

    /// The same between column heights `y0` and `y1`, in the coordinates `render_flow` draws in.
    pub fn flow_words(&self, y0: f32, y1: f32, style: &PageStyle, book: &Book) -> Vec<Word> {
        let mut out = Vec::new();
        let blocks = &book.chapters[self.chapter].blocks;
        for (i, lines, top) in self.flow_lines(y0, y1) {
            let b = &self.blocks[i];
            if let Some(block) = blocks.get(self.block_index[i]) {
                let at = Locator { chapter: self.chapter, block: self.block_index[i], offset: 0 };
                word_block(&mut out, b, block, at, lines, style.margin_h + b.x, top);
            }
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
pub(super) fn draw_block(out: &mut PageContent, b: &BlockLayout, lines: std::ops::Range<usize>, left: f32, top: f32) {
    match &b.content {
        Content::Text(t) => {
            for line in lines.clone() {
                let y = top + (line - lines.start) as f32 * t.line_height + t.baseline;
                push_line(out, &t.lines[line], left, y);
            }
        }
        Content::Empty => {}
        Content::Image { id, w, h } => out.images.push(PageImage { id: id.clone(), rect: Rect { x: left, y: top, w: *w, h: *h } }),
        Content::Row { cells, rules, height, width, .. } => {
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

/// Rectangles over the text `[from, to)` in lines `lines` of block layout `b` (model block `block`), the first line's top
/// at `top`: one per line, a line high.
#[allow(clippy::too_many_arguments)]
fn mark_block(
    out: &mut Vec<Rect>,
    b: &BlockLayout,
    block: usize,
    lines: std::ops::Range<usize>,
    left: f32,
    top: f32,
    from: Locator,
    to: Locator,
) {
    let inside = |offset: usize| (block, offset) >= (from.block, from.offset) && (block, offset) < (to.block, to.offset);
    // The glyphs of one line inside the range, as one rectangle; `shift` is where the line's text starts in the block.
    let span = |line: &super::Line, shift: usize| {
        line.glyphs
            .iter()
            .filter(|g| inside(g.0 as usize + shift))
            .fold(None, |span: Option<(f32, f32)>, &(_, x, w)| Some(span.map_or((x, x + w), |(a, b)| (a.min(x), b.max(x + w)))))
    };
    match &b.content {
        Content::Text(t) => {
            for line in lines.clone() {
                if let Some((a, b)) = span(&t.lines[line], 0) {
                    let y = top + (line - lines.start) as f32 * t.line_height;
                    out.push(Rect { x: left + a, y, w: b - a, h: t.line_height });
                }
            }
        }
        // A table row's text is its cells joined by line breaks (`Table::cell_starts`).
        Content::Row { cells, .. } => {
            for cell in cells {
                for (n, line) in cell.text.lines.iter().enumerate() {
                    if let Some((a, b)) = span(line, cell.offset) {
                        let y = top + cell.y + n as f32 * cell.text.line_height;
                        out.push(Rect { x: left + cell.x + a, y, w: b - a, h: cell.text.line_height });
                    }
                }
            }
        }
        _ => {}
    }
}

/// The words in lines `lines` of block layout `b` (model block `block`, at `at` with offset 0), the first line's top at
/// `top`, the column at `left`.
fn word_block(out: &mut Vec<Word>, b: &BlockLayout, block: &Block, at: Locator, lines: std::ops::Range<usize>, left: f32, top: f32) {
    match (&b.content, block) {
        (Content::Text(t), _) => {
            let Some(text) = block.text() else { return };
            for line in lines.clone() {
                let y = top + (line - lines.start) as f32 * t.line_height;
                line_words(out, &t.lines[line], &text.text, 0, at, left, y, t.line_height);
            }
        }
        // A table row's text is its cells joined by line breaks (`Table::cell_starts`); words stay in their cell.
        (Content::Row { cells, .. }, Block::TableRow { table, row }) => {
            for (cell, c) in cells.iter().zip(&table.rows[*row]) {
                for (n, line) in cell.text.lines.iter().enumerate() {
                    let y = top + cell.y + n as f32 * cell.text.line_height;
                    line_words(out, line, &c.text.text, cell.offset, at, left + cell.x, y, cell.text.line_height);
                }
            }
        }
        _ => {}
    }
}

/// The words of one line over `text` (which starts at `shift` in the block's text): runs of glyphs with no white space
/// between them, each widened over the text to the white space around it.
#[allow(clippy::too_many_arguments)]
fn line_words(out: &mut Vec<Word>, line: &super::Line, text: &str, shift: usize, at: Locator, left: f32, y: f32, h: f32) {
    let space = |range: std::ops::Range<usize>| text.get(range).is_some_and(|t| t.chars().any(char::is_whitespace));
    let is_space = |offset: usize| text.get(offset..).and_then(|t| t.chars().next()).is_none_or(char::is_whitespace);
    // (first offset, last offset, x0, x1) of each piece, offsets in `text`.
    let mut pieces: Vec<(usize, usize, f32, f32)> = Vec::new();
    for &(offset, x, w) in &line.glyphs {
        // Offsets are in the cell's own text (`mark_block` adds the cell's start the same way).
        let offset = offset as usize;
        if offset > text.len() {
            continue;
        }
        if is_space(offset) && offset < text.len() {
            continue;
        }
        match pieces.last_mut() {
            Some(p) if !space(p.1.min(offset)..p.1.max(offset)) => {
                p.1 = p.1.max(offset);
                p.0 = p.0.min(offset);
                p.2 = p.2.min(x);
                p.3 = p.3.max(x + w);
            }
            _ => pieces.push((offset, offset, x, x + w)),
        }
    }
    for (first, last, x0, x1) in pieces {
        let start =
            text[..first.min(text.len())].char_indices().rev().find(|(_, c)| c.is_whitespace()).map_or(0, |(i, c)| i + c.len_utf8());
        let end = text[last.min(text.len())..].char_indices().find(|(_, c)| c.is_whitespace()).map_or(text.len(), |(i, _)| last + i);
        out.push(Word {
            start: Locator { offset: shift + start, ..at },
            end: Locator { offset: shift + end, ..at },
            rect: Rect { x: left + x0, y, w: x1 - x0, h },
        });
    }
}

/// A line of text in `ink` centred on `center` at baseline `y`, shortened with "…" to `max_width`.
#[allow(clippy::too_many_arguments)]
fn label(typesetter: &mut Typesetter, text: &str, ink: Ink, size: f32, center: f32, y: f32, max_width: f32) -> GlyphRun {
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
    let mut run = GlyphRun { font, size, ink, glyphs: Vec::new(), positions: Vec::new() };
    for g in glyphs.iter() {
        run.glyphs.push(g.id as u16);
        run.positions.push([x + g.x_offset as f32 * scale, y - g.y_offset as f32 * scale]);
        x += g.advance as f32 * scale;
    }
    run
}

/// The end page after the book's last page: its title (set like a chapter heading), its authors and "Finished",
/// centred in a window of `style`, without running head or foot.
pub fn end_page(book: &Book, style: &PageStyle, typesetter: &mut Typesetter) -> PageContent {
    let mut out = PageContent::default();
    let size = style.text_size;
    let heading = Block::Heading { text: crate::model::StyledText { text: book.title.clone(), styles: Vec::new() }, level: 1 };
    let title = typesetter.layout(&heading, book);
    let title_height = title.as_ref().map_or(0.0, BlockLayout::height);
    let lh = typesetter.line_height;
    let authors = book.authors.join(", ");
    // Title, a line, the authors, two lines, "Finished".
    let height = title_height + if authors.is_empty() { 0.0 } else { 2.0 * lh } + 3.0 * lh;
    let mut y = ((style.height - height) / 2.0).max(style.margin_top);
    if let Some(title) = &title {
        draw_block(&mut out, title, 0..title.line_count(), style.margin_h + title.x, y);
        y += title_height + lh;
    }
    let (center, width) = (style.width / 2.0, style.text_width());
    if !authors.is_empty() {
        out.runs.push(label(typesetter, &authors, Ink::Text, size, center, y + size, width));
        y += 2.0 * lh;
    }
    out.runs.push(label(typesetter, crate::i18n::tr("Finished"), Ink::Secondary, size * 0.85, center, y + size, width));
    out
}

/// Note `id` typeset as one tall page at `style`'s width: its text (the shell shows the title as the panel's label), the
/// chapter start not lowered. Draw it with `render_body`; images are the book's.
pub fn layout_note(book: &Book, id: &str, style: &PageStyle, typesetter: &mut Typesetter) -> Option<ChapterLayout> {
    Some(layout_blocks(book, book.notes.get(id)?.blocks.clone(), style, typesetter))
}

/// The short target of link `key` (`Book::preview`) typeset like a note, for the note sheet.
pub fn layout_preview(book: &Book, key: &str, style: &PageStyle, typesetter: &mut Typesetter) -> Option<ChapterLayout> {
    Some(layout_blocks(book, book.preview(key)?.to_vec(), style, typesetter))
}

/// `blocks` of `book` typeset as one tall page, the start not lowered.
fn layout_blocks(book: &Book, blocks: Vec<Block>, style: &PageStyle, typesetter: &mut Typesetter) -> ChapterLayout {
    let images = blocks
        .iter()
        .filter_map(|b| if let Block::Image { id } = b { book.images.get_key_value(id) } else { None })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let note_book = Book { language: book.language.clone(), images, chapters: vec![Chapter { title: None, blocks }], ..Book::default() };
    ChapterLayout { viewable: false, ..paginate_from(&note_book, 0, style, typesetter, 0.0) }
}

fn push_line(out: &mut PageContent, line: &super::Line, x: f32, y: f32) {
    out.runs.extend(line.runs.iter().map(|r| r.offset(x, y)));
    out.areas.extend(line.areas.iter().map(|a| Area { rect: a.rect.offset(x, y), target: a.target.clone() }));
    out.strokes.extend(line.strokes.iter().map(|s| Stroke { rect: s.rect.offset(x, y), ..*s }));
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
    let book_title = Some(book.title.clone()).filter(|t| !t.is_empty());
    let block_index = laid.iter().map(|(i, _)| *i).collect();
    ChapterLayout {
        chapter,
        blocks: laid.into_iter().map(|(_, l)| l).collect(),
        pages,
        title,
        book_title,
        position,
        block_index,
        flow,
        flow_height,
        viewable: true,
    }
}
