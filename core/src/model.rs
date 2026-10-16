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

    /// How much text the block holds (bytes; table rows count their cells): the measure of progress through a book.
    pub fn text_len(&self) -> usize {
        match self {
            Block::TableRow { table, row } => table.rows[*row].iter().map(|c| c.text.text.len()).sum(),
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
