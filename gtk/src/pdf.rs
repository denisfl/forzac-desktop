//! A window for books made of pages, not typeset here (`pdf-reading`, `fixed-layout`): a PDF drawn by PDFium, or a book
//! of picture pages. Pages are drawn off the main thread, one at a time with the neighbours ahead; the keys, clicks and
//! "Slide" are a book's. Over a PDF the menu and panels use the Light theme, whatever the reader's theme is. The core
//! keeps the place: `Locator(0, page, 0)`.
//!
//! PDFium makes no promise about threads, so one thread owns it and everything else asks that thread ([`call`]).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Duration, Instant};

use forzac_core::i18n::{fill, tr};
use forzac_core::{Book, Error, Locator, log};
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, graphene, gsk, prelude::*};
use pdfium_render::prelude::*;

use crate::icons::{self, Icon};
use crate::menu::Slider;
use crate::page_view::{THEMES, latin, rgba};
use crate::{Options, SharedStore, fitted, now_ms, utc_offset};

const FOOT: f32 = 26.0;
const MAX_ZOOM: f32 = 4.0;
/// A turn's slide, as a book's.
const SLIDE: Duration = Duration::from_millis(200);
/// Moves are saved this long after the last, as a book's.
const SAVE_DELAY: Duration = Duration::from_secs(1);
/// The longest a click on the middle waits to see whether it is half a double click.
const DOUBLE_CLICK: Duration = Duration::from_millis(300);
/// The Light theme's link colour as a page's pixels: blue, green, red, alpha.
const LINK_LINE: [u8; 4] = [0x1F, 0x62, 0x94, 0xFF];

// region the engine

type Job = Box<dyn FnOnce(&mut Engine) + Send>;

struct Engine {
    pdfium: Option<&'static Pdfium>,
    docs: HashMap<u32, PdfDocument<'static>>,
    next: u32,
}

/// Where `libpdfium.so` is: beside the binary, in the archive's or the package's `lib/forzac`, or named for development.
fn library_paths() -> Vec<PathBuf> {
    let exe = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_owned));
    let mut paths: Vec<PathBuf> = std::env::var_os("FORZAC_PDFIUM").map(PathBuf::from).into_iter().collect();
    if let Some(exe) = exe {
        paths.push(exe.join("libpdfium.so"));
        paths.push(exe.join("../lib/forzac/libpdfium.so"));
    }
    paths.push(PathBuf::from("/usr/lib/forzac/libpdfium.so"));
    paths
}

fn engine_thread() -> &'static mpsc::Sender<Job> {
    static JOBS: OnceLock<mpsc::Sender<Job>> = OnceLock::new();
    JOBS.get_or_init(|| {
        let (send, jobs) = mpsc::channel::<Job>();
        std::thread::spawn(move || {
            let bound = library_paths().into_iter().find_map(|p| Pdfium::bind_to_library(p).ok());
            if bound.is_none() {
                eprintln!("libpdfium.so not found: PDFs cannot be opened");
            }
            let pdfium = bound.map(|b| &*Box::leak(Box::new(Pdfium::new(b))));
            let mut engine = Engine { pdfium, docs: HashMap::new(), next: 0 };
            for job in jobs {
                job(&mut engine);
            }
        });
        send
    })
}

/// Runs `f` on the engine's thread and waits for it; never on the main thread ([`ask`]).
fn call<R: Send + 'static>(f: impl FnOnce(&mut Engine) -> R + Send + 'static) -> Option<R> {
    let (send, reply) = mpsc::channel();
    engine_thread()
        .send(Box::new(move |engine| {
            let _ = send.send(f(engine));
        }))
        .ok()?;
    reply.recv().ok()
}

/// [`call`] for the main thread.
async fn ask<R: Send + 'static>(f: impl FnOnce(&mut Engine) -> R + Send + 'static) -> Option<R> {
    gio::spawn_blocking(move || call(f)).await.ok().flatten()
}

fn open_error(e: &PdfiumError) -> Error {
    match e {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => Error::Password,
        _ => Error::Damaged,
    }
}

/// What a PDF holds, read when it opens.
struct Info {
    doc: u32,
    /// Page sizes in points, rotation applied.
    sizes: Vec<(f32, f32)>,
    title: String,
    author: String,
    /// The outline: title, depth, page.
    outline: Vec<(String, usize, usize)>,
}

/// The page a destination or an action inside the file goes to.
fn target_page(destination: Option<PdfDestination>, action: Option<PdfAction>) -> Option<usize> {
    if let Some(d) = destination {
        return d.page_index().ok().map(|p| p as usize);
    }
    let action = action?;
    let local = action.as_local_destination_action()?;
    local.destination().ok()?.page_index().ok().map(|p| p as usize)
}

fn walk(mark: PdfBookmark, depth: usize, doc_pages: usize, out: &mut Vec<(String, usize, usize)>) {
    let mut at = Some(mark);
    while let Some(m) = at {
        let target = target_page(m.destination(), m.action()).filter(|&p| p < doc_pages);
        if let (Some(title), Some(page)) = (m.title(), target) {
            out.push((title, depth, page));
        }
        if let Some(child) = m.first_child() {
            walk(child, depth + 1, doc_pages, out);
        }
        at = m.next_sibling();
    }
}

fn open_doc(engine: &mut Engine, path: &Path) -> Result<Info, Error> {
    let pdfium = engine.pdfium.ok_or(Error::Unavailable)?;
    let doc = pdfium.load_pdf_from_file(path, None).map_err(|e| open_error(&e))?;
    let sizes: Vec<(f32, f32)> =
        doc.pages().page_sizes().map_err(|e| open_error(&e))?.iter().map(|r| (r.width().value, r.height().value)).collect();
    if sizes.is_empty() {
        return Err(Error::Damaged);
    }
    let (title, author) = (tag(&doc, PdfDocumentMetadataTagType::Title), tag(&doc, PdfDocumentMetadataTagType::Author));
    let mut outline = Vec::new();
    if let Some(root) = doc.bookmarks().root() {
        walk(root, 0, sizes.len(), &mut outline);
    }
    engine.next += 1;
    let id = engine.next;
    engine.docs.insert(id, doc);
    Ok(Info { doc: id, sizes, title, author, outline })
}

fn tag(doc: &PdfDocument, name: PdfDocumentMetadataTagType) -> String {
    doc.metadata().get(name).map(|t| t.value().to_owned()).unwrap_or_default()
}

/// Why a PDF cannot be opened, if it cannot; off the main thread.
pub fn problem(path: &Path) -> Option<Error> {
    let path = path.to_owned();
    call(move |engine| {
        let Some(pdfium) = engine.pdfium else { return Some(Error::Unavailable) };
        match pdfium.load_pdf_from_file(&path, None) {
            Ok(doc) if doc.pages().is_empty() => Some(Error::Damaged),
            Ok(_) => None,
            Err(e) => Some(open_error(&e)),
        }
    })
    .unwrap_or(Some(Error::Unavailable))
}

/// A PDF's first page as a PNG, its title and author, for the library; off the main thread.
pub fn cover(path: &Path) -> Option<(Vec<u8>, String, String)> {
    let path = path.to_owned();
    let (raw, w, h, title, author) = call(move |engine| {
        let doc = engine.pdfium?.load_pdf_from_file(&path, None).ok()?;
        let page = doc.pages().get(0).ok()?;
        let config = PdfRenderConfig::new().set_target_height(800).set_format(PdfBitmapFormat::BGRA).set_reverse_byte_order(true);
        let bitmap = page.render_with_config(&config).ok()?;
        let (title, author) = (tag(&doc, PdfDocumentMetadataTagType::Title), tag(&doc, PdfDocumentMetadataTagType::Author));
        Some((bitmap.as_raw_bytes(), bitmap.width(), bitmap.height(), title, author))
    })??;
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_mut_slice(raw, gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, w, h, w * 4);
    Some((pixbuf.save_to_bufferv("png", &[]).ok()?, title, author))
}

/// Where a link goes.
enum Link {
    Page(usize),
    External(String),
}

/// The pixels of part of a page: the page `full` pixels large, its part from `origin`, `size` pixels large; `line` the
/// thickness of the line under links.
struct Part {
    page: usize,
    full: (i32, i32),
    origin: (i32, i32),
    size: (i32, i32),
    line: i32,
}

/// How a page's coordinates, y up from its box's corner and before its rotation, sit on the page as it shows.
struct Shown {
    box_: PdfRect,
    rotation: PdfPageRenderRotation,
}

impl Shown {
    fn of(page: &PdfPage) -> Option<Self> {
        Some(Shown { box_: page.boundaries().bounding().ok()?.bounds, rotation: page.rotation().ok()? })
    }

    /// A point of the page as fractions of the page shown, y down.
    fn fractions(&self, x: f32, y: f32) -> (f32, f32) {
        let b = &self.box_;
        let (u, v) = ((x - b.left().value) / b.width().value.max(1.0), (y - b.bottom().value) / b.height().value.max(1.0));
        match self.rotation {
            PdfPageRenderRotation::None => (u, 1.0 - v),
            PdfPageRenderRotation::Degrees90 => (v, u),
            PdfPageRenderRotation::Degrees180 => (1.0 - u, v),
            PdfPageRenderRotation::Degrees270 => (1.0 - v, 1.0 - u),
        }
    }

    /// The point of the page at fractions of the page shown.
    fn point(&self, fx: f32, fy: f32) -> (f32, f32) {
        let (u, v) = match self.rotation {
            PdfPageRenderRotation::None => (fx, 1.0 - fy),
            PdfPageRenderRotation::Degrees90 => (fy, fx),
            PdfPageRenderRotation::Degrees180 => (1.0 - fx, fy),
            PdfPageRenderRotation::Degrees270 => (1.0 - fy, 1.0 - fx),
        };
        let b = &self.box_;
        (b.left().value + u * b.width().value, b.bottom().value + v * b.height().value)
    }

