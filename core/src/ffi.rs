//! C ABI for the macOS shell (Swift). Small on purpose; the full version may move to UniFFI.
//!
//! A document owns the book, the typesetter and the current chapter layout. Pointers returned by
//! [`forzac_page_runs`] stay valid until the next call on the same document.

use std::ffi::{CStr, c_char};
use std::path::Path;
use std::rc::Rc;

use crate::fonts::Face;
use crate::hyphen::Hyphenator;
use crate::layout::{ChapterLayout, GlyphRun, PageStyle, Typesetter, paginate};
use crate::model::Book;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacStyle {
    pub width: f32,
    pub height: f32,
    pub text_size: f32,
}

#[repr(C)]
pub struct ForzacRun {
    /// [`Face`] as a number: 0 regular, 1 italic, 2 bold, 3 bold italic.
    pub face: u8,
    pub size: f32,
    pub count: u32,
    pub glyphs: *const u16,
    /// `count` pairs of x, y: glyph origins on the baseline, y down, in logical pixels.
    pub positions: *const f32,
}

pub struct Document {
    book: Book,
    title: std::ffi::CString,
    hyphenator: Option<Rc<Hyphenator>>,
    style: Option<PageStyle>,
    typesetter: Option<Typesetter>,
    chapter: Option<ChapterLayout>,
    runs: Vec<GlyphRun>,
    ffi_runs: Vec<ForzacRun>,
}

/// Opens a book; null when the file is not a readable book.
///
/// # Safety
/// `path` must be a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_open(path: *const c_char) -> *mut Document {
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else { return std::ptr::null_mut() };
    let Ok(book) = crate::open(Path::new(path)) else { return std::ptr::null_mut() };
    let title = std::ffi::CString::new(book.title.replace('\0', "")).unwrap_or_default();
    let hyphenator = Hyphenator::for_language(book.language.as_deref()).map(Rc::new);
    Box::into_raw(Box::new(Document {
        book,
        title,
        hyphenator,
        style: None,
        typesetter: None,
        chapter: None,
        runs: Vec::new(),
        ffi_runs: Vec::new(),
    }))
}

/// # Safety
/// `doc` must come from [`forzac_open`] and not be closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_title(doc: *const Document) -> *const c_char {
    unsafe { &*doc }.title.as_ptr()
}

/// # Safety
/// `doc` must come from [`forzac_open`] and not be closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_chapter_count(doc: *const Document) -> u32 {
    unsafe { &*doc }.book.chapters.len() as u32
}

/// Typesets `chapter` for `style` and returns its page count.
///
/// # Safety
/// `doc` must come from [`forzac_open`]; `style` must point to a valid style.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_paginate(doc: *mut Document, chapter: u32, style: *const ForzacStyle) -> u32 {
    let doc = unsafe { &mut *doc };
    let s = unsafe { *style };
    let page = PageStyle::for_window(s.width, s.height, s.text_size);
    if doc.style != Some(page) {
        doc.typesetter = Some(Typesetter::new(page, doc.hyphenator.clone()));
        doc.style = Some(page);
    }
    let chapter = (chapter as usize).min(doc.book.chapters.len().saturating_sub(1));
    let layout = paginate(&doc.book, chapter, &page, doc.typesetter.as_mut().unwrap());
    let count = layout.pages.len() as u32;
    doc.chapter = Some(layout);
    count
}

/// Glyph runs of `page` of the last paginated chapter; returns the run count and sets `*out`.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`]; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_runs(doc: *mut Document, page: u32, out: *mut *const ForzacRun) -> u32 {
    let doc = unsafe { &mut *doc };
    let (Some(chapter), Some(style), Some(t)) = (&doc.chapter, &doc.style, doc.typesetter.as_mut()) else { return 0 };
    let page = (page as usize).min(chapter.pages.len() - 1);
    doc.runs = chapter.render(page, style, t);
    doc.ffi_runs = doc
        .runs
        .iter()
        .map(|r| ForzacRun {
            face: r.face as u8,
            size: r.size,
            count: r.glyphs.len() as u32,
            glyphs: r.glyphs.as_ptr(),
            positions: r.positions.as_ptr().cast(),
        })
        .collect();
    unsafe { *out = doc.ffi_runs.as_ptr() };
    doc.ffi_runs.len() as u32
}

/// Reading position: chapter, block, byte offset in the block. Pages are derived from it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacLocator {
    pub chapter: u32,
    pub block: u32,
    pub offset: u32,
}

/// Where `page` of the last paginated chapter starts.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_start(doc: *const Document, page: u32) -> ForzacLocator {
    let doc = unsafe { &*doc };
    let Some(chapter) = &doc.chapter else { return ForzacLocator { chapter: 0, block: 0, offset: 0 } };
    let l = chapter.page_start((page as usize).min(chapter.pages.len() - 1));
    ForzacLocator { chapter: l.chapter as u32, block: l.block as u32, offset: l.offset as u32 }
}

/// The page of the last paginated chapter that holds `at`.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_of(doc: *const Document, at: ForzacLocator) -> u32 {
    let doc = unsafe { &*doc };
    let Some(chapter) = &doc.chapter else { return 0 };
    chapter.page_of(crate::model::Locator { chapter: at.chapter as usize, block: at.block as usize, offset: at.offset as usize }) as u32
}

/// The bundled font file for `face`, so the shell rasterises exactly what was shaped.
///
/// # Safety
/// `len` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_font_data(face: u8, len: *mut usize) -> *const u8 {
    let data = Face::ALL.get(face as usize).copied().unwrap_or(Face::Regular).data();
    unsafe { *len = data.len() };
    data.as_ptr()
}

/// # Safety
/// `doc` must come from [`forzac_open`]; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_close(doc: *mut Document) {
    if !doc.is_null() {
        drop(unsafe { Box::from_raw(doc) });
    }
}
