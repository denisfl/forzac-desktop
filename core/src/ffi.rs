//! C ABI for the macOS shell (Swift): plain functions over a document, the reading state and the settings.
//!
//! A document owns the book, the typesetter and the layouts of the chapters typeset for the current style (the current
//! one and a few around it, for "Scroll"). Pointers in a [`ForzacPage`] stay valid until the next call on the same
//! document; strings and lists a function returns stay valid as long as the function says.

use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::fonts::{self, Family, FontId, FontSource};
use crate::hyphen::Hyphenator;
use crate::layout::{
    ChapterLayout, PageContent, PageStyle, Rect, TableView, Target, Typesetter, end_page, layout_note, layout_preview, layout_table,
    paginate,
};
use crate::model::{Book, LinkTarget, Locator};
use crate::settings::{Align, Margins, Setting, Settings, Spacing, TEXT_SIZES, THEMES, Turning};

#[repr(C)]
pub struct ForzacRun {
    /// Font id: 0–3 the bundled faces (regular, italic, bold, bold italic), later ids other families and fallbacks.
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

/// A line through struck-out text, in an ink (0 text, 1 link, 2 secondary).
#[repr(C)]
pub struct ForzacStroke {
    pub rect: ForzacRect,
    pub ink: u8,
}

/// A note marker (`kind` 0, `target` the note id), a link (`kind` 1, `target` the link key), an image that opens in the
/// view (`kind` 2, `target` the image id) or a table (`kind` 3, `at` its first row, `target` empty).
#[repr(C)]
pub struct ForzacArea {
    pub rect: ForzacRect,
    pub kind: u8,
    pub target: *const c_char,
    pub at: ForzacLocator,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacPage {
    pub runs: *const ForzacRun,
    pub run_count: u32,
    pub images: *const ForzacImage,
    pub image_count: u32,
    pub rules: *const ForzacRule,
    pub rule_count: u32,
    pub areas: *const ForzacArea,
    pub area_count: u32,
    pub strokes: *const ForzacStroke,
    pub stroke_count: u32,
}

/// Reading position: chapter, block, byte offset in the block. Pages are derived from it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacLocator {
    pub chapter: u32,
    pub block: u32,
    pub offset: u32,
}

impl From<Locator> for ForzacLocator {
    fn from(l: Locator) -> Self {
        ForzacLocator { chapter: l.chapter as u32, block: l.block as u32, offset: l.offset as u32 }
    }
}

impl From<ForzacLocator> for Locator {
    fn from(l: ForzacLocator) -> Self {
        Locator { chapter: l.chapter as usize, block: l.block as usize, offset: l.offset as usize }
    }
}

/// The reader's settings as indices into the presets (`settings`): `theme` −1 follows the system's light or dark.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacSettings {
    pub theme: i32,
    pub size: u32,
    pub family: u32,
    pub spacing: u32,
    pub align: u32,
    pub margins: u32,
    pub turning: u32,
}

fn pick<T: Copy + Default>(all: &[T], i: u32) -> T {
    all.get(i as usize).copied().unwrap_or_default()
}

fn index<T: PartialEq>(all: &[T], value: T) -> u32 {
    all.iter().position(|v| *v == value).unwrap_or(0) as u32
}

impl From<ForzacSettings> for Settings {
    fn from(s: ForzacSettings) -> Self {
        Settings {
            theme: usize::try_from(s.theme).ok().map(|t| t.min(THEMES.len() - 1)),
            size: (s.size as usize).min(TEXT_SIZES.len() - 1),
            family: pick(&Family::ALL, s.family),
            spacing: pick(&Spacing::ALL, s.spacing),
            align: pick(&Align::ALL, s.align),
            margins: pick(&Margins::ALL, s.margins),
            turning: pick(&Turning::ALL, s.turning),
        }
    }
}

impl From<Settings> for ForzacSettings {
    fn from(s: Settings) -> Self {
        ForzacSettings {
            theme: s.theme.map_or(-1, |t| t as i32),
            size: s.size as u32,
            family: index(&Family::ALL, s.family),
            spacing: index(&Spacing::ALL, s.spacing),
            align: index(&Align::ALL, s.align),
            margins: index(&Margins::ALL, s.margins),
            turning: index(&Turning::ALL, s.turning),
        }
    }
}

/// A contents entry: its title, nesting (1 top level), place and share of the book before it.
#[repr(C)]
pub struct ForzacTocEntry {
    pub title: *const c_char,
    pub level: u32,
    pub at: ForzacLocator,
    pub position: f32,
}

/// How many chapter layouts a document keeps: the current chapter and its neighbours.
const KEPT_LAYOUTS: usize = 5;

pub struct Document {
    /// Shared with searches running on other threads.
    book: Arc<Book>,
    path: PathBuf,
    title: CString,
    authors: CString,
    hyphenator: Option<Arc<Hyphenator>>,
    style: Option<PageStyle>,
    typesetter: Option<Typesetter>,
    /// Layouts for `style`, by chapter.
    layouts: Vec<ChapterLayout>,
    /// The chapter of the last [`forzac_paginate`].
    current: usize,
    page: PageContent,
    published: Published,
    contents: Vec<ForzacTocEntry>,
    contents_titles: Vec<CString>,
    /// The last string returned (a note title, a link's address), kept alive for the caller.
    text: CString,
    /// The book's bookmarks in reading order, and what [`forzac_bookmark_places`] and [`forzac_bookmarks`] last
    /// returned.
    bookmarks: Vec<Locator>,
    bookmark_places: Vec<ForzacLocator>,
    bookmark_list: Vec<ForzacBookmark>,
    bookmark_strings: Vec<CString>,
    /// The words last returned by [`forzac_page_words`] or [`forzac_flow_words`].
    words: Vec<ForzacWord>,
    /// The book's highlights in reading order, and what [`forzac_quote_places`] and [`forzac_quotes`] last returned.
    quotes: Vec<(Locator, Locator)>,
    quote_places: Vec<ForzacRange>,
    quote_list: Vec<ForzacQuote>,
    quote_strings: Vec<CString>,
}

