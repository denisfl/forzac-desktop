//! C ABI for the macOS shell (Swift). Small on purpose; the full version may move to UniFFI.
//!
//! A document owns the book, the typesetter and the current chapter layout. Pointers in a [`ForzacPage`] stay valid
//! until the next call on the same document.

use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::fonts::{self, FontId, FontSource};
use crate::hyphen::Hyphenator;
use crate::layout::{ChapterLayout, PageContent, PageStyle, Rect, Target, Typesetter, paginate};
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
    /// Font id: 0–3 the bundled faces (regular, italic, bold, bold italic), later ids fallback fonts.
    pub font: u16,
    /// 0 text, 1 link, 2 secondary.
    pub ink: u8,
    pub size: f32,
    pub count: u32,
    pub glyphs: *const u16,
    /// `count` pairs of x, y: glyph origins on the baseline, y down, in logical pixels.
    pub positions: *const f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl From<Rect> for ForzacRect {
    fn from(r: Rect) -> Self {
        ForzacRect { x: r.x, y: r.y, w: r.w, h: r.h }
    }
}

/// An image: the file's bytes (owned by the document, the same pointer for the same image) and where to draw it.
#[repr(C)]
pub struct ForzacImage {
    pub bytes: *const u8,
    pub len: usize,
    pub rect: ForzacRect,
}

/// A table rule; `strength` 1 light, 2 medium, 3 heavy.
#[repr(C)]
pub struct ForzacRule {
    pub rect: ForzacRect,
    pub strength: u8,
}

/// A note marker (`kind` 0, `target` the note id) or a link (`kind` 1, `target` the link key).
#[repr(C)]
pub struct ForzacArea {
    pub rect: ForzacRect,
    pub kind: u8,
    pub target: *const c_char,
}

#[repr(C)]
pub struct ForzacPage {
    pub runs: *const ForzacRun,
    pub run_count: u32,
    pub images: *const ForzacImage,
    pub image_count: u32,
    pub rules: *const ForzacRule,
    pub rule_count: u32,
    pub areas: *const ForzacArea,
    pub area_count: u32,
}

pub struct Document {
    book: Book,
    path: PathBuf,
    title: CString,
    hyphenator: Option<Arc<Hyphenator>>,
    style: Option<PageStyle>,
    typesetter: Option<Typesetter>,
    chapter: Option<ChapterLayout>,
    page: PageContent,
    runs: Vec<ForzacRun>,
    images: Vec<ForzacImage>,
    rules: Vec<ForzacRule>,
    areas: Vec<ForzacArea>,
    targets: Vec<CString>,
}

/// Opens a book; null when the file is not a readable book.
///
/// # Safety
/// `path` must be a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_open(path: *const c_char) -> *mut Document {
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else { return std::ptr::null_mut() };
    let Ok(book) = crate::open(Path::new(path)) else { return std::ptr::null_mut() };
    let title = CString::new(book.title.replace('\0', "")).unwrap_or_default();
    let hyphenator = Hyphenator::for_language(book.language.as_deref()).map(Arc::new);
    Box::into_raw(Box::new(Document {
        book,
        path: PathBuf::from(path),
        title,
        hyphenator,
        style: None,
        typesetter: None,
        chapter: None,
        page: PageContent::default(),
        runs: Vec::new(),
        images: Vec::new(),
        rules: Vec::new(),
        areas: Vec::new(),
        targets: Vec::new(),
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

/// What to draw on `page` of the last paginated chapter.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page(doc: *mut Document, page: u32) -> ForzacPage {
    let doc = unsafe { &mut *doc };
    if let (Some(chapter), Some(style), Some(t)) = (&doc.chapter, &doc.style, doc.typesetter.as_mut()) {
        doc.page = chapter.render((page as usize).min(chapter.pages.len() - 1), style, t);
    }
    let p = &doc.page;
    doc.runs = p
        .runs
        .iter()
        .map(|r| ForzacRun {
            font: r.font.0,
            ink: r.ink as u8,
            size: r.size,
            count: r.glyphs.len() as u32,
            glyphs: r.glyphs.as_ptr(),
            positions: r.positions.as_ptr().cast(),
        })
        .collect();
    let book = &doc.book;
    doc.images = p
        .images
        .iter()
        .filter_map(|i| book.images.get(&i.id).map(|b| ForzacImage { bytes: b.bytes.as_ptr(), len: b.bytes.len(), rect: i.rect.into() }))
        .collect();
    doc.rules = p.rules.iter().map(|r| ForzacRule { rect: r.rect.into(), strength: r.strength }).collect();
    let (kinds, targets): (Vec<u8>, Vec<CString>) = p
        .areas
        .iter()
        .map(|a| match &a.target {
            Target::Note(id) => (0, CString::new(id.as_str()).unwrap_or_default()),
            Target::Link(key) => (1, CString::new(key.as_str()).unwrap_or_default()),
        })
        .unzip();
    doc.targets = targets;
    doc.areas = p
        .areas
        .iter()
        .zip(kinds)
        .zip(&doc.targets)
        .map(|((a, kind), target)| ForzacArea { rect: a.rect.into(), kind, target: target.as_ptr() })
        .collect();
    ForzacPage {
        runs: doc.runs.as_ptr(),
        run_count: doc.runs.len() as u32,
        images: doc.images.as_ptr(),
        image_count: doc.images.len() as u32,
        rules: doc.rules.as_ptr(),
        rule_count: doc.rules.len() as u32,
        areas: doc.areas.as_ptr(),
        area_count: doc.areas.len() as u32,
    }
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

/// The bundled font file of `font`, so the shell rasterises exactly what was shaped; null for a fallback font.
///
/// # Safety
/// `len` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_font_data(font: u16, len: *mut usize) -> *const u8 {
    let Some(face) = FontId(font).face() else { return std::ptr::null() };
    let data = face.data();
    unsafe { *len = data.len() };
    data.as_ptr()
}

/// The file of fallback font `font`: copies its path (NUL-terminated) into `path` and sets `*index` to the face index
/// in a collection. False for a bundled face, an unknown id or a path longer than `capacity`.
///
/// # Safety
/// `path` must be writable for `capacity` bytes and `index` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_font_file(font: u16, path: *mut c_char, capacity: usize, index: *mut u32) -> bool {
    let Some(FontSource::File { path: file, index: i }) = fonts::font_source(FontId(font)) else { return false };
    let bytes = file.as_os_str().as_encoded_bytes();
    if bytes.len() >= capacity {
        return false;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), path.cast(), bytes.len());
        *path.add(bytes.len()) = 0;
        *index = i;
    }
    true
}

