//! A PDF's window (`pdf-reading`): pages as on paper, drawn by PDFium off the main thread, one at a time with the
//! neighbours ahead; the same keys, clicks and "Slide" as a book's. Its menu and panels use the Light theme, whatever the
//! reader's theme is. The core only recognises the type and keeps the place: `Locator(0, page, 0)`.
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
    let title = doc.metadata().get(PdfDocumentMetadataTagType::Title).map(|t| t.value().to_owned()).unwrap_or_default();
    let mut outline = Vec::new();
    if let Some(root) = doc.bookmarks().root() {
        walk(root, 0, sizes.len(), &mut outline);
    }
    engine.next += 1;
    let id = engine.next;
    engine.docs.insert(id, doc);
    Ok(Info { doc: id, sizes, title, outline })
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

/// A PDF's first page as a PNG and its title, for the library; off the main thread.
pub fn cover(path: &Path) -> Option<(Vec<u8>, String)> {
    let path = path.to_owned();
    let (raw, w, h, title) = call(move |engine| {
        let doc = engine.pdfium?.load_pdf_from_file(&path, None).ok()?;
        let page = doc.pages().get(0).ok()?;
        let config = PdfRenderConfig::new().set_target_height(800).set_format(PdfBitmapFormat::BGRA).set_reverse_byte_order(true);
        let bitmap = page.render_with_config(&config).ok()?;
        let title = doc.metadata().get(PdfDocumentMetadataTagType::Title).map(|t| t.value().to_owned()).unwrap_or_default();
        Some((bitmap.as_raw_bytes(), bitmap.width(), bitmap.height(), title))
    })??;
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_mut_slice(raw, gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, w, h, w * 4);
    Some((pixbuf.save_to_bufferv("png", &[]).ok()?, title))
}

/// Where a link goes.
enum Link {
    Page(usize),
    External(String),
}

/// The pixels of part of a page: the page `full` pixels large, its part from `origin`, `size` pixels large.
struct Part {
    page: usize,
    full: (i32, i32),
    origin: (i32, i32),
    size: (i32, i32),
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
    let bytes = bitmap.as_raw_bytes();
    let stride = bytes.len() as i32 / part.size.1;
    Some((bytes, stride))
}

fn texture(bytes: Vec<u8>, size: (i32, i32), stride: i32) -> gdk::Texture {
    gdk::MemoryTexture::new(size.0, size.1, gdk::MemoryFormat::B8g8r8a8Premultiplied, &glib::Bytes::from_owned(bytes), stride as usize)
        .upcast()
}

// endregion

// region the pages