    /// A rectangle of the page as fractions of the page shown: left, top, right, bottom.
    fn rect(&self, r: &PdfRect) -> [f32; 4] {
        let (x0, y0) = self.fractions(r.left().value, r.bottom().value);
        let (x1, y1) = self.fractions(r.right().value, r.top().value);
        [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)]
    }
}

/// Draws a line under the page's links into its pixels: PDFs often leave links unmarked.
fn underline_links(page: &PdfPage, part: &Part, bytes: &mut [u8], stride: i32) {
    // A page of links, such as the contents, is a list already.
    if page.links().iter().count() >= 8 {
        return;
    }
    let Some(shown) = Shown::of(page) else { return };
    let px = |f: f32, full: i32, origin: i32, size: i32| ((f * full as f32).round() as i32 - origin).clamp(0, size);
    for link in page.links().iter() {
        let Ok(rect) = link.rect() else { continue };
        // The link's bottom edge, which a rotated page shows along another side.
        let (x0, y0) = shown.fractions(rect.left().value, rect.bottom().value);
        let (x1, y1) = shown.fractions(rect.right().value, rect.bottom().value);
        let (left, right) =
            (px(x0.min(x1), part.full.0, part.origin.0, part.size.0), px(x0.max(x1), part.full.0, part.origin.0, part.size.0));
        let (top, bottom) =
            (px(y0.min(y1), part.full.1, part.origin.1, part.size.1), px(y0.max(y1), part.full.1, part.origin.1, part.size.1));
        let right = right.max(left + part.line).min(part.size.0);
        for y in top..bottom.max(top + part.line).min(part.size.1) {
            let row = (y * stride) as usize;
            for pixel in bytes[row + left as usize * 4..row + right as usize * 4].as_chunks_mut::<4>().0 {
                *pixel = LINK_LINE;
            }
        }
    }
}

/// The lines of match `ordinal` of `query` on `page`, as fractions of the page shown. The search reads the page's
/// text whole; here its characters give the match's place.
fn match_rects(engine: &Engine, doc: u32, page: usize, query: &str, ordinal: usize) -> Option<Vec<[f32; 4]>> {
    let pdf_page = engine.docs.get(&doc)?.pages().get(page as _).ok()?;
    let text = pdf_page.text().ok()?;
    let (mut whole, mut indices) = (String::new(), Vec::new());
    for c in text.chars().iter() {
        if let Some(u) = c.unicode_char() {
            whole.push(u);
            indices.extend(std::iter::repeat_n(c.index(), u.len_utf16()));
        }
    }
    let place = forzac_core::search::search_page(&whole, query).into_iter().nth(ordinal)?.place;
    let (first, last) = (*indices.get(place.start)?, *indices.get(place.end - 1)?);
    let shown = Shown::of(&pdf_page)?;
    Some(text.segments_subset(first, last - first + 1).iter().map(|line| shown.rect(&line.bounds())).collect())
}

fn render(engine: &Engine, doc: u32, part: &Part) -> Option<(Vec<u8>, i32)> {
    let page = engine.docs.get(&doc)?.pages().get(part.page as _).ok()?;
    let mut bitmap = PdfBitmap::empty(part.size.0 as _, part.size.1 as _, PdfBitmapFormat::BGRA).ok()?;
    let config = PdfRenderConfig::new()
        .set_fixed_size(part.full.0 as _, part.full.1 as _)
        .set_origin(-part.origin.0 as _, -part.origin.1 as _)
        .set_format(PdfBitmapFormat::BGRA)
        .set_reverse_byte_order(false);
    page.render_into_bitmap_with_config(&mut bitmap, &config).ok()?;
    let mut bytes = bitmap.as_raw_bytes();
    let stride = bytes.len() as i32 / part.size.1;
    underline_links(&page, part, &mut bytes, stride);
    Some((bytes, stride))
}

/// A page's pixels, made off the main thread.
struct Pixels {
    bytes: glib::Bytes,
    format: gdk::MemoryFormat,
    size: (i32, i32),
    stride: usize,
}

impl Pixels {
    fn texture(&self) -> gdk::Texture {
        gdk::MemoryTexture::new(self.size.0, self.size.1, self.format, &self.bytes, self.stride).upcast()
    }
}

/// Picture page `page` of `book` at `size` pixels; `None` for an empty page or a picture that does not decode.
fn decode(book: &Book, page: usize, size: (i32, i32)) -> Option<Pixels> {
    let id = book.pictures.as_ref()?.pages.get(page)?.as_ref()?;
    let bytes = &book.images.get(id)?.bytes;
    let drawn = forzac_core::images::is_svg(bytes).then(|| forzac_core::images::png(bytes, size.0 as u32, size.1 as u32)).flatten();
    let loader = gtk::gdk_pixbuf::PixbufLoader::new();
    loader.set_size(size.0, size.1);
    loader.write(drawn.as_deref().unwrap_or(bytes)).ok()?;
    loader.close().ok()?;
    let pixbuf = loader.pixbuf()?;
    let format = if pixbuf.has_alpha() { gdk::MemoryFormat::R8g8b8a8 } else { gdk::MemoryFormat::R8g8b8 };
    Some(Pixels { bytes: pixbuf.read_pixel_bytes(), format, size: (pixbuf.width(), pixbuf.height()), stride: pixbuf.rowstride() as usize })
}

/// Part of a page from where the book's pages come from: PDFium's thread, or a worker decoding its picture.
async fn pixels(doc: u32, pictures: Option<Arc<Book>>, near: Option<Arc<AtomicUsize>>, part: Part) -> Option<Pixels> {
    let page = part.page;
    if let Some(book) = pictures {
        return gio::spawn_blocking(move || {
            // Flipped past meanwhile.
            if near.is_some_and(|n| n.load(Ordering::Relaxed).abs_diff(page) > 4) {
                return None;
            }
            decode(&book, page, part.size)
        })
        .await
        .ok()
        .flatten();
    }
    let size = part.size;
    let (bytes, stride) = ask(move |engine| {
        if near.is_some_and(|n| n.load(Ordering::Relaxed).abs_diff(page) > 4) {
            return None;
        }
        render(engine, doc, &part)
    })
    .await
    .flatten()?;
    Some(Pixels { bytes: glib::Bytes::from_owned(bytes), format: gdk::MemoryFormat::B8g8r8a8Premultiplied, size, stride: stride as usize })
}

// endregion

// region the pages

/// The pages from `first` (two in a spread) fitted to `area` at one scale; the first page of a pair on the right in a
/// book turned right to left.
fn placed(sizes: &[(f32, f32)], first: usize, two: bool, rtl: bool, area: (f32, f32)) -> Vec<(usize, graphene::Rect)> {
    let indices = match (two && first + 1 < sizes.len(), rtl) {
        (false, _) => vec![first],
        (true, false) => vec![first, first + 1],
        (true, true) => vec![first + 1, first],
    };
    let width: f32 = indices.iter().map(|&i| sizes[i].0).sum();
    let height = indices.iter().map(|&i| sizes[i].1).fold(1.0, f32::max);
    let scale = (area.0 / width.max(1.0)).min(area.1 / height);
    let mut x = (area.0 - width * scale) / 2.0;
    indices
        .into_iter()
        .map(|i| {
            let s = sizes[i];
            let rect = graphene::Rect::new(x, (area.1 - s.1 * scale) / 2.0, s.0 * scale, s.1 * scale);
            x += s.0 * scale;
            (i, rect)
        })
        .collect()
}

/// A turn on its way: the pages leaving and the direction.
struct Slide {
    from: usize,
    dir: f32,
    start: Instant,
}

#[derive(Default)]
pub struct State {
    doc: u32,
    /// A book of picture pages; `None`: a PDF, on PDFium's thread as `doc`.
    pictures: Option<Arc<Book>>,
    /// The theme of the pill and, in a book of picture pages, of the paper and the foot.
    theme: usize,
    sizes: Vec<(f32, f32)>,
    page: usize,
    zoom: f32,
    pan: (f32, f32),
    /// Rendered pages for `cached_for`.
    textures: HashMap<usize, gdk::Texture>,
    waiting: HashSet<usize>,
    cached_for: (i32, i32, i32),
    /// The part in view drawn at the zoom, in the page area's units.
    sharp: Vec<(graphene::Rect, gdk::Texture)>,
    sharp_ticket: u64,
    slide: Option<Slide>,
    back: Vec<usize>,
    pill_turns: i32,
    swipe: f64,
    swiped: bool,
    jumped: bool,
    /// A match gone to from the search: its page and lines as fractions of it, until the next click.
    found: Option<(usize, Vec<[f32; 4]>)>,
}

impl State {
    fn rtl(&self) -> bool {
        self.pictures.as_ref().and_then(|b| b.pictures.as_ref()).is_some_and(|p| p.rtl)
    }

    /// The first page of the screen with `page`: in a spread the first page is alone, then pairs.
    fn screen_of(&self, page: usize, spread: bool) -> usize {
        if spread && page > 0 { page - (page + 1) % 2 } else { page }
    }

    fn placed(&self, first: usize, spread: bool, area: (f32, f32)) -> Vec<(usize, graphene::Rect)> {
        let two = spread && first > 0 && first + 1 < self.sizes.len();
        if spread && !two && self.pictures.is_none() && self.sizes.len() > 1 {
            // A PDF as a paper book: page 1 on the right, a last page on the left.
            let (w, h) = self.sizes[first];
            let half = area.0 / 2.0;
            let scale = (half / w.max(1.0)).min(area.1 / h.max(1.0));
            let x = if first == 0 { half } else { half - w * scale };
            return vec![(first, graphene::Rect::new(x, (area.1 - h * scale) / 2.0, w * scale, h * scale))];
        }
        placed(&self.sizes, first, two, self.rtl(), area)
    }

    fn last_shown(&self, spread: bool, area: (f32, f32)) -> usize {
        self.placed(self.page, spread, area).iter().map(|p| p.0).max().unwrap_or(self.page)
    }

