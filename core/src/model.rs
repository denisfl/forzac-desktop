//! Book model shared by the parsers and the typesetter.
//!
//! Mirrors the Android app's `:core:model` (Forzac for Android).

use std::collections::HashMap;
use std::sync::Arc;

/// A parsed book: metadata, chapters and what they refer to. Chapters are what the reader paginates one at a time.
#[derive(Debug, Clone, Default)]
pub struct Book {
    /// Identity by content, for the reading state: a renamed or moved file is the same book.
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    /// BCP 47 tag from the book, e.g. `ru`. Selects hyphenation patterns.
    pub language: Option<String>,
    /// The book's own description, paragraphs joined by line breaks.
    pub annotation: Option<String>,
    /// Image id of the cover, if the book names one.
    pub cover: Option<String>,
    /// The series the book belongs to, if it names one.
    pub series: Option<Series>,
    /// Genres: FB2's codes by their English names (`i18n::genre`), EPUB's and FB3's subjects as written.
    pub genres: Vec<String>,
    /// The year it was written or published.
    pub year: Option<u32>,
    pub chapters: Vec<Chapter>,
    /// Footnotes and comments by id.
    pub notes: HashMap<String, Note>,
    /// Link targets by the key in [`StyleRange::target`] of [`Style::Link`] ranges.
    pub links: HashMap<String, LinkTarget>,
    /// Titled sections in reading order.
    pub toc: Vec<TocEntry>,
    /// Images by id (in FB2 the `<binary>` id).
    pub images: HashMap<String, Image>,
}

/// A series and the book's number in it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Series {
    pub name: String,
    /// May be fractional ("2.5"); none when the book gives no number.
    pub number: Option<f32>,
}

impl Series {
    /// `name` and `number` as books write them; none for an empty name. A number that is not one is dropped.
    pub fn parse(name: &str, number: Option<&str>) -> Option<Series> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        let number = number.and_then(|n| n.trim().replace(',', ".").parse::<f32>().ok()).filter(|n| n.is_finite() && *n >= 0.0);
        (!name.is_empty()).then_some(Series { name, number })
    }

    /// The number as shown: "2", "2.5".
    pub fn number_label(&self) -> Option<String> {
        self.number.map(|n| if n.fract() == 0.0 { format!("{}", n as u32) } else { format!("{n}") })
    }
}

/// The first year (four digits, 1000–2999) in `text`: "1962-01-01", "1962", "c. 1962".
pub fn year_in(text: &str) -> Option<u32> {
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(3)).find_map(|i| {
        let run = &bytes[i..i + 4];
        let bounded = (i == 0 || !bytes[i - 1].is_ascii_digit()) && bytes.get(i + 4).is_none_or(|b| !b.is_ascii_digit());
        (bounded && run.iter().all(u8::is_ascii_digit) && (b'1'..=b'2').contains(&run[0])).then(|| text[i..i + 4].parse().ok()).flatten()
    })
}

impl Book {
    /// The entries of the contents: the book's own, else every chapter, an untitled one as "Chapter N" in the interface
    /// language (as on Android).
    pub fn contents(&self) -> Vec<TocEntry> {
        if !self.toc.is_empty() {
            return self.toc.clone();
        }
        self.chapters
            .iter()
            .enumerate()
            .map(|(i, c)| TocEntry {
                title: c
                    .title
                    .clone()
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or_else(|| crate::i18n::fill(crate::i18n::tr("Chapter {n}"), &[("n", &(i + 1).to_string())])),
                level: 1,
                target: Locator { chapter: i, ..Locator::default() },
            })
            .collect()
    }
}

/// The most blocks and characters of text a link target may have to be shown in the note sheet instead of jumped to.
const PREVIEW_BLOCKS: usize = 3;
const PREVIEW_CHARS: usize = 600;

impl Book {
    /// The blocks of link `key`'s target when it is short — a paragraph, a figure with its caption, a small table —
    /// to show over the page instead of jumping (Android's `Book.preview`): within one chapter, at most 3 blocks and
    /// 600 characters of text. `None` for a long target, a point in the text, a note or an outside address.
    pub fn preview(&self, key: &str) -> Option<&[Block]> {
        let LinkTarget::Place { start, end: Some(end) } = self.links.get(key)? else { return None };
        if end.chapter != start.chapter {
            return None;
        }
        let blocks = &self.chapters.get(start.chapter)?.blocks;
        let to = end.block.min(blocks.len());
        if start.block >= to || to - start.block > PREVIEW_BLOCKS {
            return None;
        }
        let target = &blocks[start.block..to];
        let chars: usize = target
            .iter()
            .map(|b| match b {
                Block::TableRow { table, row } => table.rows[*row].iter().map(|c| c.text.text.chars().count() + 1).sum::<usize>() - 1,
                b => b.text().map_or(0, |t| t.text.chars().count()),
            })
            .sum();
        (chars <= PREVIEW_CHARS).then_some(target)
    }
}