/// A page's contents as the C arrays a [`ForzacPage`] points to.
#[derive(Default)]
struct Published {
    runs: Vec<ForzacRun>,
    images: Vec<ForzacImage>,
    rules: Vec<ForzacRule>,
    strokes: Vec<ForzacStroke>,
    areas: Vec<ForzacArea>,
    targets: Vec<CString>,
}

impl Published {
    /// `p` as pointers into `p` and these arrays, valid until the next call; images are `book`'s.
    fn publish(&mut self, p: &PageContent, book: &Book) -> ForzacPage {
        self.runs = p
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
        self.images = p
            .images
            .iter()
            .filter_map(|i| {
                book.images.get(&i.id).map(|b| ForzacImage { bytes: b.bytes.as_ptr(), len: b.bytes.len(), rect: i.rect.into() })
            })
            .collect();
        self.rules = p.rules.iter().map(|r| ForzacRule { rect: r.rect.into(), strength: r.strength }).collect();
        self.strokes = p.strokes.iter().map(|s| ForzacStroke { rect: s.rect.into(), ink: s.ink as u8 }).collect();
        let none = Locator::default().into();
        let (kinds, targets): (Vec<(u8, ForzacLocator)>, Vec<CString>) = p
            .areas
            .iter()
            .map(|a| match &a.target {
                Target::Note(id) => ((0, none), cstring(id)),
                Target::Link(key) => ((1, none), cstring(key)),
                Target::Image(id) => ((2, none), cstring(id)),
                Target::Table(at) => ((3, (*at).into()), CString::default()),
                Target::Highlight(at) => ((4, (*at).into()), CString::default()),
            })
            .unzip();
        self.targets = targets;
        self.areas = p
            .areas
            .iter()
            .zip(kinds)
            .zip(&self.targets)
            .map(|((a, (kind, at)), target)| ForzacArea { rect: a.rect.into(), kind, target: target.as_ptr(), at })
            .collect();
        ForzacPage {
            runs: self.runs.as_ptr(),
            run_count: self.runs.len() as u32,
            images: self.images.as_ptr(),
            image_count: self.images.len() as u32,
            rules: self.rules.as_ptr(),
            rule_count: self.rules.len() as u32,
            areas: self.areas.as_ptr(),
            area_count: self.areas.len() as u32,
            strokes: self.strokes.as_ptr(),
            stroke_count: self.strokes.len() as u32,
        }
    }
}

impl Document {
    /// The layout of `chapter` for the current style, typeset now if it is not kept.
    fn layout(&mut self, chapter: usize) -> Option<&ChapterLayout> {
        let style = self.style?;
        let chapter = chapter.min(self.book.chapters.len().checked_sub(1)?);
        if !self.layouts.iter().any(|l| l.chapter == chapter) {
            let layout = paginate(&self.book, chapter, &style, self.typesetter.as_mut()?);
            if self.layouts.len() >= KEPT_LAYOUTS {
                let current = self.current;
                if let Some(far) = (0..self.layouts.len()).max_by_key(|&i| self.layouts[i].chapter.abs_diff(current)) {
                    self.layouts.swap_remove(far);
                }
            }
            self.layouts.push(layout);
        }
        self.layouts.iter().find(|l| l.chapter == chapter)
    }

    /// `self.page` as pointers for the caller.
    fn publish(&mut self) -> ForzacPage {
        self.published.publish(&self.page, &self.book)
    }

    /// Keeps `s` alive until the next string and returns it; null for `None`.
    fn text(&mut self, s: Option<&str>) -> *const c_char {
        let Some(s) = s else { return std::ptr::null() };
        self.text = cstring(s);
        self.text.as_ptr()
    }
}

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

/// # Safety
/// `s` must be a valid NUL-terminated string.
unsafe fn str<'a>(s: *const c_char) -> std::borrow::Cow<'a, str> {
    unsafe { CStr::from_ptr(s) }.to_string_lossy()
}

/// Opens a book; null when the file is not a readable book, with the reason in `*error` (when not null): 1 not a
/// book, 2 no text, 3 protected (DRM), 4 a damaged archive, 5 the file cannot be read.
///
/// # Safety
/// `path` must be a valid NUL-terminated string; `error` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_open(path: *const c_char, error: *mut u32) -> *mut Document {
    let fail = |e: crate::Error| {
        if !error.is_null() {
            let code = match e {
                crate::Error::NotABook => 1,
                crate::Error::NoText => 2,
                crate::Error::Protected => 3,
                crate::Error::Damaged => 4,
                crate::Error::Unavailable => 5,
            };
            unsafe { *error = code };
        }
        std::ptr::null_mut()
    };
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else { return fail(crate::Error::Unavailable) };
    let book = match crate::open(Path::new(path)) {
        Ok(book) => book,
        Err(e) => return fail(e),
    };
    let title = cstring(&book.title);
    let authors = cstring(&book.authors.join(", "));
    let hyphenator = Hyphenator::for_language(book.language.as_deref()).map(Arc::new);
    Box::into_raw(Box::new(Document {
        book: Arc::new(book),
        path: PathBuf::from(path),
        title,
        authors,
        hyphenator,
        style: None,
        typesetter: None,
        layouts: Vec::new(),
        current: 0,
        page: PageContent::default(),
        published: Published::default(),
        contents: Vec::new(),
        contents_titles: Vec::new(),
        text: CString::default(),
        bookmarks: Vec::new(),
        bookmark_places: Vec::new(),
        bookmark_list: Vec::new(),
        bookmark_strings: Vec::new(),
        words: Vec::new(),
        quotes: Vec::new(),
        quote_places: Vec::new(),
        quote_list: Vec::new(),
        quote_strings: Vec::new(),
    }))
}

/// # Safety
/// `doc` must come from [`forzac_open`] and not be closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_title(doc: *const Document) -> *const c_char {
    unsafe { &*doc }.title.as_ptr()
}

/// The authors, comma separated; empty when unknown.
///
/// # Safety
/// `doc` must come from [`forzac_open`] and not be closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_authors(doc: *const Document) -> *const c_char {
    unsafe { &*doc }.authors.as_ptr()
}