/// Whether `font` has colour glyphs (emoji), to be drawn in their own colours.
#[unsafe(no_mangle)]
pub extern "C" fn forzac_font_is_color(font: u16) -> bool {
    fonts::font_is_color(FontId(font))
}

/// Finds a font file with character `ch`: writes its NUL-terminated path into `path` (`capacity` bytes) and its face
/// index into `*index`; returns false when there is none.
pub type ForzacFallback = extern "C" fn(ch: u32, path: *mut c_char, capacity: usize, index: *mut u32) -> bool;

/// Sets where fallback fonts come from, before the first paginate.
#[unsafe(no_mangle)]
pub extern "C" fn forzac_set_fallback(provider: ForzacFallback) {
    fonts::set_fallback_provider(Box::new(move |ch| {
        let mut buf = vec![0u8; 4096];
        let mut index = 0;
        if !provider(ch as u32, buf.as_mut_ptr().cast(), buf.len(), &mut index) {
            return None;
        }
        let path = CStr::from_bytes_until_nul(&buf).ok()?.to_str().ok()?;
        Some((PathBuf::from(path), index))
    }));
}

/// # Safety
/// `doc` must come from [`forzac_open`]; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_close(doc: *mut Document) {
    if !doc.is_null() {
        drop(unsafe { Box::from_raw(doc) });
    }
}

/// The reading state (positions, settings) in the file at a path the shell chooses.
pub struct State {
    store: crate::state::Store,
    /// The last setting returned, kept alive for the caller.
    value: CString,
}

/// Opens the state file at `path` (it need not exist yet).
///
/// # Safety
/// `path` must be a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_open(path: *const c_char) -> *mut State {
    let path = unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned();
    Box::into_raw(Box::new(State { store: crate::state::Store::open(path), value: CString::default() }))
}

/// Where the document's book was left; false when it is not remembered.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`]; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_position(state: *const State, doc: *const Document, out: *mut ForzacLocator) -> bool {
    let (state, doc) = unsafe { (&*state, &*doc) };
    let Some(l) = state.store.position(&doc.book, &doc.path) else { return false };
    unsafe { *out = ForzacLocator { chapter: l.chapter as u32, block: l.block as u32, offset: l.offset as u32 } };
    true
}

/// Remembers that the document's book is read at `at`; false when the file cannot be written.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_save_position(state: *mut State, doc: *const Document, at: ForzacLocator) -> bool {
    let (state, doc) = unsafe { (&mut *state, &*doc) };
    let at = crate::model::Locator { chapter: at.chapter as usize, block: at.block as usize, offset: at.offset as usize };
    state.store.save_position(&doc.book, &doc.path, at).is_ok()
}

/// A setting's value, valid until the next call on `state`; null when it is not set.
///
/// # Safety
/// `state` from [`forzac_state_open`]; `key` a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_setting(state: *mut State, key: *const c_char) -> *const c_char {
    let state = unsafe { &mut *state };
    let key = unsafe { CStr::from_ptr(key) }.to_string_lossy();
    let Some(value) = state.store.setting(&key) else { return std::ptr::null() };
    state.value = CString::new(value).unwrap_or_default();
    state.value.as_ptr()
}

/// Sets a setting; false when the file cannot be written.
///
/// # Safety
/// `state` from [`forzac_state_open`]; `key` and `value` valid NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_set_setting(state: *mut State, key: *const c_char, value: *const c_char) -> bool {
    let state = unsafe { &mut *state };
    let (key, value) = unsafe { (CStr::from_ptr(key).to_string_lossy(), CStr::from_ptr(value).to_string_lossy()) };
    state.store.set_setting(&key, &value).is_ok()
}

/// # Safety
/// `state` must come from [`forzac_state_open`]; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_close(state: *mut State) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}