impl Book {
    /// The text of `[from, to)`, at most `limit` characters: blocks joined by line breaks (a table row's cells too),
    /// without note markers and soft hyphens, no-break spaces as spaces, trimmed.
    pub fn text(&self, from: Locator, to: Locator, limit: usize) -> String {
        let mut out = String::new();
        let mut count = 0;
        'chapters: for (c, chapter) in self.chapters.iter().enumerate().skip(from.chapter) {
            for (b, block) in chapter.blocks.iter().enumerate() {
                let here = Locator { chapter: c, block: b, offset: 0 };
                if (c, b) < (from.chapter, from.block) {
                    continue;
                }
                if here >= to {
                    break 'chapters;
                }
                // The block's text and the byte ranges of its note markers.
                let (text, notes): (String, Vec<(usize, usize)>) = match block {
                    Block::TableRow { table, row } => {
                        let cells = &table.rows[*row];
                        let starts = table.cell_starts(*row);
                        let text = cells.iter().map(|c| c.text.text.as_str()).collect::<Vec<_>>().join("\n");
                        let notes =
                            cells.iter().zip(&starts).flat_map(|(c, &s)| note_ranges(&c.text).map(move |(a, b)| (a + s, b + s))).collect();
                        (text, notes)
                    }
                    b => match b.text() {
                        Some(t) => (t.text.clone(), note_ranges(t).collect()),
                        None => continue,
                    },
                };
                let start = if (c, b) == (from.chapter, from.block) { from.offset } else { 0 };
                let end = if (c, b) == (to.chapter, to.block) { to.offset.min(text.len()) } else { text.len() };
                if start >= end {
                    continue;
                }
                if !out.is_empty() {
                    out.push('\n');
                }
                for (i, ch) in text.char_indices() {
                    if i < start || i >= end || notes.iter().any(|&(a, b)| i >= a && i < b) {
                        continue;
                    }
                    match ch {
                        '\u{AD}' | '\r' => continue,
                        '\u{A0}' | '\u{202F}' => out.push(' '),
                        ch => out.push(ch),
                    }
                    count += 1;
                    if count >= limit {
                        break 'chapters;
                    }
                }
            }
        }
        out.trim().to_owned()
    }
}

impl Book {
    /// The block holding `at`, as a range of the text (a paragraph, a heading, a table row).
    pub fn block_range(&self, at: Locator) -> (Locator, Locator) {
        let len = self.chapters.get(at.chapter).and_then(|c| c.blocks.get(at.block)).map_or(0, Block::text_len);
        (Locator { offset: 0, ..at }, Locator { offset: len, ..at })
    }
}

/// Characters of text read in a minute, for the time left in a chapter (Android's and the web reader's estimate).
const CHARS_PER_MINUTE: usize = 1000;

impl Book {
    /// The title of the section `at` is in: the contents entry at or before it, else the book's title (the menu's
    /// title, the web reader's `chapterTitle`).
    pub fn section_at(&self, at: Locator) -> String {
        self.contents().into_iter().rev().find(|e| e.target <= at).map_or_else(|| self.title.clone(), |e| e.title)
    }

    /// Minutes of reading from `at` to the end of its chapter: its characters at 1000 a minute, rounded.
    pub fn minutes_left(&self, at: Locator) -> usize {
        let Some(chapter) = self.chapters.get(at.chapter) else { return 0 };
        let chars = |b: &Block| b.texts().iter().map(|t| t.text.chars().count()).sum::<usize>();
        let here = chapter.blocks.get(at.block).map_or(0, |b| match b.text() {
            Some(t) => t.text.get(at.offset.min(t.text.len())..).map_or(0, |rest| rest.chars().count()),
            None => chars(b),
        });
        let after: usize = chapter.blocks.iter().skip(at.block + 1).map(chars).sum();
        (here + after + CHARS_PER_MINUTE / 2) / CHARS_PER_MINUTE
    }
}