/// # Safety
/// `doc` must come from [`forzac_open`] and not be closed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_chapter_count(doc: *const Document) -> u32 {
    unsafe { &*doc }.book.chapters.len() as u32
}

/// Typesets `chapter` (or takes its kept layout) for a window of `width` × `height` with `settings`, makes it the
/// current chapter and returns its page count.
///
/// # Safety
/// `doc` must come from [`forzac_open`]; `settings` must point to valid settings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_paginate(
    doc: *mut Document,
    chapter: u32,
    width: f32,
    height: f32,
    settings: *const ForzacSettings,
) -> u32 {
    let doc = unsafe { &mut *doc };
    let style = Settings::from(unsafe { *settings }).page_style(width, height);
    if doc.style != Some(style) {
        doc.typesetter = Some(Typesetter::new(style, doc.hyphenator.clone()));
        doc.style = Some(style);
        doc.layouts.clear();
    }
    doc.current = (chapter as usize).min(doc.book.chapters.len().saturating_sub(1));
    doc.layout(doc.current).map_or(0, |l| l.pages.len() as u32)
}

/// What to draw on `page` of the current chapter.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page(doc: *mut Document, page: u32) -> ForzacPage {
    let doc = unsafe { &mut *doc };
    let current = doc.current;
    if let (Some(style), Some(t)) = (doc.style, doc.typesetter.as_mut())
        && let Some(layout) = doc.layouts.iter().find(|l| l.chapter == current)
    {
        doc.page = layout.render((page as usize).min(layout.pages.len() - 1), &style, t);
    }
    doc.publish()
}

/// Where `page` of the current chapter ends: the next page's start, or the next chapter's start after the last page.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_end(doc: *mut Document, page: u32) -> ForzacLocator {
    let doc = unsafe { &mut *doc };
    let Some(layout) = doc.layout(doc.current) else { return Locator::default().into() };
    layout.page_end((page as usize).min(layout.pages.len() - 1)).into()
}

/// Where `page` of the current chapter starts.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_start(doc: *mut Document, page: u32) -> ForzacLocator {
    let doc = unsafe { &mut *doc };
    let Some(layout) = doc.layout(doc.current) else { return Locator::default().into() };
    layout.page_start((page as usize).min(layout.pages.len() - 1)).into()
}

/// The page of the current chapter that holds `at`.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_of(doc: *mut Document, at: ForzacLocator) -> u32 {
    let doc = unsafe { &mut *doc };
    doc.layout(doc.current).map_or(0, |l| l.page_of(at.into()) as u32)
}

/// The current style's text top margin and line height (logical pixels): the column's top line is at its offset plus
/// the margin; chapters in "Scroll" are four lines apart.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`]; `margin_top` and `line_height` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_metrics(doc: *const Document, margin_top: *mut f32, line_height: *mut f32) {
    let doc = unsafe { &*doc };
    unsafe {
        *margin_top = doc.style.map_or(0.0, |s| s.margin_top);
        *line_height = doc.typesetter.as_ref().map_or(0.0, |t| t.line_height);
    }
}

/// Where the bookmark ribbon hangs: its centre, and the left edge of the corner (in the top margin, to the window's
/// right edge) where a click sets or removes a bookmark.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`]; `centre` and `corner` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_ribbon(doc: *const Document, centre: *mut f32, corner: *mut f32) {
    let (c, k) = unsafe { &*doc }.style.map_or((0.0, f32::MAX), |s| s.ribbon());
    unsafe {
        *centre = c;
        *corner = k;
    }
}

/// The height of `chapter` as one continuous column ("Scroll"), without the page margins.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_flow_height(doc: *mut Document, chapter: u32) -> f32 {
    unsafe { &mut *doc }.layout(chapter as usize).map_or(0.0, |l| l.flow_height())
}

/// What to draw of `chapter`'s column between `y0` and `y1` (column coordinates; the text's top margin is added).
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_flow(doc: *mut Document, chapter: u32, y0: f32, y1: f32) -> ForzacPage {
    let doc = unsafe { &mut *doc };
    if let Some(style) = doc.style {
        doc.page = doc.layout(chapter as usize).map(|l| l.render_flow(y0, y1, &style)).unwrap_or_default();
    }
    doc.publish()
}

/// Where `at` is in its chapter's column.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_flow_y(doc: *mut Document, at: ForzacLocator) -> f32 {
    unsafe { &mut *doc }.layout(at.chapter as usize).map_or(0.0, |l| l.flow_y(at.into()))
}

/// The place at `y` of `chapter`'s column.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_flow_locator(doc: *mut Document, chapter: u32, y: f32) -> ForzacLocator {
    let doc = unsafe { &mut *doc };
    doc.layout(chapter as usize).map_or(ForzacLocator { chapter, block: 0, offset: 0 }, |l| l.flow_locator(y).into())
}

/// The contents: the book's titled sections, else its titled chapters. Valid until the document is closed.
///
/// # Safety
/// `doc` must come from [`forzac_open`]; `count` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_contents(doc: *mut Document, count: *mut u32) -> *const ForzacTocEntry {
    let doc = unsafe { &mut *doc };
    if doc.contents.is_empty() {
        let entries = doc.book.contents();
        let places: Vec<Locator> = entries.iter().map(|e| e.target).collect();
        let positions = crate::state::progress_of(&doc.book, &places);
        doc.contents_titles = entries.iter().map(|e| cstring(&e.title)).collect();
        doc.contents = entries
            .iter()
            .zip(&doc.contents_titles)
            .zip(positions)
            .map(|((e, title), position)| ForzacTocEntry { title: title.as_ptr(), level: e.level, at: e.target.into(), position })
            .collect();
    }
    unsafe { *count = doc.contents.len() as u32 };
    doc.contents.as_ptr()
}

/// The share of the book's text before `at`, 0 to 1.
///
/// # Safety
/// `doc` must come from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_progress(doc: *const Document, at: ForzacLocator) -> f32 {
    crate::state::progress(&unsafe { &*doc }.book, at.into())
}