    fn next_screen(&self, spread: bool, area: (f32, f32)) -> Option<usize> {
        Some(self.last_shown(spread, area) + 1).filter(|&p| p < self.sizes.len())
    }

    fn previous_screen(&self, spread: bool) -> Option<usize> {
        (self.page > 0).then(|| self.screen_of(self.page - 1, spread))
    }
}

type FirstFrame = Box<dyn FnOnce(&PdfView)>;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct PdfView {
        pub s: RefCell<State>,
        pub settings: Cell<Option<forzac_core::settings::Settings>>,
        /// The page the reader is on, for the engine's thread: pages left far behind are not rendered.
        pub near: Arc<AtomicUsize>,
        pub on_moved: RefCell<Option<Box<dyn Fn()>>>,
        pub on_first: RefCell<Option<FirstFrame>>,
        pub frames: RefCell<Vec<i64>>,
        pub sharp_timer: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PdfView {
        const NAME: &'static str = "ForzacPdfView";
        type Type = super::PdfView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for PdfView {}

    impl WidgetImpl for PdfView {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            let key = (width, height, self.obj().scale_factor());
            if self.s.borrow().cached_for != key {
                {
                    let mut s = self.s.borrow_mut();
                    s.cached_for = key;
                    s.textures.clear();
                    s.waiting.clear();
                    s.sharp.clear();
                    s.zoom = 1.0;
                    s.pan = (0.0, 0.0);
                    let spread = self.obj().spread();
                    s.page = s.screen_of(s.page, spread);
                }
                self.obj().prepare();
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let view = self.obj();
            let (w, h) = (view.width() as f32, view.height() as f32);
            let s = self.s.borrow();
            let page_paper = if s.pictures.is_some() { rgba(THEMES[s.theme].paper) } else { gdk::RGBA::WHITE };
            snapshot.append_color(&page_paper, &graphene::Rect::new(0.0, 0.0, w, h));
            if s.sizes.is_empty() {
                return;
            }
            let area = view.area();
            let spread = view.spread();
            snapshot.push_clip(&graphene::Rect::new(0.0, 0.0, area.0, area.1));
            let draw = |first: usize, dx: f32| {
                snapshot.save();
                snapshot.translate(&graphene::Point::new(dx + s.pan.0, s.pan.1));
                snapshot.scale(s.zoom, s.zoom);
                for (page, rect) in s.placed(first, spread, area) {
                    if let Some(t) = s.textures.get(&page) {
                        snapshot.append_scaled_texture(t, gsk::ScalingFilter::Linear, &rect);
                    }
                }
                if first == s.page {
                    for (rect, t) in &s.sharp {
                        snapshot.append_scaled_texture(t, gsk::ScalingFilter::Linear, rect);
                    }
                }
                // The match gone to, in the book search's soft bars.
                let mut bar = rgba(THEMES[s.theme].ink);
                bar.set_alpha(0.14);
                for (page, rect) in s.placed(first, spread, area) {
                    let Some((_, lines)) = s.found.as_ref().filter(|f| f.0 == page) else { continue };
                    for &[left, top, right, bottom] in lines {
                        let line = graphene::Rect::new(
                            rect.x() + left * rect.width() - 2.0,
                            rect.y() + top * rect.height(),
                            (right - left) * rect.width() + 4.0,
                            (bottom - top) * rect.height(),
                        );
                        snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(line, 4.0));
                        snapshot.append_color(&bar, &line);
                        snapshot.pop();
                    }
                }
                snapshot.restore();
            };
            match &s.slide {
                Some(slide) => {
                    let t = (slide.start.elapsed().as_secs_f32() / SLIDE.as_secs_f32()).min(1.0);
                    let eased = 1.0 - (1.0 - t).powi(3);
                    let x = (1.0 - eased) * slide.dir * area.0;
                    draw(slide.from, x - slide.dir * area.0);
                    draw(s.page, x);
                }
                None => draw(s.page, 0.0),
            }
            snapshot.pop();
            let ink = rgba(THEMES[s.theme].ink);
            let paper = rgba(THEMES[s.theme].paper);
            if s.back.is_empty() && s.pictures.is_some() {
                // The whole window is the page.
            } else if s.back.is_empty() {
                let layout = view.create_pango_layout(Some(&format!("{} / {}", s.page + 1, s.sizes.len())));
                let mut font = gtk::pango::FontDescription::from_string("Golos Text 9");
                font.set_absolute_size(12.0 * f64::from(gtk::pango::SCALE));
                layout.set_font_description(Some(&font));
                let (tw, _) = layout.pixel_size();
                snapshot.save();
                snapshot.translate(&graphene::Point::new((w - tw as f32) / 2.0, h - FOOT + 5.0));
                snapshot.append_layout(&layout, &rgba(THEMES[s.theme].muted));
                snapshot.restore();
            } else {
                let layout = view.create_pango_layout(Some(tr("← Back · Ctrl+O")));
                let (tw, th) = layout.pixel_size();
                let pill = view.pill_rect(tw as f32);
                snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(pill, pill.height() / 2.0));
                snapshot.append_color(&ink, &pill);
                snapshot.pop();
                snapshot.save();
                snapshot.translate(&graphene::Point::new(pill.x() + 18.0, pill.y() + (pill.height() - th as f32) / 2.0));
                snapshot.append_layout(&layout, &paper);
                snapshot.restore();
            }
        }
    }
}

