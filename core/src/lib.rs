//! Forzac core: the book model, parsers and the typesetting engine shared by the desktop shells
//! (GTK 4 on Linux, AppKit on macOS). Shells get pages as positioned glyph runs and only draw them.

mod archive;
pub mod epub;
pub mod fb2;
pub mod fb3;
pub mod ffi;
pub mod fonts;
pub mod hyphen;
pub mod knuth_plass;
mod language;
pub mod layout;
pub mod model;
pub mod search;
pub mod settings;
pub mod state;
mod typography;
mod xml;

pub use fonts::Face;
pub use layout::{
    Area, ChapterLayout, GlyphRun, Ink, PageContent, PageImage, PageStyle, Rect, Rule, Stroke, Target, Typesetter, end_page, layout_note,
    layout_preview, paginate,
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

/// Reads and parses a book file: FB2 (`.fb2`, `.fb2.zip`), FB3 or EPUB, whatever its name says.
pub fn open(path: &std::path::Path) -> Result<Book, Error> {
    let bytes = std::fs::read(path).map_err(|_| Error::Unavailable)?;
    parse(&bytes)
}

/// Parses a book, the format decided by the content: a zip with `META-INF/container.xml` is EPUB, a zip with a
/// FictionBook 3 relationship is FB3, anything else is tried as FB2. A book that names no language gets one from
/// its text.
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
    fb2::parse(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn why_a_file_cannot_be_opened() {
        use crate::Error;
        assert_eq!(crate::open(std::path::Path::new("/nonexistent/book.fb2")).err(), Some(Error::Unavailable));
        assert_eq!(crate::parse(b"PK\x03\x04 broken archive").err(), Some(Error::Damaged));
        assert_eq!(crate::parse(b"\x89PNG not a book").err(), Some(Error::NotABook));
    }
}