/// The cover image's file bytes (owned by the document); null when the book has none.
///
/// # Safety
/// `doc` must come from [`forzac_open`]; `len` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_cover(doc: *const Document, len: *mut usize) -> *const u8 {
    let book = &unsafe { &*doc }.book;
    let Some(image) = book.cover.as_ref().and_then(|id| book.images.get(id)) else { return std::ptr::null() };
    unsafe { *len = image.bytes.len() };
    image.bytes.as_ptr()
}

/// Note `id` typeset for a panel `width` wide (the design's note style for `settings`) into `*out`; returns its
/// height, or a negative number when the book has no such note.
///
/// # Safety
/// `doc` from [`forzac_open`]; `id` a valid NUL-terminated string; `settings` valid; `out` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_note(
    doc: *mut Document,
    id: *const c_char,
    width: f32,
    settings: *const ForzacSettings,
    out: *mut ForzacPage,
) -> f32 {
    let doc = unsafe { &mut *doc };
    let id = unsafe { str(id) };
    let style = Settings::from(unsafe { *settings }).note_style(width);
    let mut typesetter = Typesetter::new(style, doc.hyphenator.clone());
    let Some(layout) = layout_note(&doc.book, &id, &style, &mut typesetter) else { return -1.0 };
    doc.page = layout.render_body(0, &style);
    unsafe { *out = doc.publish() };
    style.margin_top + layout.height(0) + style.margin_bottom
}

/// The short target of link `key` (`Book::preview`) typeset like a note into `*out`; returns its height, or a negative
/// number when the link has no short target.
///
/// # Safety
/// `doc` from [`forzac_open`]; `key` a valid NUL-terminated string; `settings` valid; `out` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_link_preview(
    doc: *mut Document,
    key: *const c_char,
    width: f32,
    settings: *const ForzacSettings,
    out: *mut ForzacPage,
) -> f32 {
    let doc = unsafe { &mut *doc };
    let key = unsafe { str(key) };
    let style = Settings::from(unsafe { *settings }).note_style(width);
    let mut typesetter = Typesetter::new(style, doc.hyphenator.clone());
    let Some(layout) = layout_preview(&doc.book, &key, &style, &mut typesetter) else { return -1.0 };
    doc.page = layout.render_body(0, &style);
    unsafe { *out = doc.publish() };
    style.margin_top + layout.height(0) + style.margin_bottom
}

/// The end page after the book's last page, for the current style.
///
/// # Safety
/// `doc` must come from [`forzac_open`] after [`forzac_paginate`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_end_page(doc: *mut Document) -> ForzacPage {
    let doc = unsafe { &mut *doc };
    if let (Some(style), Some(t)) = (doc.style, doc.typesetter.as_mut()) {
        doc.page = end_page(&doc.book, &style, t);
    }
    doc.publish()
}

/// Image `id`'s file bytes (owned by the document, the same pointer pages give); null when the book has no such image.
///
/// # Safety
/// `doc` from [`forzac_open`]; `id` a valid NUL-terminated string; `len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_image(doc: *const Document, id: *const c_char, len: *mut usize) -> *const u8 {
    let book = &unsafe { &*doc }.book;
    let Some(image) = book.images.get(unsafe { str(id) }.as_ref()) else { return std::ptr::null() };
    unsafe { *len = image.bytes.len() };
    image.bytes.as_ptr()
}

/// A table typeset for the view, owning what its page points to.
pub struct Table {
    view: TableView,
    /// The arrays `page` points to, kept alive with it.
    _arrays: Published,
    page: ForzacPage,
}

/// The table whose first row is `at` typeset for the view at `zoom` times the text size of `settings`; null when `at`
/// is not a table's first row. Free it with [`forzac_table_free`].
///
/// # Safety
/// `doc` from [`forzac_open`]; `settings` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_table(doc: *const Document, at: ForzacLocator, zoom: f32, settings: *const ForzacSettings) -> *mut Table {
    let doc = unsafe { &*doc };
    let style = Settings::from(unsafe { *settings }).table_style(zoom);
    let mut typesetter = Typesetter::new(style, doc.hyphenator.clone());
    let Some(view) = layout_table(&doc.book, at.into(), &mut typesetter) else { return std::ptr::null_mut() };
    // The page points into the heap buffers of `view` and `published`, which moving them into the box keeps.
    let mut published = Published::default();
    let page = published.publish(&view.content, &doc.book);
    Box::into_raw(Box::new(Table { view, _arrays: published, page }))
}

/// What to draw of a table for the view, from the origin, and its size; valid until the table is freed.
///
/// # Safety
/// `table` from [`forzac_table`]; `width` and `height` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_table_page(table: *const Table, width: *mut f32, height: *mut f32) -> ForzacPage {
    let table = unsafe { &*table };
    unsafe {
        *width = table.view.width;
        *height = table.view.height;
    }
    table.page
}

/// # Safety
/// `table` from [`forzac_table`], or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_table_free(table: *mut Table) {
    if !table.is_null() {
        drop(unsafe { Box::from_raw(table) });
    }
}

/// Note `id`'s title (its number, as a rule); null when it has none. Valid until the next string from `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`]; `id` a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_note_title(doc: *mut Document, id: *const c_char) -> *const c_char {
    let doc = unsafe { &mut *doc };
    let title = doc.book.notes.get(unsafe { str(id) }.as_ref()).and_then(|n| n.title.clone()).filter(|t| !t.trim().is_empty());
    doc.text(title.as_deref())
}

/// Where link `key` leads: 1 a place in the book (`*at`), 2 a note (`*text` its id), 3 an address outside the book
/// (`*text`), 4 a short place to show in the note sheet (`forzac_link_preview`; `*at` its start); 0 for an unknown key. `*text` is valid until the next string from `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`]; `key` a valid NUL-terminated string; `at` and `text` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_link(doc: *mut Document, key: *const c_char, at: *mut ForzacLocator, text: *mut *const c_char) -> u8 {
    let doc = unsafe { &mut *doc };
    let key = unsafe { str(key) };
    let (kind, place, s) = match doc.book.links.get(key.as_ref()) {
        Some(LinkTarget::Place { start, .. }) => (if doc.book.preview(&key).is_some() { 4 } else { 1 }, Some(*start), None),
        Some(LinkTarget::Note(id)) => (2, None, Some(id.clone())),
        Some(LinkTarget::External(url)) => (3, None, Some(url.clone())),
        None => return 0,
    };
    unsafe {
        if let Some(place) = place {
            *at = place.into();
        }
        *text = doc.text(s.as_deref());
    }
    kind
}