glib::wrapper! {
    pub struct PdfView(ObjectSubclass<imp::PdfView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PdfView {
    fn new(info: &Info, pictures: Option<Arc<Book>>, theme: usize, page: usize, settings: forzac_core::settings::Settings) -> Self {
        let view: Self = glib::Object::new();
        {
            let mut s = view.imp().s.borrow_mut();
            s.doc = info.doc;
            s.pictures = pictures;
            s.theme = theme;
            s.sizes = info.sizes.clone();
            s.page = page.min(info.sizes.len() - 1);
            s.zoom = 1.0;
        }
        view.imp().settings.set(Some(settings));
        view.set_hexpand(true);
        view.set_vexpand(true);
        view.set_focusable(true);
        view.set_overflow(gtk::Overflow::Hidden);
        view
    }

    fn area(&self) -> (f32, f32) {
        let foot = if self.imp().s.borrow().pictures.is_some() { 0.0 } else { FOOT };
        (self.width() as f32, (self.height() as f32 - foot).max(1.0))
    }

    /// The window holds two pages, by the rule for books; a PDF slides whichever mode books are read in.
    fn spread(&self) -> bool {
        let mut s = self.imp().settings.get().unwrap_or_default();
        s.turning = forzac_core::settings::Turning::Slide;
        s.spread(self.width() as f32, self.height() as f32)
    }

    /// The window's theme changed, for a book of picture pages: the same textures on another paper.
    fn set_theme(&self, theme: usize) {
        self.imp().s.borrow_mut().theme = theme;
        self.queue_draw();
    }

    pub fn page(&self) -> usize {
        self.imp().s.borrow().page
    }

    pub fn pages(&self) -> usize {
        self.imp().s.borrow().sizes.len()
    }

    fn finished(&self) -> bool {
        self.imp().s.borrow().last_shown(self.spread(), self.area()) + 1 >= self.pages()
    }

    fn locator(&self) -> Locator {
        Locator { chapter: 0, block: self.page(), offset: 0 }
    }

    fn jumped(&self) -> bool {
        self.imp().s.borrow().jumped
    }

    fn pill_rect(&self, text: f32) -> graphene::Rect {
        let w = text + 36.0;
        graphene::Rect::new((self.width() as f32 - w) / 2.0, self.height() as f32 - FOOT + 1.0, w, 24.0)
    }

    /// The page and its neighbours rendered on the engine's thread, the page first; those no longer near are dropped.
    fn prepare(&self) {
        let (spread, area, scale) = (self.spread(), self.area(), self.scale_factor() as f32);
        let (doc, pictures, here, key, near) = {
            let s = self.imp().s.borrow();
            let screens = [Some(s.page), s.next_screen(spread, area), s.previous_screen(spread)];
            let near: Vec<(usize, graphene::Rect)> =
                screens.into_iter().flatten().flat_map(|first| s.placed(first, spread, area)).collect();
            (s.doc, s.pictures.clone(), s.page, s.cached_for, near)
        };
        self.imp().near.store(here, Ordering::Relaxed);
        {
            let mut s = self.imp().s.borrow_mut();
            s.textures.retain(|p, _| near.iter().any(|(n, _)| n == p));
            s.waiting.retain(|p| near.iter().any(|(n, _)| n == p));
        }
        self.notify_first();
        if self.width() <= 0 || self.height() <= 0 {
            return;
        }
        for (page, rect) in near {
            {
                let mut s = self.imp().s.borrow_mut();
                if s.textures.contains_key(&page) || !s.waiting.insert(page) {
                    continue;
                }
            }
            let full = (((rect.width() * scale).round() as i32).max(1), ((rect.height() * scale).round() as i32).max(1));
            let part = Part { page, full, origin: (0, 0), size: full, line: scale.round() as i32 };
            let (reader, book) = (self.imp().near.clone(), pictures.clone());
            let weak = self.downgrade();
            glib::spawn_future_local(async move {
                let made = pixels(doc, book, Some(reader), part).await;
                let Some(view) = weak.upgrade() else { return };
                {
                    let mut s = view.imp().s.borrow_mut();
                    if s.cached_for != key || !s.waiting.remove(&page) {
                        return;
                    }
                    if let Some(made) = made {
                        s.textures.insert(page, made.texture());
                    }
                }
                view.notify_first();
                view.queue_draw();
            });
        }
        self.queue_draw();
    }

    fn notify_first(&self) {
        let ready = {
            let s = self.imp().s.borrow();
            // An empty page of a picture book has nothing to wait for.
            s.textures.contains_key(&s.page) || (s.pictures.is_some() && !s.waiting.contains(&s.page) && s.cached_for != (0, 0, 0))
        };
        if ready && let Some(f) = self.imp().on_first.borrow_mut().take() {
            f(self);
        }
    }

    /// A turn forward or back in reading order: the slide goes the way the book turns.
    pub fn turn(&self, delta: i32) {
        let (spread, area) = (self.spread(), self.area());
        let (to, rtl) = {
            let s = self.imp().s.borrow();
            (if delta > 0 { s.next_screen(spread, area) } else { s.previous_screen(spread) }, s.rtl())
        };
        let Some(to) = to else { return };
        {
            let mut s = self.imp().s.borrow_mut();
            s.pill_turns += delta;
            if s.pill_turns.abs() >= 2 {
                s.back.clear();
            }
        }
        self.move_to(to, if (delta > 0) != rtl { 1.0 } else { -1.0 }, false);
    }

    /// A turn towards the right of the window (`towards` 1) or the left (−1).
    fn turn_side(&self, towards: i32) {
        let rtl = self.imp().s.borrow().rtl();
        self.turn(if rtl { -towards } else { towards });
    }

    /// Goes to `page` as a jump: the pill brings the reader back.
    pub fn go_to(&self, page: usize) {
        let spread = self.spread();
        let page = self.imp().s.borrow().screen_of(page, spread);
        if page >= self.pages() || page == self.page() {
            return;
        }
        {
            let mut s = self.imp().s.borrow_mut();
            let here = s.page;
            s.back.push(here);
            s.pill_turns = 0;
        }
        self.move_to(page, 0.0, true);
    }

    fn go_back(&self) {
        let to = {
            let mut s = self.imp().s.borrow_mut();
            s.pill_turns = 0;
            s.back.pop()
        };
        if let Some(to) = to {
            self.move_to(to, 0.0, true);
        }
    }

    fn move_to(&self, page: usize, dir: f32, jump: bool) {
        self.reset_zoom();
        let start = {
            let mut s = self.imp().s.borrow_mut();
            let from = s.page;
            s.page = page;
            s.jumped = jump;
            s.slide = (dir != 0.0).then(|| Slide { from, dir, start: Instant::now() });
            s.slide.is_some()
        };
        if start {
            self.animate();
        }
        self.prepare();
        if let Some(f) = self.imp().on_moved.borrow().as_ref() {
            f();
        }
    }

    /// Draws every frame of a slide, and nothing once it ends.
    fn animate(&self) {
        self.add_tick_callback(|view, clock| {
            let imp = view.imp();
            imp.frames.borrow_mut().push(clock.frame_time());
            view.queue_draw();
            let done = imp.s.borrow().slide.as_ref().is_none_or(|s| s.start.elapsed() >= SLIDE);
            if done {
                imp.s.borrow_mut().slide = None;
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    fn reset_zoom(&self) {
        let mut s = self.imp().s.borrow_mut();
        s.zoom = 1.0;
        s.pan = (0.0, 0.0);
        s.sharp.clear();
        s.sharp_ticket += 1;
    }

    fn zoom_by(&self, factor: f32, around: (f32, f32)) {
        {
            let mut s = self.imp().s.borrow_mut();
            let next = (s.zoom * factor).clamp(1.0, MAX_ZOOM);
            if next == s.zoom {
                return;
            }
            s.pan.0 = around.0 - (around.0 - s.pan.0) * next / s.zoom;
            s.pan.1 = around.1 - (around.1 - s.pan.1) * next / s.zoom;
            s.zoom = next;
            s.sharp.clear();
        }
        self.clamp_pan();
        self.schedule_sharp();
    }

    fn pan_by(&self, dx: f32, dy: f32) {
        {
            let mut s = self.imp().s.borrow_mut();
            s.pan.0 += dx;
            s.pan.1 += dy;
        }
        self.clamp_pan();
        self.schedule_sharp();
    }

    fn clamp_pan(&self) {
        let area = self.area();
        let mut s = self.imp().s.borrow_mut();
        s.pan.0 = s.pan.0.clamp(area.0 * (1.0 - s.zoom), 0.0);
        s.pan.1 = s.pan.1.clamp(area.1 * (1.0 - s.zoom), 0.0);
        drop(s);
        self.queue_draw();
    }

    /// When the gesture has been still a moment, the part in view drawn again at its zoom, over the scaled page.
    fn schedule_sharp(&self) {
        if let Some(id) = self.imp().sharp_timer.take() {
            id.remove();
        }
        let weak = self.downgrade();
        let id = glib::timeout_add_local_once(Duration::from_millis(120), move || {
            let Some(view) = weak.upgrade() else { return };
            view.imp().sharp_timer.take();
            view.make_sharp();
        });
        self.imp().sharp_timer.replace(Some(id));
    }

    fn make_sharp(&self) {
        let (area, spread, scale) = (self.area(), self.spread(), self.scale_factor() as f32);
        let (doc, pictures, zoom, pan, ticket, pages, natural) = {
            let mut s = self.imp().s.borrow_mut();
            s.sharp_ticket += 1;
            (s.doc, s.pictures.clone(), s.zoom, s.pan, s.sharp_ticket, s.placed(s.page, spread, area), s.sizes.clone())
        };
        if zoom <= 1.0 {
            return;
        }
        let visible = graphene::Rect::new(-pan.0 / zoom, -pan.1 / zoom, area.0 / zoom, area.1 / zoom);
        let px = scale * zoom;
        for (page, rect) in pages {
            let Some(part) = rect.intersection(&visible) else { continue };
            let full = ((rect.width() * px).round() as i32, (rect.height() * px).round() as i32);
            // A picture is drawn whole, at up to its own size; of a PDF, the part in view.
            let (shown, job) = if pictures.is_some() {
                let size = (full.0.min(natural[page].0 as i32).max(1), full.1.min(natural[page].1 as i32).max(1));
                (rect, Part { page, full: size, origin: (0, 0), size, line: 0 })
            } else {
                let origin = (((part.x() - rect.x()) * px).round() as i32, ((part.y() - rect.y()) * px).round() as i32);
                let size = (((part.width() * px).round() as i32).max(1), ((part.height() * px).round() as i32).max(1));
                (part, Part { page, full, origin, size, line: px.round() as i32 })
            };
            let (weak, book) = (self.downgrade(), pictures.clone());
            glib::spawn_future_local(async move {
                let made = pixels(doc, book, None, job).await;
                let (Some(view), Some(made)) = (weak.upgrade(), made) else { return };
                let mut s = view.imp().s.borrow_mut();
                if s.sharp_ticket == ticket {
                    s.sharp.push((shown, made.texture()));
                    drop(s);
                    view.queue_draw();
                }
            });
        }
    }

    /// Marks match `ordinal` of `query` on `page`.
    fn mark(&self, page: usize, query: String, ordinal: usize) {
        let (doc, weak) = (self.imp().s.borrow().doc, self.downgrade());
        glib::spawn_future_local(async move {
            let lines = ask(move |engine| match_rects(engine, doc, page, &query, ordinal)).await.flatten();
            if let (Some(view), Some(lines)) = (weak.upgrade(), lines) {
                view.imp().s.borrow_mut().found = Some((page, lines));
                view.queue_draw();
            }
        });
    }

    /// A double click: to 2× at the point, or back to 1×.
    fn toggle_zoom(&self, around: (f32, f32)) {
        if self.imp().s.borrow().zoom > 1.0 {
            self.reset_zoom();
            self.queue_draw();
        } else {
            self.zoom_by(2.0, around);
        }
    }

    /// The link under a point of the view, if any.
    async fn link_at(&self, x: f32, y: f32) -> Option<Link> {
        let (doc, zoom, pan, pages) = {
            let s = self.imp().s.borrow();
            if s.pictures.is_some() {
                return None;
            }
            (s.doc, s.zoom, s.pan, s.placed(s.page, self.spread(), self.area()))
        };
        let (ax, ay) = ((x - pan.0) / zoom, (y - pan.1) / zoom);
        let (page, rect) = pages.into_iter().find(|(_, r)| r.contains_point(&graphene::Point::new(ax, ay)))?;
        let (fx, fy) = ((ax - rect.x()) / rect.width(), (ay - rect.y()) / rect.height());
        ask(move |engine| {
            let pdf_page = engine.docs.get(&doc)?.pages().get(page as _).ok()?;
            let (px, py) = Shown::of(&pdf_page)?.point(fx, fy);
            let link = pdf_page.links().link_at_point(PdfPoints::new(px), PdfPoints::new(py))?;
            if let Some(uri) = link.action().and_then(|a| a.as_uri_action().and_then(|u| u.uri().ok())) {
                return Some(Link::External(uri));
            }
            target_page(link.destination(), link.action()).map(Link::Page)
        })
        .await
        .flatten()
    }
}

// endregion

// region input

impl PdfView {
    /// The keys of a book that mean something on a PDF; `+ - 0` zoom, and arrows pan while zoomed.
    fn key(&self, key: gdk::Key, state: gdk::ModifierType, win: &Rc<Window>) -> bool {
        use gdk::Key;
        let (ctrl, shift) = (state.contains(gdk::ModifierType::CONTROL_MASK), state.contains(gdk::ModifierType::SHIFT_MASK));
        let zoomed = self.imp().s.borrow().zoom > 1.0;
        let middle = (self.area().0 / 2.0, self.area().1 / 2.0);
        const BY: f32 = 80.0;
        let section = |forward: bool| {
            let here = self.page();
            let sections = win.outline.iter().map(|e| e.2);
            let target = if forward { sections.filter(|&p| p > here).min() } else { sections.filter(|&p| p < here).max() };
            if let Some(target) = target {
                self.go_to(target);
            }
        };
        match key {
            Key::d if ctrl => self.turn(1),
            Key::u if ctrl => self.turn(-1),
            Key::o if ctrl => self.go_back(),
            Key::space if shift => self.turn(-1),
            Key::Return | Key::KP_Enter | Key::ISO_Enter => win.menu.toggle(),
            Key::j | Key::Down | Key::KP_Down => {
                if zoomed {
                    self.pan_by(0.0, -BY)
                } else {
                    self.turn(1)
                }
            }
            Key::k | Key::Up | Key::KP_Up => {
                if zoomed {
                    self.pan_by(0.0, BY)
                } else {
                    self.turn(-1)
                }
            }
            Key::l | Key::Right | Key::KP_Right => {
                if zoomed {
                    self.pan_by(-BY, 0.0)
                } else {
                    self.turn_side(1)
                }
            }
            Key::h | Key::Left | Key::KP_Left => {
                if zoomed {
                    self.pan_by(BY, 0.0)
                } else {
                    self.turn_side(-1)
                }
            }
            Key::space | Key::Page_Down | Key::KP_Page_Down => self.turn(1),
            Key::Page_Up | Key::KP_Page_Up | Key::BackSpace => self.turn(-1),
            Key::bracketright => section(true),
            Key::bracketleft => section(false),
            Key::G | Key::End | Key::KP_End => self.go_to(self.pages().saturating_sub(1)),
            Key::g | Key::Home | Key::KP_Home => self.go_to(0),
            Key::plus | Key::equal | Key::KP_Add => self.zoom_by(1.25, middle),
            Key::minus | Key::KP_Subtract => self.zoom_by(0.8, middle),
            Key::_0 | Key::KP_0 => {
                self.reset_zoom();
                self.queue_draw();
            }
            Key::t if self.imp().s.borrow().pictures.is_some() => win.step_theme(1),
            Key::T if self.imp().s.borrow().pictures.is_some() => win.step_theme(-1),
            Key::o => win.show_contents(),
            Key::slash => win.show_search(),
            Key::question => win.help.show(),
            Key::Escape => {
                win.close_panels();
            }
            Key::f => {
                if win.window.is_fullscreen() {
                    win.window.unfullscreen();
                } else {
                    win.window.fullscreen();
                }
            }
            Key::q => win.window.close(),
            _ => return false,
        }
        true
    }

    fn connect_input(&self, win: &Rc<Window>) {
        let keys = gtk::EventControllerKey::new();
        let w = Rc::downgrade(win);
        keys.connect_key_pressed(move |c, key, code, state| {
            let Some(win) = w.upgrade() else { return glib::Propagation::Proceed };
            let key = latin(key, code);
            if win.help.key(key, key, state) || win.menu.key(key) || win.view.key(key, state, &win) {
                let _ = c;
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        self.add_controller(keys);

        // A press that is not a drag: a link, then the side thirds turn and the middle opens the menu.
        let drag = gtk::GestureDrag::new();
        let v = self.downgrade();
        let last = Rc::new(Cell::new((0.0, 0.0)));
        let l = last.clone();
        drag.connect_drag_begin(move |_, _, _| l.set((0.0, 0.0)));
        drag.connect_drag_update(move |_, dx, dy| {
            let Some(v) = v.upgrade() else { return };
            let (px, py) = last.replace((dx, dy));
            if v.imp().s.borrow().zoom > 1.0 {
                v.pan_by((dx - px) as f32, (dy - py) as f32);
            }
        });
        self.add_controller(drag);
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(win);
        click.connect_released(move |_, _, x, y| {
            if let Some(win) = w.upgrade() {
                win.click(x as f32, y as f32);
            }
        });
        self.add_controller(click);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        let pointer = Rc::new(Cell::new((0.0f32, 0.0f32)));
        let motion = gtk::EventControllerMotion::new();
        let p = pointer.clone();
        motion.connect_motion(move |_, x, y| p.set((x as f32, y as f32)));
        self.add_controller(motion);
        let v = self.downgrade();
        scroll.connect_scroll_begin(move |_| {
            if let Some(v) = v.upgrade() {
                let mut s = v.imp().s.borrow_mut();
                s.swipe = 0.0;
                s.swiped = false;
            }
        });
        let v = self.downgrade();
        scroll.connect_scroll(move |c, dx, dy| {
            let Some(v) = v.upgrade() else { return glib::Propagation::Proceed };
            let wheel = c.unit() == gdk::ScrollUnit::Wheel;
            if c.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                v.zoom_by(1.0 - dy as f32 * if wheel { 0.1 } else { 0.01 }, pointer.get());
            } else if v.imp().s.borrow().zoom > 1.0 {
                let k = if wheel { 40.0 } else { 1.0 };
                v.pan_by(-dx as f32 * k, -dy as f32 * k);
            } else if wheel {
                if dy != 0.0 {
                    v.turn(if dy > 0.0 { 1 } else { -1 });
                }
            } else {
                let turn = {
                    let mut s = v.imp().s.borrow_mut();
                    s.swipe += if dx.abs() > dy.abs() { dx } else { 0.0 };
                    let turn = !s.swiped && s.swipe.abs() > 60.0;
                    s.swiped |= turn;
                    turn.then_some(s.swipe)
                };
                if let Some(d) = turn {
                    v.turn_side(if d > 0.0 { 1 } else { -1 });
                }
            }
            glib::Propagation::Stop
        });
        self.add_controller(scroll);

        let pinch = gtk::GestureZoom::new();
        let v = self.downgrade();
        let was = Rc::new(Cell::new(1.0));
        let w2 = was.clone();
        pinch.connect_begin(move |_, _| w2.set(1.0));
        pinch.connect_scale_changed(move |g, scale| {
            let Some(v) = v.upgrade() else { return };
            let around = g.bounding_box_center().map_or((0.0, 0.0), |(x, y)| (x as f32, y as f32));
            v.zoom_by((scale / was.replace(scale)) as f32, around);
        });
        self.add_controller(pinch);
    }
}

// endregion

// region the menu and panels

/// A PDF's reading menu: as a book's, without "Aa".
struct PdfMenu {
    top: gtk::Box,
    bottom: gtk::Box,
    slider: Slider,
    page: gtk::Label,
    scrub: Cell<Option<usize>>,
    view: glib::WeakRef<PdfView>,
    open: Cell<bool>,
    theme: Cell<usize>,
}

impl PdfMenu {
    fn new(overlay: &gtk::Overlay, view: &PdfView, title: &str) -> (Rc<Self>, gtk::Button, gtk::Button) {
        let icon = |paths, tooltip: &str| {
            let b = gtk::Button::new();
            b.set_child(Some(&Icon::new(paths, 24)));
            b.add_css_class("bar-icon");
            b.set_tooltip_text(Some(tooltip));
            b.set_focusable(false);
            b
        };
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        for c in ["forzac-panel", "reader-bar", "bar-top", "open"] {
            top.add_css_class(c);
        }
        top.set_valign(gtk::Align::Start);
        let back = icon(icons::CHEVRON_LEFT, tr("Back to library"));
        let contents = icon(icons::LIST, tr("Contents"));
        let book = gtk::Label::new(Some(title));
        book.set_ellipsize(gtk::pango::EllipsizeMode::End);
        book.set_hexpand(true);
        book.add_css_class("bar-section");
        top.append(&back);
        top.append(&book);
        top.append(&contents);
        let bottom = gtk::Box::new(gtk::Orientation::Vertical, 8);
        for c in ["forzac-panel", "reader-bar", "bar-bottom", "open"] {
            bottom.add_css_class(c);
        }
        bottom.set_valign(gtk::Align::End);
        let slider = Slider::new();
        let page = gtk::Label::new(None);
        page.add_css_class("bar-page");
        page.set_xalign(0.0);
        bottom.append(&slider);
        bottom.append(&page);
        for bar in [&top, &bottom] {
            bar.set_visible(false);
            overlay.add_overlay(bar);
        }
        let menu = Rc::new(PdfMenu {
            top,
            bottom,
            slider,
            page,
            scrub: Cell::new(None),
            view: view.downgrade(),
            open: Cell::new(false),
            theme: Cell::new(view.imp().s.borrow().theme),
        });
        let drag = gtk::GestureDrag::new();
        let m = Rc::downgrade(&menu);
        drag.connect_drag_begin(move |_, x, _| {
            if let Some(m) = m.upgrade() {
                m.scrub_to(m.slider.page_at(x as f32));
            }
        });
        let m = Rc::downgrade(&menu);
        drag.connect_drag_update(move |g, dx, _| {
            if let (Some(m), Some((x, _))) = (m.upgrade(), g.start_point()) {
                m.scrub_to(m.slider.page_at((x + dx) as f32));
            }
        });
        let m = Rc::downgrade(&menu);
        drag.connect_drag_end(move |_, _, _| {
            if let Some(m) = m.upgrade() {
                m.go();
            }
        });
        menu.slider.add_controller(drag);
        (menu, back, contents)
    }

    fn toggle(&self) {
        if self.open.get() {
            self.close();
            return;
        }
        let Some(view) = self.view.upgrade() else { return };
        self.open.set(true);
        self.scrub.set(None);
        let side = 20.max((view.width() - 640) / 2);
        self.bottom.set_margin_start(0);
        self.slider.set_margin_start(side);
        self.slider.set_margin_end(side);
        self.page.set_margin_start(side);
        self.page.set_margin_bottom(14);
        self.show_place();
        self.top.set_visible(true);
        self.bottom.set_visible(true);
    }

    fn close(&self) -> bool {
        if !self.open.replace(false) {
            return false;
        }
        self.scrub.set(None);
        self.top.set_visible(false);
        self.bottom.set_visible(false);
        true
    }

    /// The keys while the menu is open, as a book's.
    fn key(&self, key: gdk::Key) -> bool {
        use gdk::Key;
        if !self.open.get() {
            return false;
        }
        let Some(view) = self.view.upgrade() else { return false };
        let from = self.scrub.get().unwrap_or_else(|| view.page());
        match key {
            Key::Left | Key::h | Key::KP_Left => self.scrub_to(from.saturating_sub(1)),
            Key::Right | Key::l | Key::KP_Right => self.scrub_to((from + 1).min(view.pages() - 1)),
            Key::Return | Key::KP_Enter | Key::ISO_Enter => self.go(),
            Key::Escape => {
                self.close();
            }
            Key::Shift_L | Key::Shift_R | Key::Control_L | Key::Control_R | Key::Alt_L | Key::Alt_R => {}
            _ => {
                self.close();
                return false;
            }
        }
        true
    }

    fn scrub_to(&self, page: usize) {
        self.scrub.set(Some(page));
        self.show_place();
    }

    fn go(&self) {
        let target = self.scrub.take();
        let view = self.view.upgrade();
        self.close();
        if let (Some(view), Some(page)) = (view, target) {
            view.go_to(page);
        }
    }

    fn show_place(&self) {
        let Some(view) = self.view.upgrade() else { return };
        let shown = self.scrub.get().unwrap_or_else(|| view.page());
        self.page.set_text(&fill(tr("Page {n} of {m}"), &[("n", &(shown + 1).to_string()), ("m", &view.pages().to_string())]));
        self.slider.set(view.pages(), shown, &[], self.theme.get());
    }
}

/// The list over a PDF's page, the contents or the search: a list view, as thousands of matches may come.
struct PdfPanel {
    root: gtk::Box,
    list: gtk::ListView,
    selection: gtk::SingleSelection,
    model: gtk::StringList,
    message: gtk::Label,
    field: Option<gtk::SearchEntry>,
    pages: RefCell<Vec<usize>>,
    /// In the search: the query, and each row's match among its page's.
    query: RefCell<String>,
    ordinals: RefCell<Vec<usize>>,
}

impl PdfPanel {
    fn new(overlay: &gtk::Overlay, title: &str, search: bool) -> Rc<Self> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        for c in ["forzac-panel", "pdf-panel"] {
            root.add_css_class(c);
        }
        root.set_halign(gtk::Align::End);
        root.set_width_request(360);
        root.set_visible(false);
        let heading = gtk::Label::new(Some(title));
        heading.set_xalign(0.0);
        heading.add_css_class("title");
        root.append(&heading);
        let field = search.then(|| {
            let f = gtk::SearchEntry::new();
            f.set_placeholder_text(Some(tr("Search in book")));
            f.add_css_class("search");
            root.append(&f);
            f
        });
        let message = gtk::Label::new(None);
        message.set_wrap(true);
        message.set_xalign(0.0);
        message.add_css_class("muted");
        message.set_visible(false);
        root.append(&message);
        let model = gtk::StringList::new(&[]);
        let selection = gtk::SingleSelection::new(Some(model.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let text = gtk::Label::new(None);
            text.set_wrap(true);
            text.set_xalign(0.0);
            text.set_hexpand(true);
            let number = gtk::Label::new(None);
            number.add_css_class("muted");
            row.append(&text);
            row.append(&number);
            item.downcast_ref::<gtk::ListItem>().expect("a list item").set_child(Some(&row));
        });
        factory.connect_bind(|_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
            let (Some(row), Some(entry)) = (item.child(), item.item().and_downcast::<gtk::StringObject>()) else { return };
            let entry = entry.string();
            let mut fields = entry.splitn(3, '\t');
            let (indent, page, markup) = (fields.next().unwrap_or("0"), fields.next().unwrap_or(""), fields.next().unwrap_or(""));
            let text = row.first_child().and_downcast::<gtk::Label>().expect("the row's text");
            text.set_markup(markup);
            text.set_margin_start(indent.parse::<i32>().unwrap_or(0) * 14);
            row.last_child().and_downcast::<gtk::Label>().expect("the row's page").set_text(page);
        });
        let list = gtk::ListView::new(Some(selection.clone()), Some(factory));
        list.set_single_click_activate(true);
        let scrolled = gtk::ScrolledWindow::builder().child(&list).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
        root.append(&scrolled);
        overlay.add_overlay(&root);
        Rc::new(PdfPanel {
            root,
            list,
            selection,
            model,
            message,
            field,
            pages: RefCell::default(),
            query: RefCell::default(),
            ordinals: RefCell::default(),
        })
    }

    fn clear(&self) {
        self.model.splice(0, self.model.n_items(), &[]);
        self.pages.borrow_mut().clear();
        self.ordinals.borrow_mut().clear();
        self.say(None);
    }

    fn say(&self, text: Option<&str>) {
        self.message.set_text(text.unwrap_or(""));
        self.message.set_visible(text.is_some());
    }

    /// Adds a row: `markup` in Pango's, the page on the right.
    fn add(&self, markup: &str, indent: usize, page: usize) {
        self.model.append(&format!("{indent}\t{}\t{markup}", page + 1));
        self.pages.borrow_mut().push(page);
    }

    /// Selects a row and shows it.
    fn select(&self, row: u32) {
        self.list.scroll_to(row, gtk::ListScrollFlags::SELECT | gtk::ListScrollFlags::FOCUS, None);
    }
}

// endregion

// region the window

/// A PDF's window.
struct Window {
    window: gtk::ApplicationWindow,
    view: PdfView,
    overlay: gtk::Overlay,
    help: Rc<crate::help::Help>,
    menu: Rc<PdfMenu>,
    outline: Vec<(String, usize, usize)>,
    contents: RefCell<Option<Rc<PdfPanel>>>,
    search: RefCell<Option<Rc<PdfPanel>>>,
    search_ticket: Cell<u64>,
    /// The menu, waiting to see whether a click on the middle was half a double click.
    pending_menu: RefCell<Option<glib::SourceId>>,
    doc: u32,
    count: usize,
    /// A book of picture pages follows the reader's theme: its settings, where they are saved, whether to save.
    settings: Cell<forzac_core::settings::Settings>,
    store: SharedStore,
    saves: bool,
}

impl Window {
    /// `t` and `T` in a book of picture pages: the next or previous theme, saved as a book's.
    fn step_theme(&self, delta: i32) {
        let mut settings = self.settings.get();
        let dark = crate::appearance::dark();
        settings.step(forzac_core::settings::Setting::Theme, delta, dark);
        self.settings.set(settings);
        if self.saves
            && let Err(e) = settings.save(&mut self.store.borrow_mut())
        {
            eprintln!("cannot save the reading state: {e}");
        }
        self.show_theme(settings.theme_index(dark));
    }

    fn show_theme(&self, theme: usize) {
        crate::panels::set_theme(theme);
        pictures_chrome(theme);
        self.menu.theme.set(theme);
        self.view.set_theme(theme);
    }

    /// A click on the page: closes what is over it, else the pill, a link, then the side thirds turn and the middle
    /// opens the menu.
    fn click(self: &Rc<Self>, x: f32, y: f32) {
        let view = self.view.clone();
        if view.imp().s.borrow_mut().found.take().is_some() {
            view.queue_draw();
        }
        if self.close_panels() {
            return;
        }
        if !view.imp().s.borrow().back.is_empty() && view.pill_rect(120.0).contains_point(&graphene::Point::new(x, y)) {
            return view.go_back();
        }
        let (window, w) = (self.window.clone(), Rc::downgrade(self));
        glib::spawn_future_local(async move {
            match view.link_at(x, y).await {
                Some(Link::Page(p)) => view.go_to(p),
                Some(Link::External(uri)) => gtk::UriLauncher::new(&uri).launch(Some(&window), gio::Cancellable::NONE, |_| {}),
                None if x >= view.width() as f32 / 3.0 && x <= view.width() as f32 * 2.0 / 3.0 => {
                    if let Some(win) = w.upgrade() {
                        win.clicked_middle((x, y));
                    }
                }
                None if view.imp().s.borrow().zoom == 1.0 => view.turn_side(if x < view.width() as f32 / 3.0 { -1 } else { 1 }),
                None => {}
            }
        });
    }

    /// The second click of a double click zooms to 2× at the point, or back to 1×; a single one opens the menu.
    fn clicked_middle(self: &Rc<Self>, at: (f32, f32)) {
        if let Some(waiting) = self.pending_menu.take() {
            waiting.remove();
            return self.view.toggle_zoom(at);
        }
        let system = self.window.settings().gtk_double_click_time().max(0) as u64;
        let w = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(DOUBLE_CLICK.min(Duration::from_millis(system)), move || {
            if let Some(win) = w.upgrade() {
                win.pending_menu.take();
                win.menu.toggle();
            }
        });
        self.pending_menu.replace(Some(id));
    }

    /// Goes to row `row` of `panel`, and marks a match on its page.
    fn pick(&self, panel: &PdfPanel, row: usize) {
        let page = panel.pages.borrow().get(row).copied();
        let ordinal = panel.ordinals.borrow().get(row).copied();
        self.close_panels();
        let Some(page) = page else { return };
        self.view.go_to(page);
        if let Some(ordinal) = ordinal {
            self.view.mark(page, panel.query.borrow().clone(), ordinal);
        }
    }

    /// Closes whatever is over the page; false when nothing was.
    fn close_panels(&self) -> bool {
        let mut closed = self.menu.close();
        for panel in [&self.contents, &self.search] {
            if let Some(p) = panel.borrow().as_ref().filter(|p| p.root.is_visible()) {
                p.root.set_visible(false);
                closed = true;
            }
        }
        self.search_ticket.set(self.search_ticket.get() + 1);
        if closed {
            self.view.grab_focus();
        }
        closed
    }

    fn connect_panel(self: &Rc<Self>, panel: &Rc<PdfPanel>) {
        let w = Rc::downgrade(self);
        let p = Rc::downgrade(panel);
        panel.list.connect_activate(move |_, row| {
            if let (Some(w), Some(p)) = (w.upgrade(), p.upgrade()) {
                w.pick(&p, row as usize);
            }
        });
        let keys = gtk::EventControllerKey::new();
        let (w, p) = (Rc::downgrade(self), Rc::downgrade(panel));
        keys.connect_key_pressed(move |_, key, code, _| {
            let (Some(w), Some(p)) = (w.upgrade(), p.upgrade()) else { return glib::Propagation::Proceed };
            let in_field = p.field.as_ref().is_some_and(|f| f.focus_child().is_some());
            let count = p.model.n_items();
            let selected = p.selection.selected();
            match latin(key, code) {
                gdk::Key::Escape => {
                    w.close_panels();
                }
                gdk::Key::Down if count > 0 => {
                    p.select(if selected == gtk::INVALID_LIST_POSITION { 0 } else { (selected + 1).min(count - 1) })
                }
                gdk::Key::j if !in_field && count > 0 => {
                    p.select(if selected == gtk::INVALID_LIST_POSITION { 0 } else { (selected + 1).min(count - 1) })
                }
                gdk::Key::Up if count > 0 && selected != gtk::INVALID_LIST_POSITION => p.select(selected.saturating_sub(1)),
                gdk::Key::k if !in_field && count > 0 && selected != gtk::INVALID_LIST_POSITION => p.select(selected.saturating_sub(1)),
                gdk::Key::o | gdk::Key::q if p.field.is_none() => {
                    w.close_panels();
                }
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        });
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        panel.root.add_controller(keys);
    }

    fn show_contents(self: &Rc<Self>) {
        let open = self.contents.borrow().as_ref().is_some_and(|p| p.root.is_visible());
        self.close_panels();
        if open {
            return;
        }
        let panel = self
            .contents
            .borrow_mut()
            .get_or_insert_with(|| {
                let panel = PdfPanel::new(&self.overlay, tr("Contents"), false);
                for (title, depth, page) in &self.outline {
                    panel.add(&glib::markup_escape_text(title), *depth, *page);
                }
                if self.outline.is_empty() {
                    panel.say(Some(tr("No contents")));
                }
                self.connect_panel(&panel);
                panel
            })
            .clone();
        panel.root.set_visible(true);
        let here = self.view.page();
        let at = panel.pages.borrow().iter().rposition(|&p| p <= here);
        match at {
            // Once it is laid out, or it does not scroll into view.
            Some(i) => {
                let p = panel.clone();
                glib::idle_add_local_once(move || p.select(i as u32));
            }
            None => {
                panel.list.grab_focus();
            }
        }
    }

    fn show_search(self: &Rc<Self>) {
        if self.doc == 0 {
            return; // a book of picture pages has no text
        }
        let open = self.search.borrow().as_ref().is_some_and(|p| p.root.is_visible());
        self.close_panels();
        if open {
            return;
        }
        let panel = self
            .search
            .borrow_mut()
            .get_or_insert_with(|| {
                let panel = PdfPanel::new(&self.overlay, tr("Search"), true);
                self.connect_panel(&panel);
                let w = Rc::downgrade(self);
                if let Some(field) = &panel.field {
                    field.connect_search_changed(move |f| {
                        if let Some(w) = w.upgrade() {
                            w.run_search(f.text().to_string());
                        }
                    });
                    let (w, p) = (Rc::downgrade(self), Rc::downgrade(&panel));
                    field.connect_activate(move |_| {
                        if let (Some(w), Some(p)) = (w.upgrade(), p.upgrade()) {
                            w.pick(&p, 0);
                        }
                    });
                }
                panel
            })
            .clone();
        panel.root.set_visible(true);
        if let Some(field) = &panel.field {
            field.grab_focus();
        }
    }

    /// Reads the pages' text in the background a page at a time, so a turn waits at most for one page's text; results
    /// show as they are found. A newer query stops the old one.
    fn run_search(self: &Rc<Self>, query: String) {
        let ticket = self.search_ticket.get() + 1;
        self.search_ticket.set(ticket);
        let Some(panel) = self.search.borrow().clone() else { return };
        panel.clear();
        panel.query.replace(query.clone());
        if query.trim().chars().count() < 2 {
            return;
        }
        let (weak, doc, count) = (Rc::downgrade(self), self.doc, self.count);
        glib::spawn_future_local(async move {
            let mut had_text = false;
            for page in 0..count {
                let text = ask(move |engine| {
                    let pages = engine.docs.get(&doc)?;
                    pages.pages().get(page as _).ok()?.text().ok().map(|t| t.all())
                })
                .await
                .flatten()
                .unwrap_or_default();
                let Some(win) = weak.upgrade().filter(|w| w.search_ticket.get() == ticket) else { return };
                had_text |= !text.trim().is_empty();
                for (ordinal, hit) in forzac_core::search::search_page(&text, &query).into_iter().enumerate() {
                    let (snippet, matched) = (hit.snippet, hit.matched);
                    let markup = format!(
                        "{}<b>{}</b>{}",
                        glib::markup_escape_text(&snippet[..matched.start]),
                        glib::markup_escape_text(&snippet[matched.clone()]),
                        glib::markup_escape_text(&snippet[matched.end..])
                    );
                    panel.add(&markup, 0, page);
                    panel.ordinals.borrow_mut().push(ordinal);
                }
                drop(win);
                // Below the frame clock: a turn's frames come first.
                glib::timeout_future_with_priority(glib::Priority::DEFAULT_IDLE, Duration::ZERO).await;
            }
            if panel.pages.borrow().is_empty() {
                panel.say(Some(if had_text {
                    tr("No matches")
                } else {
                    tr("The pages of this PDF are scanned: there is no text to search")
                }));
            }
        });
    }
}

/// Saves the place and counts the reading, as a book's window does (`reading-log`).
fn remember(view: &PdfView, window: &gtk::ApplicationWindow, book: Arc<Book>, path: &Path, store: &SharedStore) {
    let reading = Rc::new(RefCell::new(log::Reading::new(now_ms())));
    let event: Rc<dyn Fn(log::Event)> = {
        let (view, reading, book) = (view.downgrade(), reading.clone(), book.clone());
        Rc::new(move |e| {
            if let Some(view) = view.upgrade() {
                reading.borrow_mut().event(&book, e, view.locator(), now_ms(), utc_offset());
            }
        })
    };
    let append: Rc<dyn Fn(bool)> = {
        let (ev, store, reading, id) = (event.clone(), store.clone(), reading.clone(), book.id.clone());
        Rc::new(move |leaving| {
            if leaving {
                ev(log::Event::Paused);
            }
            let sums = reading.borrow_mut().due(now_ms(), leaving);
            if let Err(e) = log::append(&mut store.borrow_mut(), &id, &sums, now_ms()) {
                eprintln!("cannot write the reading log: {e}");
            }
        })
    };
    let save: Rc<dyn Fn()> = {
        let (view, store, path, append, book) = (view.downgrade(), store.clone(), path.to_owned(), append.clone(), book.clone());
        Rc::new(move || {
            let Some(view) = view.upgrade() else { return };
            if let Err(e) = store.borrow_mut().save_position(&book, &path, view.locator(), view.finished()) {
                eprintln!("cannot save the reading state: {e}");
            }
            if view.finished()
                && let Err(e) = log::finish(&store.borrow(), &book, now_ms())
            {
                eprintln!("cannot write the reading log: {e}");
            }
            append(false);
        })
    };
    save();
    if let Err(e) = log::opened(&store.borrow(), &book.id, now_ms()) {
        eprintln!("cannot write the reading log: {e}");
    }
    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    let (p, s, ev, v) = (pending.clone(), save.clone(), event.clone(), view.downgrade());
    *view.imp().on_moved.borrow_mut() = Some(Box::new(move || {
        if let Some(view) = v.upgrade() {
            ev(if view.jumped() { log::Event::Jumped } else { log::Event::Moved });
        }
        if let Some(id) = p.borrow_mut().take() {
            id.remove();
        }
        let (p2, s2) = (p.clone(), s.clone());
        *p.borrow_mut() = Some(glib::timeout_add_local_once(SAVE_DELAY, move || {
            p2.borrow_mut().take();
            s2();
        }));
    }));
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ev = event.clone();
    keys.connect_key_pressed(move |_, _, _, _| {
        ev(log::Event::Active);
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
    let click = gtk::GestureClick::new();
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ev = event.clone();
    click.connect_pressed(move |_, _, _, _| ev(log::Event::Active));
    window.add_controller(click);
    let (a, s) = (append, save);
    window.connect_close_request(move |_| {
        if let Some(id) = pending.borrow_mut().take() {
            id.remove();
        }
        s();
        a(true);
        glib::Propagation::Proceed
    });
}

/// The rules for a window's bars and panels in `theme`, after the theme's.
fn chrome_css(class: &str, theme: usize) -> String {
    let hex = |c: u32| format!("#{c:06x}");
    let p = THEMES[theme];
    let (paper, ink, muted, line, sheet) = (hex(p.paper), hex(p.ink), hex(p.muted), hex(p.line), hex(p.sheet));
    format!(
        ".{class} .forzac-panel {{ color: {ink}; }}
         .{class} .forzac-panel .muted {{ color: {muted}; }}
         .{class} .reader-bar {{ background-color: {paper}; }}
         .{class} .bar-top {{ box-shadow: inset 0 -1px {line}; }}
         .{class} .bar-bottom {{ box-shadow: inset 0 1px {line}; padding-top: 12px; }}
         .{class} .pdf-panel {{ background-color: {sheet}; padding: 20px 14px 0 14px; box-shadow: 0 0 16px alpha(black, 0.16); }}
         .{class} .pdf-panel list {{ background: none; color: {ink}; }}
         .{class} .pdf-panel row {{ padding: 6px; }}
         .{class} .pdf-panel row:selected {{ background-color: alpha({ink}, 0.07); color: {ink}; }}
         .{class} .pdf-panel .title {{ font-size: 17px; font-weight: 500; }}
         .{class} .pdf-panel scrolledwindow {{ background: none; }}
         .{class} .help-card {{ background-color: {sheet}; box-shadow: 0 0 0 1px {line}, 0 12px 32px alpha(black, 0.16); color: {ink}; }}"
    )
}

fn add_provider(css: &str) -> gtk::CssProvider {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(css);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    }
    provider
}

/// The Light theme over a PDF, whatever the reader's theme is.
fn light_chrome() {
    thread_local!(static DONE: Cell<bool> = const { Cell::new(false) });
    if !DONE.replace(true) {
        add_provider(&chrome_css("pdf-chrome", 0));
    }
}

/// The reader's theme over a book of picture pages.
fn pictures_chrome(theme: usize) {
    thread_local!(static PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) });
    let css = chrome_css("pictures-chrome", theme);
    PROVIDER.with(|p| {
        let mut p = p.borrow_mut();
        match p.as_ref() {
            Some(provider) => provider.load_from_string(&css),
            None => *p = Some(add_provider(&css)),
        }
    });
}

/// Opens the PDF or the book of picture pages at `path` (`book`, as the core parsed it) in a window of its own.
pub fn open_window(
    app: &gtk::Application,
    path: &Path,
    mut book: Book,
    options: &Options,
    store: &SharedStore,
) -> Result<gtk::ApplicationWindow, Error> {
    let stateless = options.stateless();
    let is_pdf = book.pdf.is_some();
    let info = if is_pdf {
        let file = path.to_owned();
        // Opening a window waits for this, as parsing a book does.
        let info = call(move |engine| open_doc(engine, &file)).ok_or(Error::Unavailable)??;
        book.pdf = Some(info.sizes.len());
        if let Some(title) = forzac_core::library::pdf_title(&info.title) {
            book.title = title.to_owned();
        }
        if let Some(author) = forzac_core::library::pdf_author(&info.title, &info.author) {
            book.authors = vec![author.to_owned()];
        }
        info
    } else {
        picture_info(&book)
    };
    let (at, settings) = if stateless {
        (Locator { block: options.page, ..Locator::default() }, Default::default())
    } else {
        let s = store.borrow();
        (s.position(&book, path).unwrap_or_default(), forzac_core::settings::Settings::load(&s))
    };
    let theme = settings.theme_index(crate::appearance::dark());
    crate::panels::set_theme(theme);
    if is_pdf {
        light_chrome();
    } else {
        pictures_chrome(theme);
    }
    crate::fonts::add_interface_fonts();
    let book = Arc::new(book);
    let view = PdfView::new(&info, (!is_pdf).then(|| book.clone()), if is_pdf { 0 } else { theme }, at.block, settings);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&view));
    let help = crate::help::Help::new(false);
    let outer = gtk::Overlay::new();
    outer.set_child(Some(&overlay));
    outer.add_overlay(&help.root);
    let (width, height) = match options.size {
        Some((w, h)) => (w, h),
        None if stateless => fitted(760, 980),
        None => crate::remembered_size(&store.borrow(), "window-book", (760, 980)),
    };
    let title = if book.title.is_empty() { tr("Forzac").to_owned() } else { fill(tr("{title} — Forzac"), &[("title", &book.title)]) };
    let window =
        gtk::ApplicationWindow::builder().application(app).title(&title).default_width(width).default_height(height).child(&outer).build();
    window.add_css_class(if is_pdf { "pdf-chrome" } else { "pictures-chrome" });
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
        window.set_decorated(false);
    }
    let (menu, back, contents_button) = PdfMenu::new(&overlay, &view, &book.title);
    let win = Rc::new(Window {
        window: window.clone(),
        view: view.clone(),
        overlay,
        help,
        menu,
        count: info.sizes.len(),
        doc: info.doc,
        outline: info.outline,
        contents: RefCell::default(),
        search: RefCell::default(),
        search_ticket: Cell::new(0),
        pending_menu: RefCell::default(),
        settings: Cell::new(settings),
        store: store.clone(),
        saves: !stateless,
    });
    view.connect_input(&win);
    if !is_pdf && !stateless {
        let w = Rc::downgrade(&win);
        let watch = Cell::new(crate::appearance::watch(move |dark| {
            if let Some(w) = w.upgrade() {
                w.show_theme(w.settings.get().theme_index(dark));
            }
        }));
        window.connect_destroy(move |_| drop(watch.take()));
    }
    let w = Rc::downgrade(&win);
    contents_button.connect_clicked(move |_| {
        if let Some(w) = w.upgrade() {
            w.show_contents();
        }
    });
    let (w, app2, options2, store2) = (window.downgrade(), app.downgrade(), options.clone(), store.clone());
    back.connect_clicked(move |_| {
        if let Some(w) = w.upgrade() {
            w.close();
        }
        if let Some(app) = app2.upgrade().filter(|_| !stateless) {
            crate::library_window(&app, &options2, &store2, |_| {});
        }
    });
    measure(&view, &window, options, &win);
    // The window keeps its panels; the document goes with it.
    let held = RefCell::new(Some(win));
    let doc = info.doc;
    window.connect_destroy(move |_| {
        held.borrow_mut().take();
        if is_pdf {
            drop(gio::spawn_blocking(move || call(move |engine| engine.docs.remove(&doc).is_some())));
        }
    });
    if !stateless {
        remember(&view, &window, book.clone(), path, store);
        if is_pdf {
            remember_cover(&book, path);
        } else {
            crate::library::cache_cover(&book);
        }
        crate::remember_size(&window, store, "window-book");
        crate::keep_awake(app, &window);
    }
    window.present();
    view.grab_focus();
    Ok(window)
}

/// What a book of picture pages holds, in the terms of a PDF: an empty page takes its neighbour's size.
fn picture_info(book: &Book) -> Info {
    let mut last = (2.0, 3.0);
    let pages = book.pictures.as_ref().map_or(&[][..], |p| p.pages.as_slice());
    let sizes = pages
        .iter()
        .map(|id| {
            if let Some(size) = id.as_ref().and_then(|id| book.images.get(id)).and_then(|i| forzac_core::images::size(&i.bytes)) {
                last = size;
            }
            last
        })
        .collect();
    let outline = book.toc.iter().map(|e| (e.title.clone(), e.level.saturating_sub(1) as usize, e.target.block)).collect();
    Info { doc: 0, sizes, title: String::new(), author: String::new(), outline }
}

/// The library's cover, when it has none yet: the first page.
fn remember_cover(book: &Book, path: &Path) {
    if crate::library::has_thumbnail(&book.id) {
        return;
    }
    let (id, path) = (book.id.clone(), path.to_owned());
    drop(gio::spawn_blocking(move || crate::library::write_thumbnail(&id, cover(&path).as_ref().map(|c| c.0.as_slice()))));
}

/// `--png` and `--turns` on a PDF, as a book's: once its first page shows.
fn measure(view: &PdfView, window: &gtk::ApplicationWindow, options: &Options, win: &Rc<Window>) {
    if !(options.measure || options.png.is_some() || options.turns > 0) {
        return;
    }
    let (options, window, start) = (options.clone(), window.clone(), options.start);
    let args: Vec<String> = std::env::args().collect();
    let keys: Vec<char> = args.iter().position(|a| a == "--keys").and_then(|i| args.get(i + 1)).map_or(Vec::new(), |k| k.chars().collect());
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let (query, at, pick) = (value("--query"), value("--click"), args.iter().any(|a| a == "--pick"));
    let win = Rc::downgrade(win);
    *view.imp().on_first.borrow_mut() = Some(Box::new(move |view| {
        println!("first page: {:?} after start, {} pages", start.elapsed(), view.pages());
        let (turns, view, window, png) = (options.turns, view.clone(), window.clone(), options.png.clone());
        let left = Cell::new(turns);
        let wait = Cell::new(2);
        let (keys, win) = (keys.clone(), win.clone());
        let next_key = Cell::new(0);
        let (query, at, pick) = (query.clone(), RefCell::new(at.clone()), Cell::new(pick));
        if let (Some(query), Some(win)) = (query, win.upgrade()) {
            win.show_search();
            if let Some(field) = win.search.borrow().as_ref().and_then(|p| p.field.clone()) {
                field.set_text(&query);
                win.run_search(query);
            }
        }
        glib::timeout_add_local(Duration::from_millis(400), move || {
            if wait.get() > 0 {
                wait.set(wait.get() - 1);
                return glib::ControlFlow::Continue;
            }
            if pick.take()
                && let Some(win) = win.upgrade()
                && let Some(panel) = win.search.borrow().clone()
            {
                win.pick(&panel, 0);
                return glib::ControlFlow::Continue;
            }
            // x,y and, for a double click, the number of clicks.
            if let (Some(at), Some(win)) = (at.borrow_mut().take(), win.upgrade()) {
                let at: Vec<f32> = at.split(',').filter_map(|n| n.parse().ok()).collect();
                if let [x, y, ..] = at[..] {
                    for _ in 0..at.get(2).map_or(1, |&n| n as usize) {
                        win.click(x, y);
                    }
                }
                return glib::ControlFlow::Continue;
            }
            if let (Some(&c), Some(win)) = (keys.get(next_key.get()), win.upgrade()) {
                next_key.set(next_key.get() + 1);
                let key = if c == '\r' {
                    gdk::Key::Return
                } else {
                    gdk::Key::from_name(match c {
                        '=' => "equal".to_owned(),
                        '-' => "minus".to_owned(),
                        '/' => "slash".to_owned(),
                        ' ' => "space".to_owned(),
                        c => c.to_string(),
                    })
                    .unwrap_or(gdk::Key::VoidSymbol)
                };
                let _ = win.help.key(key, key, gdk::ModifierType::empty())
                    || win.menu.key(key)
                    || win.view.key(key, gdk::ModifierType::empty(), &win);
                return glib::ControlFlow::Continue;
            }
            if left.get() > 0 {
                left.set(left.get() - 1);
                view.turn(1);
                return glib::ControlFlow::Continue;
            }
            if turns > 0 {
                crate::report_frames(0, view.imp().frames.borrow().windows(2).map(|w| w[1] - w[0]).filter(|&d| d < 100_000).collect(), &[]);
            }
            if let Some(png) = &png {
                crate::save_window_png(&window, png);
            }
            window.close();
            glib::ControlFlow::Break
        });
    }));
}

// endregion
