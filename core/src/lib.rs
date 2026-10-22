//! Forzac core: the book model, parsers and the typesetting engine shared by the desktop shells
//! (GTK 4 on Linux, AppKit on macOS). Shells get pages as positioned glyph runs and only draw them.

mod archive;
pub mod bookmarks;
pub mod epub;
pub mod fb2;
pub mod fb3;
pub mod ffi;
pub mod fonts;
pub mod hyphen;
pub mod i18n;
pub mod kindle;
pub mod knuth_plass;
mod language;
pub mod layout;
pub mod library;
pub mod log;
mod markdown;
pub mod model;
pub mod quotes;
pub mod search;
pub mod selection;
pub mod settings;
pub mod state;
pub mod stats;
pub mod sync;
mod text;
mod typography;
pub mod update;
mod xml;

pub use fonts::Face;
pub use layout::{
    Area, BookPages, ChapterLayout, CountControl, EndLines, GlyphRun, Ink, PageContent, PageImage, PageStyle, Rect, Rule, Shape, Stroke,
    TableView, Target, Typesetter, Word, end_page, layout_note, layout_preview, layout_table, paginate,
};
pub use model::{Block, Book, Chapter, Locator};

/// Why a file is not a readable book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not a book in a known format, a damaged archive, or an archive that unpacks too large.
    NotABook,
    /// A well-formed book without any text.
    NoText,
    /// The book's content is encrypted (DRM).
    Protected,
    /// A zip archive that cannot be read as one.
    Damaged,
    /// The file cannot be read: missing, no permission.
    Unavailable,
}

/// Reads and parses a book file: FB2 (`.fb2`, `.fb2.zip`), FB3, EPUB, plain text or Markdown, whatever its name says.
/// A book without a title is titled by the file's name ([`library::file_title`]).
pub fn open(path: &std::path::Path) -> Result<Book, Error> {
    let bytes = std::fs::read(path).map_err(|_| Error::Unavailable)?;
    let mut book = parse(&bytes)?;
    if book.title.trim().is_empty() {
        book.title = library::file_title(&path.file_name().unwrap_or_default().to_string_lossy());
    }
    Ok(book)
}

/// Parses a book, the format decided by the content: a zip with `META-INF/container.xml` is EPUB, a zip with a
/// FictionBook 3 relationship is FB3, anything else is tried as FB2, then as text (plain or Markdown, `text`). A book
/// that names no language gets one from its text.
pub fn parse(bytes: &[u8]) -> Result<Book, Error> {
    let mut book = parse_format(bytes)?;
    book.id = state::book_id(bytes);
    if book.language.as_deref().is_none_or(|l| l.trim().is_empty()) {
        book.language = Some(language::guess(&book).to_owned());
    }
    Ok(book)
}

fn parse_format(bytes: &[u8]) -> Result<Book, Error> {
    // A zip archive is a package (EPUB, FB3) or holds an FB2; one that cannot be read as an archive is damaged.
    if bytes.starts_with(b"PK") {
        let files = archive::unzip(bytes, archive::MAX_UNPACKED)?;
        if files.contains_key(epub::CONTAINER) {
            return epub::read(&files);
        }
        if fb3::is_fb3(&files) {
            return fb3::read(&files);
        }
    }
    match fb2::parse(bytes) {
        Err(Error::NotABook) if text::is_text(bytes) => text::parse(bytes),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn why_a_file_cannot_be_opened() {
        use crate::Error;
        assert_eq!(crate::open(std::path::Path::new("/nonexistent/book.fb2")).err(), Some(Error::Unavailable));
        assert_eq!(crate::parse(b"PK\x03\x04 broken archive").err(), Some(Error::Damaged));
        assert_eq!(crate::parse(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR not a book").err(), Some(Error::NotABook));
    }
}