/// Rectangles over the text `[from, to)` (a search match) on `page` of the current chapter: up to `capacity` into
/// `out`, one per line; returns how many.
///
/// # Safety
/// `doc` from [`forzac_open`] after [`forzac_paginate`]; `out` writable for `capacity` rectangles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_marks(
    doc: *mut Document,
    page: u32,
    from: ForzacLocator,
    to: ForzacLocator,
    out: *mut ForzacRect,
    capacity: u32,
) -> u32 {
    let doc = unsafe { &mut *doc };
    let (Some(style), current) = (doc.style, doc.current) else { return 0 };
    let Some(layout) = doc.layout(current) else { return 0 };
    let marks = layout.marks((page as usize).min(layout.pages.len() - 1), &style, from.into(), to.into());
    unsafe { copy_rects(&marks, out, capacity) }
}

/// The same between heights `y0` and `y1` of `chapter`'s column, in the coordinates [`forzac_flow`] draws in.
///
/// # Safety
/// `doc` from [`forzac_open`] after [`forzac_paginate`]; `out` writable for `capacity` rectangles.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn forzac_flow_marks(
    doc: *mut Document,
    chapter: u32,
    y0: f32,
    y1: f32,
    from: ForzacLocator,
    to: ForzacLocator,
    out: *mut ForzacRect,
    capacity: u32,
) -> u32 {
    let doc = unsafe { &mut *doc };
    let Some(style) = doc.style else { return 0 };
    let Some(layout) = doc.layout(chapter as usize) else { return 0 };
    let marks = layout.flow_marks(y0, y1, &style, from.into(), to.into());
    unsafe { copy_rects(&marks, out, capacity) }
}

/// # Safety
/// `out` writable for `capacity` rectangles.
unsafe fn copy_rects(rects: &[Rect], out: *mut ForzacRect, capacity: u32) -> u32 {
    let n = rects.len().min(capacity as usize);
    for (i, r) in rects.iter().take(n).enumerate() {
        unsafe { *out.add(i) = (*r).into() };
    }
    n as u32
}

/// A bookmark as the list shows it: its place, the section it is in (null when there is none), the text from it and
/// the share of the book before it.
#[repr(C)]
pub struct ForzacBookmark {
    pub at: ForzacLocator,
    pub section: *const c_char,
    pub snippet: *const c_char,
    pub position: f32,
}

/// Sets a bookmark at `from` when none lies in `[from, to)` (a page, a screen), else removes those that do; true when
/// it set one. Save with [`forzac_state_save_bookmarks`].
///
/// # Safety
/// `doc` must come from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_bookmark_toggle(doc: *mut Document, from: ForzacLocator, to: ForzacLocator) -> bool {
    crate::bookmarks::toggle(&mut unsafe { &mut *doc }.bookmarks, from.into(), to.into())
}

/// Removes the bookmark at `at`.
///
/// # Safety
/// `doc` must come from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_bookmark_remove(doc: *mut Document, at: ForzacLocator) {
    let at = Locator::from(at);
    unsafe { &mut *doc }.bookmarks.retain(|m| *m != at);
}

/// Whether a bookmark lies in `[from, to)`.
///
/// # Safety
/// `doc` must come from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_bookmarked(doc: *const Document, from: ForzacLocator, to: ForzacLocator) -> bool {
    crate::bookmarks::marked(&unsafe { &*doc }.bookmarks, from.into(), to.into())
}

/// The bookmarks' places in reading order (`*count` of them), valid until the next call of this function on `doc`.
///
/// # Safety
/// `doc` must come from [`forzac_open`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_bookmark_places(doc: *mut Document, count: *mut u32) -> *const ForzacLocator {
    let doc = unsafe { &mut *doc };
    doc.bookmark_places = doc.bookmarks.iter().map(|&l| l.into()).collect();
    unsafe { *count = doc.bookmark_places.len() as u32 };
    doc.bookmark_places.as_ptr()
}

/// The bookmarks as the list shows them, in reading order; valid until the next call of this function on `doc`.
///
/// # Safety
/// `doc` must come from [`forzac_open`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_bookmarks(doc: *mut Document, count: *mut u32) -> *const ForzacBookmark {
    let doc = unsafe { &mut *doc };
    let entries = crate::bookmarks::entries(&doc.book, &doc.bookmarks);
    doc.bookmark_strings = entries.iter().flat_map(|e| [cstring(e.section.as_deref().unwrap_or("")), cstring(&e.snippet)]).collect();
    doc.bookmark_list = entries
        .iter()
        .zip(doc.bookmark_strings.chunks(2))
        .map(|(e, s)| ForzacBookmark {
            at: e.at.into(),
            section: if e.section.is_some() { s[0].as_ptr() } else { std::ptr::null() },
            snippet: s[1].as_ptr(),
            position: e.position,
        })
        .collect();
    unsafe { *count = doc.bookmark_list.len() as u32 };
    doc.bookmark_list.as_ptr()
}

/// A word drawn on a line (`ChapterLayout::words`): the whole word's text `[start, end)` and the piece's rectangle.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacWord {
    pub start: ForzacLocator,
    pub end: ForzacLocator,
    pub rect: ForzacRect,
}

impl From<crate::Word> for ForzacWord {
    fn from(w: crate::Word) -> Self {
        ForzacWord { start: w.start.into(), end: w.end.into(), rect: w.rect.into() }
    }
}

impl From<ForzacWord> for crate::Word {
    fn from(w: ForzacWord) -> Self {
        let r = w.rect;
        crate::Word { start: w.start.into(), end: w.end.into(), rect: Rect { x: r.x, y: r.y, w: r.w, h: r.h } }
    }
}

