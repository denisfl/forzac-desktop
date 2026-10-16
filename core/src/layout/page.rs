//! Pagination (a port of the Android `Paginator`) and page contents.

use super::{Area, BlockLayout, Content, GlyphRun, Group, Ink, PageStyle, Rect, Typesetter};
use crate::fonts::Face;
use crate::model::{Book, Locator};

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

    /// What to draw on page `i`, with the page number at the bottom.
    pub fn render(&self, i: usize, style: &PageStyle, typesetter: &mut Typesetter) -> PageContent {
        let mut out = PageContent::default();
        for s in &self.pages[i].slices {
            let Some(b) = self.blocks.get(s.layout) else { continue };
            let left = style.margin_h + b.x;
            let top = style.margin_top + s.top;
            match &b.content {
                Content::Text(t) => {
                    for line in s.line_start..s.line_end {
                        let y = top + (line - s.line_start) as f32 * t.line_height + t.baseline;
                        push_line(&mut out, &t.lines[line], left, y);
                    }
                }
                Content::Empty => {}
                Content::Image { id, w, h } => out.images.push(PageImage { id: id.clone(), rect: Rect { x: left, y: top, w: *w, h: *h } }),
                Content::Row { cells, rules, height, width } => {
                    for cell in cells {
                        for (n, line) in cell.text.lines.iter().enumerate() {
                            let y = top + cell.y + n as f32 * cell.text.line_height + cell.text.baseline;
                            push_line(&mut out, line, left + cell.x, y);
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
        // Page number.
        let label = format!("{} / {}", i + 1, self.pages.len());
        let size = style.text_size * 0.7;
        let font = Face::Regular.id();
        let scale = size / typesetter.shaper.units_per_em(font);
        let glyphs = typesetter.shaper.shape(font, &label);
        let width: f32 = glyphs.iter().map(|g| g.advance as f32 * scale).sum();
        let mut x = (style.width - width) / 2.0;
        let y = style.height - style.margin_bottom / 2.0 + size * 0.35;
        let mut run = GlyphRun { font, size, ink: Ink::Secondary, glyphs: Vec::new(), positions: Vec::new() };
        for g in glyphs.iter() {
            run.glyphs.push(g.id as u16);
            run.positions.push([x, y]);
            x += g.advance as f32 * scale;
        }
        out.runs.push(run);
        out
    }
}

fn push_line(out: &mut PageContent, line: &super::Line, x: f32, y: f32) {
    out.runs.extend(line.runs.iter().map(|r| r.offset(x, y)));
    out.areas.extend(line.areas.iter().map(|a| Area { rect: a.rect.offset(x, y), target: a.target.clone() }));
}

/// Paginates a chapter: a port of the Android `Paginator`, which needs only line counts and heights from its layouts.
pub fn paginate(book: &Book, chapter: usize, style: &PageStyle, typesetter: &mut Typesetter) -> ChapterLayout {
    let model = &book.chapters[chapter].blocks;
    // Layouts paired with their block index in the chapter: blocks that take no room have no layout.
    let laid: Vec<(usize, BlockLayout)> =
        model.iter().enumerate().filter_map(|(i, b)| typesetter.layout(b, book).map(|l| (i, l))).collect();
    let blocks: Vec<&BlockLayout> = laid.iter().map(|(_, l)| l).collect();
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
    ChapterLayout { chapter, blocks: laid.into_iter().map(|(_, l)| l).collect(), pages }
}