/// The byte ranges of `t`'s note markers.
fn note_ranges(t: &StyledText) -> impl Iterator<Item = (usize, usize)> + '_ {
    t.styles.iter().filter(|s| s.style == Style::NoteRef).map(|s| (s.start, s.end))
}

#[derive(Debug, Clone, Default)]
pub struct Chapter {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// `level` 0 is the book title, 1 a chapter, 2+ nested sections.
    Heading {
        text: StyledText,
        level: u32,
    },
    Paragraph {
        text: StyledText,
        kind: Kind,
    },
    EmptyLine,
    /// An image by id in [`Book::images`].
    Image {
        id: String,
    },
    /// Row `row` of `table`. Pages break between rows, so each row is a block; the table is shared.
    TableRow {
        table: Arc<Table>,
        row: usize,
    },
}

impl Block {
    /// The block's text, if it has any.
    pub fn text(&self) -> Option<&StyledText> {
        match self {
            Block::Heading { text, .. } | Block::Paragraph { text, .. } => Some(text),
            _ => None,
        }
    }

    /// The block's styled texts: its text, or the cells of a table row.
    pub fn texts(&self) -> Vec<&StyledText> {
        match self {
            Block::TableRow { table, row } => table.rows[*row].iter().map(|c| &c.text).collect(),
            b => b.text().into_iter().collect(),
        }
    }

    /// How much text the block holds (bytes; a table row's cells joined by line breaks, as `Table::cell_starts`
    /// counts them): the measure of progress through a book.
    pub fn text_len(&self) -> usize {
        match self {
            Block::TableRow { table, row } => {
                let cells = &table.rows[*row];
                cells.iter().map(|c| c.text.text.len()).sum::<usize>() + cells.len().saturating_sub(1)
            }
            b => b.text().map_or(0, |t| t.text.len()),
        }
    }
}

/// What a paragraph is, which decides its indent, alignment, face and spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Body,
    Epigraph,
    Verse,
    Cite,
    Subtitle,
    TextAuthor,
    /// Preformatted text: line breaks and spaces are kept.
    Code,
}

/// Plain text plus style ranges. Ranges are byte offsets into `text`, always on char boundaries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyledText {
    pub text: String,
    pub styles: Vec<StyleRange>,
}

/// `[start, end)` in bytes of [`StyledText::text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleRange {
    pub start: usize,
    pub end: usize,
    pub style: Style,
    /// For [`Style::NoteRef`] the note id, for [`Style::Link`] the key in [`Book::links`].
    pub target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    Emphasis,
    Strong,
    Superscript,
    Subscript,
    Strikethrough,
    Code,
    /// Footnote marker; rendered as a superscript.
    NoteRef,
    Link,
}

/// A footnote or comment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Note {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
}

/// Where a link leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// A place in the book; `end` is where the target element ends (exclusive), `None` across chapters.
    Place { start: Locator, end: Option<Locator> },
    /// A note in [`Book::notes`].
    Note(String),
    /// An address outside the book: `http`, `https`, `mailto`.
    External(String),
}

/// A titled section; `level` 1 is a top-level section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub title: String,
    pub level: u32,
    pub target: Locator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// A table: rows of cells. Compared by identity of the shared `Arc`, so its geometry is computed once.
#[derive(Debug, PartialEq)]
pub struct Table {
    pub rows: Vec<Vec<TableCell>>,
}

impl Table {
    /// Where each cell of `row` starts in the row's text: its cells joined by line breaks, as on Android. A `Locator`
    /// in a table row is an offset into that text.
    pub fn cell_starts(&self, row: usize) -> Vec<usize> {
        let mut at = 0;
        self.rows[row]
            .iter()
            .map(|c| {
                let start = at;
                at += c.text.text.len() + 1;
                start
            })
            .collect()
    }

