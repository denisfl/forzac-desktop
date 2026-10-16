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
pub mod layout;
pub mod model;
pub mod search;
pub mod settings;
pub mod state;
mod typography;
mod xml;

pub use fonts::Face;
pub use layout::{
    Area, ChapterLayout, GlyphRun, Ink, PageContent, PageImage, PageStyle, Rect, Rule, Target, Typesetter, layout_note, paginate,
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
}

/// Reads and parses a book file: FB2 (`.fb2`, `.fb2.zip`), FB3 or EPUB, whatever its name says.
pub fn open(path: &std::path::Path) -> Result<Book, Error> {
    let bytes = std::fs::read(path).map_err(|_| Error::NotABook)?;
    parse(&bytes)
}

/// Parses a book, the format decided by the content: a zip with `META-INF/container.xml` is EPUB, a zip with a
/// FictionBook 3 relationship is FB3, anything else is tried as FB2.
pub fn parse(bytes: &[u8]) -> Result<Book, Error> {
    let mut book = parse_format(bytes)?;
    book.id = state::book_id(bytes);
    Ok(book)
}

fn parse_format(bytes: &[u8]) -> Result<Book, Error> {
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