/// A text range `[start, end)`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ForzacRange {
    pub start: ForzacLocator,
    pub end: ForzacLocator,
}

/// The words drawn on `page` of the current chapter, in reading order; valid until the next words from `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`] after [`forzac_paginate`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_page_words(doc: *mut Document, page: u32, count: *mut u32) -> *const ForzacWord {
    let doc = unsafe { &mut *doc };
    let (style, current, book) = (doc.style, doc.current, doc.book.clone());
    let words = match (style, doc.layout(current)) {
        (Some(style), Some(l)) => l.words((page as usize).min(l.pages.len() - 1), &style, &book),
        _ => Vec::new(),
    };
    doc.words = words.into_iter().map(Into::into).collect();
    unsafe { *count = doc.words.len() as u32 };
    doc.words.as_ptr()
}

/// The words between heights `y0` and `y1` of `chapter`'s column, in the coordinates [`forzac_flow`] draws in.
///
/// # Safety
/// `doc` from [`forzac_open`] after [`forzac_paginate`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_flow_words(doc: *mut Document, chapter: u32, y0: f32, y1: f32, count: *mut u32) -> *const ForzacWord {
    let doc = unsafe { &mut *doc };
    let (style, book) = (doc.style, doc.book.clone());
    let words = match (style, doc.layout(chapter as usize)) {
        (Some(style), Some(l)) => l.flow_words(y0, y1, &style, &book),
        _ => Vec::new(),
    };
    doc.words = words.into_iter().map(Into::into).collect();
    unsafe { *count = doc.words.len() as u32 };
    doc.words.as_ptr()
}

/// # Safety
/// `words` must point to `count` words.
unsafe fn words(words: *const ForzacWord, count: u32) -> Vec<crate::Word> {
    if words.is_null() {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(words, count as usize) }.iter().map(|&w| w.into()).collect()
}

fn word_index(i: Option<usize>) -> i32 {
    i.map_or(-1, |i| i as i32)
}

/// The word under `(x, y)` (else the nearest on the nearest line): its index, or −1 when there are no words.
///
/// # Safety
/// `list` must point to `count` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_word_at(list: *const ForzacWord, count: u32, x: f32, y: f32) -> i32 {
    word_index(crate::selection::word_at(&unsafe { words(list, count) }, x, y))
}

/// The next (`delta` 1) or previous (−1) word after word `i` (the other pieces of a hyphenated word skipped); −1 past
/// the ends.
///
/// # Safety
/// `list` must point to `count` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_word_step(list: *const ForzacWord, count: u32, i: u32, delta: i32) -> i32 {
    word_index(crate::selection::step(&unsafe { words(list, count) }, i as usize, delta))
}

/// The word on the next (`delta` 1) or previous (−1) line nearest word `i`'s middle; −1 past the ends.
///
/// # Safety
/// `list` must point to `count` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_word_line(list: *const ForzacWord, count: u32, i: u32, delta: i32) -> i32 {
    word_index(crate::selection::line_step(&unsafe { words(list, count) }, i as usize, delta))
}

/// The first piece of the word starting at `at`, else of the first word after it; −1 when there is none.
///
/// # Safety
/// `list` must point to `count` words.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_word_find(list: *const ForzacWord, count: u32, at: ForzacLocator) -> i32 {
    word_index(crate::selection::find(&unsafe { words(list, count) }, at.into()))
}

/// The block holding `at` (a paragraph, a heading, a table row) as a range.
///
/// # Safety
/// `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_block_range(doc: *const Document, at: ForzacLocator) -> ForzacRange {
    let (start, end) = unsafe { &*doc }.book.block_range(at.into());
    ForzacRange { start: start.into(), end: end.into() }
}

/// The text of `[from, to)` as it is copied: blocks on lines of their own, without soft hyphens and note markers. Valid
/// until the next string from `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_text(doc: *mut Document, from: ForzacLocator, to: ForzacLocator) -> *const c_char {
    let doc = unsafe { &mut *doc };
    let text = doc.book.text(from.into(), to.into(), usize::MAX);
    doc.text(Some(&text))
}

/// The text of `[from, to)` signed to share: «text», a blank line, "— Author, «Title»". Valid until the next string.
///
/// # Safety
/// `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_share_text(doc: *mut Document, from: ForzacLocator, to: ForzacLocator) -> *const c_char {
    let doc = unsafe { &mut *doc };
    let text = crate::quotes::share_text(&doc.book, &doc.book.text(from.into(), to.into(), usize::MAX));
    doc.text(Some(&text))
}

/// Keeps `[from, to)` as a highlight. Save with [`forzac_state_save_quotes`].
///
/// # Safety
/// `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_quote_add(doc: *mut Document, from: ForzacLocator, to: ForzacLocator) {
    let quotes = &mut unsafe { &mut *doc }.quotes;
    let q = (Locator::from(from), Locator::from(to));
    if q.0 < q.1 && !quotes.contains(&q) {
        let i = quotes.partition_point(|x| *x < q);
        quotes.insert(i, q);
    }
}

/// Deletes the highlight `[from, to)`.
///
/// # Safety
/// `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_quote_remove(doc: *mut Document, from: ForzacLocator, to: ForzacLocator) {
    let q = (Locator::from(from), Locator::from(to));
    unsafe { &mut *doc }.quotes.retain(|x| *x != q);
}

/// The highlights' ranges in reading order, valid until the next call of this function on `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_quote_places(doc: *mut Document, count: *mut u32) -> *const ForzacRange {
    let doc = unsafe { &mut *doc };
    doc.quote_places = doc.quotes.iter().map(|&(a, b)| ForzacRange { start: a.into(), end: b.into() }).collect();
    unsafe { *count = doc.quote_places.len() as u32 };
    doc.quote_places.as_ptr()
}

/// A highlight as the list shows it: its range, its text, the section it starts in (null when none) and the share of
/// the book before it.
#[repr(C)]
pub struct ForzacQuote {
    pub start: ForzacLocator,
    pub end: ForzacLocator,
    pub text: *const c_char,
    pub section: *const c_char,
    pub position: f32,
}