    /// Rows where the header is bold text instead of `th` (a converter's habit): when no row is a header and every
    /// non-empty cell of the first row is entirely bold, the first row becomes the header.
    pub fn with_bold_header(mut rows: Vec<Vec<TableCell>>) -> Vec<Vec<TableCell>> {
        if rows.len() < 2 || rows.iter().any(|row| row.iter().all(|c| c.header)) {
            return rows;
        }
        let first = &rows[0];
        let bold = first.iter().all(|cell| {
            let t = &cell.text;
            t.text.is_empty() || t.styles.iter().any(|s| s.style == Style::Strong && s.start == 0 && s.end >= t.text.len())
        });
        if bold && first.iter().any(|c| !c.text.text.is_empty()) {
            rows[0].iter_mut().for_each(|c| c.header = true);
        }
        rows
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub text: StyledText,
    pub header: bool,
    pub col_span: usize,
}

/// A place in the book: chapter, block in the chapter, byte offset in the block's text.
/// The reading position is always a locator, never a page number: pages depend on settings and window size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Locator {
    pub chapter: usize,
    pub block: usize,
    pub offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn years_and_series_as_books_write_them() {
        assert_eq!(year_in("1962-01-01"), Some(1962));
        assert_eq!(year_in("c. 1962, 2nd ed. 1970"), Some(1962));
        assert_eq!(year_in("12345"), None);
        assert_eq!(year_in("0999"), None);
        assert_eq!(Series::parse("  Мир   Полудня ", Some("3,5")).unwrap(), Series { name: "Мир Полудня".into(), number: Some(3.5) });
        assert_eq!(Series::parse("X", Some("abc")).unwrap().number, None);
        assert_eq!(Series::parse(" ", Some("1")), None);
        assert_eq!(Series::parse("X", Some("4.0")).unwrap().number_label().as_deref(), Some("4"));
    }

    #[test]
    fn contents_without_a_toc_list_every_chapter() {
        let chapter = |title: Option<&str>| Chapter { title: title.map(Into::into), blocks: Vec::new() };
        let book = Book { chapters: vec![chapter(None), chapter(Some("Вторая")), chapter(Some(" "))], ..Book::default() };
        let titles: Vec<String> = book.contents().into_iter().map(|e| e.title).collect();
        assert_eq!(titles, ["Chapter 1", "Вторая", "Chapter 3"]);
        assert_eq!(book.contents()[2].target, Locator { chapter: 2, ..Locator::default() });
    }

    #[test]
    fn time_left_and_section() {
        let para = |text: String| Block::Paragraph { text: StyledText { text, styles: Vec::new() }, kind: Kind::Body };
        let book = Book {
            title: "Книга".into(),
            chapters: vec![Chapter { title: None, blocks: vec![para("ж".repeat(1200)), para("ж".repeat(2300))] }],
            toc: vec![TocEntry { title: "Вторая часть".into(), level: 1, target: Locator { chapter: 0, block: 1, offset: 0 } }],
            ..Book::default()
        };
        let at = |block: usize, chars: usize| Locator { chapter: 0, block, offset: chars * 2 }; // "ж" is two bytes
        assert_eq!(book.minutes_left(at(0, 0)), 4, "3500 characters");
        assert_eq!(book.minutes_left(at(0, 1000)), 3, "2500 characters, by characters, not bytes");
        assert_eq!(book.minutes_left(at(1, 1900)), 0, "400 characters");
        assert_eq!(book.section_at(at(0, 5)), "Книга", "before the first entry: the book");
        assert_eq!(book.section_at(at(1, 5)), "Вторая часть");
    }

    #[test]
    fn short_targets_preview() {
        let para = |n: usize| Block::Paragraph { text: StyledText { text: "а".repeat(n), styles: Vec::new() }, kind: Kind::Body };
        let at = |chapter: usize, block: usize| Locator { chapter, block, offset: 0 };
        let place = |start: Locator, end: Option<Locator>| LinkTarget::Place { start, end };
        let links = [
            ("figure", place(at(0, 1), Some(at(0, 3)))),
            ("chapter", place(at(0, 0), Some(at(0, 5)))),
            ("long", place(at(0, 4), Some(at(0, 5)))),
            ("across", place(at(0, 4), Some(at(1, 0)))),
            ("point", place(at(0, 2), None)),
            ("note", LinkTarget::Note("n1".into())),
        ];
        let book = Book {
            chapters: vec![
                Chapter { title: None, blocks: vec![para(10), Block::Image { id: "fig".into() }, para(40), para(10), para(601)] },
                Chapter { title: None, blocks: vec![para(10)] },
            ],
            links: links.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
            ..Book::default()
        };
        assert_eq!(book.preview("figure").map(<[Block]>::len), Some(2), "an image with its caption");
        for key in ["chapter", "long", "across", "point", "note", "missing"] {
            assert!(book.preview(key).is_none(), "{key}");
        }
    }
}