/// The pages from `first` (two in a spread) fitted to `area` at one scale.
fn placed(sizes: &[(f32, f32)], first: usize, spread: bool, area: (f32, f32)) -> Vec<(usize, graphene::Rect)> {
    let count = if spread && first + 1 < sizes.len() { 2 } else { 1 };
    let shown = &sizes[first..first + count];
    let width: f32 = shown.iter().map(|s| s.0).sum();
    let height = shown.iter().map(|s| s.1).fold(1.0, f32::max);
    let scale = (area.0 / width.max(1.0)).min(area.1 / height);
    let mut x = (area.0 - width * scale) / 2.0;
    shown
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let rect = graphene::Rect::new(x, (area.1 - s.1 * scale) / 2.0, s.0 * scale, s.1 * scale);
            x += s.0 * scale;
            (first + i, rect)
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
                }
                self.obj().prepare();
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let view = self.obj();
            let (w, h) = (view.width() as f32, view.height() as f32);
            snapshot.append_color(&gdk::RGBA::WHITE, &graphene::Rect::new(0.0, 0.0, w, h));
            let s = self.s.borrow();
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
                for (page, rect) in placed(&s.sizes, first, spread, area) {
                    if let Some(t) = s.textures.get(&page) {
                        snapshot.append_scaled_texture(t, gsk::ScalingFilter::Linear, &rect);
                    }
                }
                for (rect, t) in &s.sharp {
                    snapshot.append_scaled_texture(t, gsk::ScalingFilter::Linear, rect);
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
            let ink = rgba(THEMES[0].ink);
            let paper = rgba(THEMES[0].paper);
            if s.back.is_empty() {
                let layout = view.create_pango_layout(Some(&format!("{} / {}", s.page + 1, s.sizes.len())));
                let mut font = gtk::pango::FontDescription::from_string("Golos Text 9");
                font.set_absolute_size(12.0 * f64::from(gtk::pango::SCALE));
                layout.set_font_description(Some(&font));
                let (tw, _) = layout.pixel_size();
                snapshot.save();
                snapshot.translate(&graphene::Point::new((w - tw as f32) / 2.0, h - FOOT + 5.0));
                snapshot.append_layout(&layout, &rgba(THEMES[0].muted));
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
    fn new(info: &Info, page: usize, settings: forzac_core::settings::Settings) -> Self {
        let view: Self = glib::Object::new();
        {
            let mut s = view.imp().s.borrow_mut();
            s.doc = info.doc;
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
        (self.width() as f32, (self.height() as f32 - FOOT).max(1.0))
    }

    /// The window holds two pages, by the rule for books; a PDF slides whichever mode books are read in.
    fn spread(&self) -> bool {
        let mut s = self.imp().settings.get().unwrap_or_default();
        s.turning = forzac_core::settings::Turning::Slide;
        s.spread(self.width() as f32, self.height() as f32)
    }

    fn step(&self) -> usize {
        if self.spread() { 2 } else { 1 }
    }

    pub fn page(&self) -> usize {
        self.imp().s.borrow().page
    }

    pub fn pages(&self) -> usize {
        self.imp().s.borrow().sizes.len()
    }

    fn finished(&self) -> bool {
        self.page() + self.step() >= self.pages()
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
        let (step, spread, area, scale) = (self.step(), self.spread(), self.area(), self.scale_factor() as f32);
        let (doc, here, key, near) = {
            let s = self.imp().s.borrow();
            let screens = [Some(s.page), s.page.checked_add(step).filter(|&p| p < s.sizes.len()), s.page.checked_sub(step)];
            let near: Vec<(usize, graphene::Rect)> =
                screens.into_iter().flatten().flat_map(|first| placed(&s.sizes, first, spread, area)).collect();
            (s.doc, s.page, s.cached_for, near)
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
            let part = Part { page, full, origin: (0, 0), size: full };
            let reader = self.imp().near.clone();
            let weak = self.downgrade();
            glib::spawn_future_local(async move {
                let pixels = ask(move |engine| {
                    // Flipped past meanwhile.
                    if reader.load(Ordering::Relaxed).abs_diff(page) > 4 {
                        return None;
                    }
                    render(engine, doc, &part)
                })
                .await
                .flatten();
                let Some(view) = weak.upgrade() else { return };
                {
                    let mut s = view.imp().s.borrow_mut();
                    if s.cached_for != key || !s.waiting.remove(&page) {
                        return;
                    }
                    if let Some((bytes, stride)) = pixels {
                        s.textures.insert(page, texture(bytes, full, stride));
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
            s.textures.contains_key(&s.page)
        };
        if ready && let Some(f) = self.imp().on_first.borrow_mut().take() {
            f(self);
        }
    }

    pub fn turn(&self, delta: i32) {
        let step = self.step() as i64;
        let (here, count) = (self.page() as i64, self.pages() as i64);
        let target = here + i64::from(delta) * step;
        if target < 0 && here == 0 || target >= count {
            return;
        }
        {
            let mut s = self.imp().s.borrow_mut();
            s.pill_turns += delta;
            if s.pill_turns.abs() >= 2 {
                s.back.clear();
            }
        }
        self.move_to(target.max(0) as usize, if delta > 0 { 1.0 } else { -1.0 }, false);
    }

    /// Goes to `page` as a jump: the pill brings the reader back.
    pub fn go_to(&self, page: usize) {
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
        let (doc, zoom, pan, ticket, pages) = {
            let mut s = self.imp().s.borrow_mut();
            s.sharp_ticket += 1;
            (s.doc, s.zoom, s.pan, s.sharp_ticket, placed(&s.sizes, s.page, spread, area))
        };
        if zoom <= 1.0 {
            return;
        }
        let visible = graphene::Rect::new(-pan.0 / zoom, -pan.1 / zoom, area.0 / zoom, area.1 / zoom);
        let px = scale * zoom;
        for (page, rect) in pages {
            let Some(part) = rect.intersection(&visible) else { continue };
            let full = ((rect.width() * px).round() as i32, (rect.height() * px).round() as i32);
            let origin = (((part.x() - rect.x()) * px).round() as i32, ((part.y() - rect.y()) * px).round() as i32);
            let size = (((part.width() * px).round() as i32).max(1), ((part.height() * px).round() as i32).max(1));
            let job = Part { page, full, origin, size };
            let weak = self.downgrade();
            glib::spawn_future_local(async move {
                let pixels = ask(move |engine| render(engine, doc, &job)).await.flatten();
                let (Some(view), Some((bytes, stride))) = (weak.upgrade(), pixels) else { return };
                let mut s = view.imp().s.borrow_mut();
                if s.sharp_ticket == ticket {
                    s.sharp.push((part, texture(bytes, size, stride)));
                    drop(s);
                    view.queue_draw();
                }
            });
        }
    }

    /// The link under a point of the view, if any.
    async fn link_at(&self, x: f32, y: f32) -> Option<Link> {
        let (doc, zoom, pan, pages) = {
            let s = self.imp().s.borrow();
            (s.doc, s.zoom, s.pan, placed(&s.sizes, s.page, self.spread(), self.area()))
        };
        let (ax, ay) = ((x - pan.0) / zoom, (y - pan.1) / zoom);
        let (page, rect) = pages.into_iter().find(|(_, r)| r.contains_point(&graphene::Point::new(ax, ay)))?;
        let (fx, fy) = ((ax - rect.x()) / rect.width(), (ay - rect.y()) / rect.height());
        ask(move |engine| {
            let pdf_page = engine.docs.get(&doc)?.pages().get(page as _).ok()?;
            let (w, h) = (pdf_page.width().value, pdf_page.height().value);
            let link = pdf_page.links().link_at_point(PdfPoints::new(fx * w), PdfPoints::new((1.0 - fy) * h))?;
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
                    self.turn(1)
                }
            }
            Key::h | Key::Left | Key::KP_Left => {
                if zoomed {
                    self.pan_by(BY, 0.0)
                } else {
                    self.turn(-1)
                }
            }
            Key::space | Key::Page_Down | Key::KP_Page_Down => self.turn(1),
            Key::Page_Up | Key::KP_Page_Up | Key::BackSpace => self.turn(-1),
            Key::bracketright => section(true),
            Key::bracketleft => section(false),
            Key::G | Key::End | Key::KP_End => self.go_to(self.pages().saturating_sub(self.step())),
            Key::g | Key::Home | Key::KP_Home => self.go_to(0),
            Key::plus | Key::equal | Key::KP_Add => self.zoom_by(1.25, middle),
            Key::minus | Key::KP_Subtract => self.zoom_by(0.8, middle),
            Key::_0 | Key::KP_0 => {
                self.reset_zoom();
                self.queue_draw();
            }
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
                    v.turn(if d > 0.0 { 1 } else { -1 });
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
        let menu = Rc::new(PdfMenu { top, bottom, slider, page, scrub: Cell::new(None), view: view.downgrade(), open: Cell::new(false) });
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
        self.slider.set(view.pages(), shown, &[], 0);
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
        Rc::new(PdfPanel { root, list, selection, model, message, field, pages: RefCell::default() })
    }

    fn clear(&self) {
        self.model.splice(0, self.model.n_items(), &[]);
        self.pages.borrow_mut().clear();
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
    doc: u32,
    count: usize,
}

impl Window {
    /// A click on the page: closes what is over it, else the pill, a link, then the side thirds turn and the middle
    /// opens the menu.
    fn click(self: &Rc<Self>, x: f32, y: f32) {
        if self.close_panels() {
            return;
        }
        let view = self.view.clone();
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
                        win.menu.toggle();
                    }
                }
                None if view.imp().s.borrow().zoom == 1.0 => view.turn(if x < view.width() as f32 / 3.0 { -1 } else { 1 }),
                None => {}
            }
        });
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
            let (Some(w), Some(p)) = (w.upgrade(), p.upgrade()) else { return };
            let page = p.pages.borrow().get(row as usize).copied();
            w.close_panels();
            if let Some(page) = page {
                w.view.go_to(page);
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
                    panel.say(Some(tr("This PDF has no contents")));
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
                        let (Some(w), Some(p)) = (w.upgrade(), p.upgrade()) else { return };
                        let page = p.pages.borrow().first().copied();
                        w.close_panels();
                        if let Some(page) = page {
                            w.view.go_to(page);
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
                for (snippet, matched) in forzac_core::search::search_page(&text, &query) {
                    let markup = format!(
                        "{}<b>{}</b>{}",
                        glib::markup_escape_text(&snippet[..matched.start]),
                        glib::markup_escape_text(&snippet[matched.clone()]),
                        glib::markup_escape_text(&snippet[matched.end..])
                    );
                    panel.add(&markup, 0, page);
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

/// The Light theme over a PDF, whatever the reader's theme is: rules for this window's panels after the theme's.
fn light_chrome() {
    thread_local!(static DONE: Cell<bool> = const { Cell::new(false) });
    if DONE.replace(true) {
        return;
    }
    let hex = |c: u32| format!("#{c:06x}");
    let p = THEMES[0];
    let (paper, ink, muted, line, sheet) = (hex(p.paper), hex(p.ink), hex(p.muted), hex(p.line), hex(p.sheet));
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&format!(
        ".pdf-chrome .forzac-panel {{ color: {ink}; }}
         .pdf-chrome .forzac-panel .muted {{ color: {muted}; }}
         .pdf-chrome .reader-bar {{ background-color: {paper}; }}
         .pdf-chrome .bar-top {{ box-shadow: inset 0 -1px {line}; }}
         .pdf-chrome .bar-bottom {{ box-shadow: inset 0 1px {line}; padding-top: 12px; }}
         .pdf-chrome .pdf-panel {{ background-color: {sheet}; padding: 20px 14px 0 14px; box-shadow: 0 0 16px alpha(black, 0.16); }}
         .pdf-chrome .pdf-panel list {{ background: none; color: {ink}; }}
         .pdf-chrome .pdf-panel row {{ padding: 6px; }}
         .pdf-chrome .pdf-panel row:selected {{ background-color: alpha({ink}, 0.07); color: {ink}; }}
         .pdf-chrome .pdf-panel .title {{ font-size: 17px; font-weight: 500; }}
         .pdf-chrome .pdf-panel scrolledwindow {{ background: none; }}
         .pdf-chrome .help-card {{ background-color: {sheet}; box-shadow: 0 0 0 1px {line}, 0 12px 32px alpha(black, 0.16); color: {ink}; }}"
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    }
}

/// Opens the PDF at `path` (`book`, as the core parsed it) in a window of its own.
pub fn open_window(
    app: &gtk::Application,
    path: &Path,
    mut book: Book,
    options: &Options,
    store: &SharedStore,
) -> Result<gtk::ApplicationWindow, Error> {
    let file = path.to_owned();
    // Opening a window waits for this, as parsing a book does.
    let info = call(move |engine| open_doc(engine, &file)).ok_or(Error::Unavailable)??;
    let stateless = options.stateless();
    book.pdf = Some(info.sizes.len());
    if let Some(title) = forzac_core::library::pdf_title(&info.title) {
        book.title = title.to_owned();
    }
    let (at, settings) = if stateless {
        (Locator { block: options.page, ..Locator::default() }, Default::default())
    } else {
        let s = store.borrow();
        (s.position(&book, path).unwrap_or_default(), forzac_core::settings::Settings::load(&s))
    };
    crate::panels::set_theme(settings.theme_index(crate::appearance::dark()));
    light_chrome();
    crate::fonts::add_interface_fonts();
    let view = PdfView::new(&info, at.block, settings);
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
    window.add_css_class("pdf-chrome");
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
    });
    view.connect_input(&win);
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
    let book = Arc::new(book);
    measure(&view, &window, options, &win);
    // The window keeps its panels; the document goes with it.
    let held = RefCell::new(Some(win));
    let doc = info.doc;
    window.connect_destroy(move |_| {
        held.borrow_mut().take();
        drop(gio::spawn_blocking(move || call(move |engine| engine.docs.remove(&doc).is_some())));
    });
    if !stateless {
        remember(&view, &window, book.clone(), path, store);
        remember_cover(&book, path);
        crate::remember_size(&window, store, "window-book");
        crate::keep_awake(app, &window);
    }
    window.present();
    view.grab_focus();
    Ok(window)
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
    let (query, at) = (value("--query"), value("--click"));
    let win = Rc::downgrade(win);
    *view.imp().on_first.borrow_mut() = Some(Box::new(move |view| {
        println!("first page: {:?} after start, {} pages", start.elapsed(), view.pages());
        let (turns, view, window, png) = (options.turns, view.clone(), window.clone(), options.png.clone());
        let left = Cell::new(turns);
        let wait = Cell::new(2);
        let (keys, win) = (keys.clone(), win.clone());
        let next_key = Cell::new(0);
        let (query, at) = (query.clone(), RefCell::new(at.clone()));
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
            if let (Some(at), Some(win)) = (at.borrow_mut().take(), win.upgrade())
                && let Some((x, y)) = at.split_once(',')
            {
                win.click(x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0));
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
