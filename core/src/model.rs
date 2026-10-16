//! Book model shared by the parsers and the typesetter.
//!
//! Mirrors the Android app's `:core:model` (Forzac for Android), trimmed to what the spike renders.

/// A parsed book: metadata and chapters. Chapters are what the reader paginates one at a time.
#[derive(Debug, Clone, Default)]
pub struct Book {
    pub title: String,
    pub authors: Vec<String>,
    /// BCP 47 tag from the book, e.g. `ru`. Selects hyphenation patterns.
    pub language: Option<String>,
    pub chapters: Vec<Chapter>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleRange {
    pub start: usize,
    pub end: usize,
    pub style: Style,
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
}

/// A place in the book: chapter, block in the chapter, byte offset in the block's text.
/// The reading position is always a locator, never a page number: pages depend on settings and window size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Locator {
    pub chapter: usize,
    pub block: usize,
    pub offset: usize,
}
