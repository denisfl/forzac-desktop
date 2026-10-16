//! Forzac core: the book model, parsers and the typesetting engine shared by the desktop shells
//! (GTK 4 on Linux, AppKit on macOS). Shells get pages as positioned glyph runs and only draw them.

pub mod fb2;
pub mod ffi;
pub mod fonts;
pub mod hyphen;
pub mod knuth_plass;
pub mod layout;
pub mod model;
mod typography;

pub use fonts::Face;
pub use layout::{Area, ChapterLayout, GlyphRun, Ink, PageContent, PageImage, PageStyle, Rect, Rule, Target, Typesetter, paginate};
pub use model::{Block, Book, Chapter, Locator};

/// Reads and parses a book file. The spike reads FB2 and `.fb2.zip`; EPUB and FB3 come next.
pub fn open(path: &std::path::Path) -> Result<Book, fb2::Error> {
    let bytes = std::fs::read(path).map_err(|_| fb2::Error::NotABook)?;
    fb2::parse(&bytes)
}