/// The highlights as the list shows them, in reading order; valid until the next call of this function on `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_quotes(doc: *mut Document, count: *mut u32) -> *const ForzacQuote {
    let doc = unsafe { &mut *doc };
    let entries = crate::quotes::entries(&doc.book, &doc.quotes);
    doc.quote_strings = entries.iter().flat_map(|e| [cstring(&e.text), cstring(e.section.as_deref().unwrap_or(""))]).collect();
    doc.quote_list = entries
        .iter()
        .zip(doc.quote_strings.chunks(2))
        .map(|(e, s)| ForzacQuote {
            start: e.start.into(),
            end: e.end.into(),
            text: s[0].as_ptr(),
            section: if e.section.is_some() { s[1].as_ptr() } else { std::ptr::null() },
            position: e.position,
        })
        .collect();
    unsafe { *count = doc.quote_list.len() as u32 };
    doc.quote_list.as_ptr()
}

/// All highlights as one text (Android's export); valid until the next string from `doc`.
///
/// # Safety
/// `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_quotes_export(doc: *mut Document) -> *const c_char {
    let doc = unsafe { &mut *doc };
    let text = crate::quotes::export(&doc.book, &crate::quotes::entries(&doc.book, &doc.quotes));
    doc.text(Some(&text))
}

/// The document's book, for searching on another thread; release it with [`forzac_book_release`].
pub struct BookRef(Arc<Book>);

/// # Safety
/// `doc` must come from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_book(doc: *const Document) -> *mut BookRef {
    Box::into_raw(Box::new(BookRef(unsafe { &*doc }.book.clone())))
}

/// # Safety
/// `book` must come from [`forzac_book`]; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_book_release(book: *mut BookRef) {
    if !book.is_null() {
        drop(unsafe { Box::from_raw(book) });
    }
}

/// A search match: the text `[start, end)`, a snippet with the match at bytes `[match_start, match_end)`, the section
/// (null when there is none) and the share of the book before it.
#[repr(C)]
pub struct ForzacHit {
    pub start: ForzacLocator,
    pub end: ForzacLocator,
    pub snippet: *const c_char,
    pub match_start: u32,
    pub match_end: u32,
    pub section: *const c_char,
    pub position: f32,
}

/// A search's matches, owned by the caller until [`forzac_search_free`].
pub struct Search {
    hits: Vec<ForzacHit>,
    _strings: Vec<CString>,
}

/// Searches `book` for `query` (the rules of [`crate::search`]); any thread. The matches are read with
/// [`forzac_search_hits`] and freed with [`forzac_search_free`].
///
/// # Safety
/// `book` from [`forzac_book`]; `query` a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_search(book: *const BookRef, query: *const c_char) -> *mut Search {
    let hits = crate::search::search(&unsafe { &*book }.0, &unsafe { str(query) });
    let mut strings = Vec::with_capacity(hits.len() * 2);
    let mut out = Vec::with_capacity(hits.len());
    for h in &hits {
        let snippet = cstring(&h.snippet);
        let section = h.section.as_deref().map(cstring);
        out.push(ForzacHit {
            start: h.start.into(),
            end: h.end.into(),
            snippet: snippet.as_ptr(),
            match_start: h.matched.start as u32,
            match_end: h.matched.end as u32,
            section: section.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            position: h.position,
        });
        strings.push(snippet);
        strings.extend(section);
    }
    Box::into_raw(Box::new(Search { hits: out, _strings: strings }))
}

/// The matches of `search`, valid until it is freed.
///
/// # Safety
/// `search` from [`forzac_search`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_search_hits(search: *const Search, count: *mut u32) -> *const ForzacHit {
    let s = unsafe { &*search };
    unsafe { *count = s.hits.len() as u32 };
    s.hits.as_ptr()
}

/// # Safety
/// `search` from [`forzac_search`]; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_search_free(search: *mut Search) {
    if !search.is_null() {
        drop(unsafe { Box::from_raw(search) });
    }
}

/// Where the bundled assets are (`fonts/<family>/…`): the families other than PT Serif are read from there.
///
/// # Safety
/// `dir` must be a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_set_assets(dir: *const c_char) {
    fonts::set_assets(PathBuf::from(unsafe { str(dir) }.into_owned()));
}

/// The bundled font file of `font`, so the shell rasterises exactly what was shaped; null for a font read from a file.
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

/// The file of `font` (another family's face or a fallback font): copies its path (NUL-terminated) into `path` and
/// sets `*index` to the face index in a collection. False for a bundled face, an unknown id or a path longer than
/// `capacity`.
///
/// # Safety
/// `path` must be writable for `capacity` bytes and `index` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_font_file(font: u16, path: *mut c_char, capacity: usize, index: *mut u32) -> bool {
    let Some(FontSource::File { path: file, index: i, .. }) = fonts::font_source(FontId(font)) else { return false };
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

/// The variation-axis values `font` was shaped at (a variable family's face): up to `capacity` tags (the four letters
/// big-endian, as Core Text's axis identifiers) and values; returns how many. Zero for a font without variations.
///
/// # Safety
/// `tags` and `values` must be writable for `capacity` items.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_font_variations(font: u16, tags: *mut u32, values: *mut f32, capacity: u32) -> u32 {
    let Some(FontSource::File { variations, .. }) = fonts::font_source(FontId(font)) else { return 0 };
    let n = variations.len().min(capacity as usize);
    for (i, (tag, value)) in variations.iter().take(n).enumerate() {
        let tag = tag.bytes().chain(std::iter::repeat(b' ')).take(4).fold(0u32, |acc, b| acc << 8 | b as u32);
        unsafe {
            *tags.add(i) = tag;
            *values.add(i) = *value;
        }
    }
    n as u32
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

/// The default settings.
#[unsafe(no_mangle)]
pub extern "C" fn forzac_settings_default() -> ForzacSettings {
    Settings::default().into()
}

/// The theme shown for `settings`: the chosen one, else Light or Dark as the system prefers (an index into the
/// themes: Light, Sepia, Dark, Dawn, Wave, Forest).
///
/// # Safety
/// `settings` must point to valid settings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_settings_theme(settings: *const ForzacSettings, system_dark: bool) -> u32 {
    Settings::from(unsafe { *settings }).theme_index(system_dark) as u32
}

/// The text size of `settings` in logical pixels.
///
/// # Safety
/// `settings` must point to valid settings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_settings_text_size(settings: *const ForzacSettings) -> f32 {
    Settings::from(unsafe { *settings }).text_size()
}

/// How many text sizes there are (`ForzacSettings::size` is below it).
#[unsafe(no_mangle)]
pub extern "C" fn forzac_settings_size_count() -> u32 {
    TEXT_SIZES.len() as u32
}

/// Switches `setting` (0 theme, 1 font, 2 size, 3 spacing, 4 alignment, 5 margins, 6 pages) to its next (`delta` 1)
/// or previous (−1) value: sizes stop at the ends, the others go round; a theme starts from the one shown.
///
/// # Safety
/// `settings` must point to valid, writable settings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_settings_step(settings: *mut ForzacSettings, setting: u32, delta: i32, system_dark: bool) {
    const SETTINGS: [Setting; 7] =
        [Setting::Theme, Setting::Font, Setting::Size, Setting::Spacing, Setting::Align, Setting::Margins, Setting::Turning];
    let Some(&setting) = SETTINGS.get(setting as usize) else { return };
    let mut s = Settings::from(unsafe { *settings });
    s.step(setting, delta, system_dark);
    unsafe { *settings = s.into() };
}

/// The reading state (positions, recent books, settings) in the file at a path the shell chooses.
pub struct State {
    store: crate::state::Store,
    recent: Vec<ForzacRecent>,
    recent_strings: Vec<CString>,
}

/// A recently read book.
#[repr(C)]
pub struct ForzacRecent {
    pub path: *const c_char,
    pub title: *const c_char,
    pub authors: *const c_char,
    pub progress: f32,
}

/// Opens the state file at `path` (it need not exist yet).
///
/// # Safety
/// `path` must be a valid NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_open(path: *const c_char) -> *mut State {
    let path = unsafe { str(path) }.into_owned();
    Box::into_raw(Box::new(State { store: crate::state::Store::open(path), recent: Vec::new(), recent_strings: Vec::new() }))
}

/// Where the document's book was left; false when it is not remembered.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`]; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_position(state: *const State, doc: *const Document, out: *mut ForzacLocator) -> bool {
    let (state, doc) = unsafe { (&*state, &*doc) };
    let Some(l) = state.store.position(&doc.book, &doc.path) else { return false };
    unsafe { *out = l.into() };
    true
}

/// Remembers that the document's book is read at `at`; false when the file cannot be written.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_save_position(state: *mut State, doc: *const Document, at: ForzacLocator, finished: bool) -> bool {
    let (state, doc) = unsafe { (&mut *state, &*doc) };
    state.store.save_position(&doc.book, &doc.path, at.into(), finished).is_ok()
}

/// Up to `n` books whose files still exist, the last read first; valid until the next call on `state`.
///
/// # Safety
/// `state` from [`forzac_state_open`]; `count` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_recent(state: *mut State, n: u32, count: *mut u32) -> *const ForzacRecent {
    let state = unsafe { &mut *state };
    let books = state.store.recent(n as usize);
    state.recent_strings =
        books.iter().flat_map(|e| [cstring(&e.path.to_string_lossy()), cstring(&e.title), cstring(&e.authors)]).collect();
    state.recent = books
        .iter()
        .zip(state.recent_strings.chunks(3))
        .map(|(e, s)| ForzacRecent { path: s[0].as_ptr(), title: s[1].as_ptr(), authors: s[2].as_ptr(), progress: e.progress })
        .collect();
    unsafe { *count = state.recent.len() as u32 };
    state.recent.as_ptr()
}

/// Takes the document's bookmarks from `state`.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_load_bookmarks(state: *const State, doc: *mut Document) {
    let (state, doc) = unsafe { (&*state, &mut *doc) };
    doc.bookmarks = state.store.bookmarks(&doc.book);
}

/// Remembers the document's bookmarks in `state`; false when the file cannot be written.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_save_bookmarks(state: *mut State, doc: *const Document) -> bool {
    let (state, doc) = unsafe { (&mut *state, &*doc) };
    state.store.set_bookmarks(&doc.book, &doc.bookmarks).is_ok()
}

/// Takes the document's highlights from `state`.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_load_quotes(state: *const State, doc: *mut Document) {
    let (state, doc) = unsafe { (&*state, &mut *doc) };
    doc.quotes = state.store.quotes(&doc.book);
}

/// Remembers the document's highlights in `state`; false when the file cannot be written.
///
/// # Safety
/// `state` from [`forzac_state_open`], `doc` from [`forzac_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_save_quotes(state: *mut State, doc: *const Document) -> bool {
    let (state, doc) = unsafe { (&mut *state, &*doc) };
    state.store.set_quotes(&doc.book, &doc.quotes).is_ok()
}

/// The settings remembered in `state` (defaults for anything missing or unknown); the defaults for a null `state`.
///
/// # Safety
/// `state` null or from [`forzac_state_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_settings(state: *const State) -> ForzacSettings {
    match unsafe { state.as_ref() } {
        Some(state) => Settings::load(&state.store).into(),
        None => Settings::default().into(),
    }
}

/// Remembers `settings` in `state`; false when the file cannot be written.
///
/// # Safety
/// `state` from [`forzac_state_open`]; `settings` valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_save_settings(state: *mut State, settings: *const ForzacSettings) -> bool {
    let state = unsafe { &mut *state };
    Settings::from(unsafe { *settings }).save(&mut state.store).is_ok()
}

/// # Safety
/// `state` must come from [`forzac_state_open`]; it is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn forzac_state_close(state: *mut State) {
    if !state.is_null() {
        drop(unsafe { Box::from_raw(state) });
    }
}

#[cfg(test)]
mod tests;
