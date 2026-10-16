//! The reading surface: draws pages typeset by the core as GSK text, texture and colour nodes, caches each page as a
//! render node and slides between them on the compositor's frame clock. Draws nothing while idle.
//!
//! The neighbouring chapters are typeset and their images decoded on a worker thread, so a turn across a chapter
//! boundary only draws.
//!
//! Note markers and links of the page can be focused (`Tab`) and opened; jumps keep a back stack.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use forzac_core::hyphen::Hyphenator;
use forzac_core::model::LinkTarget;
use forzac_core::{
    Area, Block, Book, ChapterLayout, Ink, Locator, PageContent, PageStyle, Rect, Target, Typesetter, fonts, layout_note, paginate,
};
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, graphene, gsk, prelude::*};

use crate::fonts::Fonts;

#[path = "scroll.rs"]
mod scroll;

/// How the reader moves through the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A page per window, sliding.
    Slide,
    /// One continuous column.
    Scroll,
}

/// Text size presets in logical pixels: steps, not a slider.
pub const TEXT_SIZES: [f32; 7] = [16.0, 18.0, 20.0, 22.0, 24.0, 27.0, 30.0];

/// Background, ink and link ink of the six themes (docs: the Android app's palettes).
pub const THEMES: [(&str, u32, u32, u32); 6] = [
    ("Light", 0xFAFAFA, 0x1C1C1C, 0x94621F),
    ("Sepia", 0xF4ECD8, 0x5B4636, 0x8A5A1C),
    ("Dark", 0x121212, 0xCFCFCF, 0xC49A52),
    ("Dawn", 0xFAF4ED, 0x575279, 0x8F5E1D),
    ("Wave", 0x1F1F28, 0xDCD7BA, 0xDCA561),
    ("Forest", 0x2D353B, 0xD3C6AA, 0xDBBC7F),
];

/// Opacity of the ink for secondary text (the page number) and for table rules by strength.
const SECONDARY: f32 = 0.6;
const RULES: [f32; 4] = [0.0, 0.35, 0.6, 0.9];

/// Page turn duration, µs.
const TURN: i64 = 200_000;

/// Places remembered for `Ctrl+O`.
const BACK_STACK: usize = 50;

/// Touchpad scrolling (logical pixels) that turns a page; one turn per swipe.
const SWIPE: f64 = 60.0;

/// What opening a note marker or link asks of the window (jumps inside the book the view does itself).
pub enum Open {
    Note(String),
    External(String),
}

fn rgba(rgb: u32) -> gdk::RGBA {
    gdk::RGBA::new(((rgb >> 16) & 0xFF) as f32 / 255.0, ((rgb >> 8) & 0xFF) as f32 / 255.0, (rgb & 0xFF) as f32 / 255.0, 1.0)
}

fn rect(r: Rect) -> graphene::Rect {
    graphene::Rect::new(r.x, r.y, r.w, r.h)
}

/// Ids of the images in `chapters`.
fn image_ids(book: &Book, chapters: impl Iterator<Item = usize>) -> HashSet<String> {
    chapters
        .filter_map(|c| book.chapters.get(c))
        .flat_map(|c| &c.blocks)
        .filter_map(|b| match b {
            Block::Image { id } => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// What the worker thread produces: neighbouring chapters for a style and decoded images.
struct Prefetched {
    style: PageStyle,
    layouts: Vec<ChapterLayout>,
    typesetter: Typesetter,
    textures: Vec<(String, Option<gdk::Texture>)>,
}

/// Opens a note or an external link for the view.
type OnOpen = Box<dyn Fn(Open)>;

/// Runs once, after the first frame with a page is presented.
type FirstFrame = Box<dyn FnOnce(&PageView)>;

struct Turn {
    from: gsk::RenderNode,
    /// +1 forward (the new page comes from the right), −1 back.
    dir: f32,
    /// Frame time of the first frame of the animation; `None` until it is drawn. The frame clock's time at the key
    /// press is stale after a pause, which would make the first frame jump ahead.
    start: Option<i64>,
}

pub struct State {
    book: Arc<Book>,
    hyphenator: Option<Arc<Hyphenator>>,
    fonts: Fonts,
    chapter: usize,
    layout: Option<ChapterLayout>,
    typesetter: Option<Typesetter>,
    /// The chapters before and after the current one, for the current style.
    neighbours: Vec<ChapterLayout>,
    /// The worker's typesetter between jobs (its caches stay warm); `None` while a job runs.
    worker: Option<Typesetter>,
    busy: bool,
    /// Decoded images by id; `None` for one that cannot be decoded.
    textures: HashMap<String, Option<gdk::Texture>>,
    style: Option<PageStyle>,
    page: usize,
    /// The reading position; pages are derived from it after every re-layout.
    locator: Locator,
    to_last: bool,
    size_step: usize,
    theme: usize,
    nodes: HashMap<usize, gsk::RenderNode>,
    /// Note markers and links of the cached pages.
    areas: HashMap<usize, Vec<Area>>,
    /// The focused area of the current page (`Tab`).
    focus: Option<usize>,
    /// Where jumps started, the latest last.
    back: Vec<Locator>,
    turn: Option<Turn>,
    mode: Mode,
    /// "Scroll": the column's offset, tiles and movement.
    column: scroll::Column,
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct PageView {
        pub state: RefCell<Option<State>>,
        /// Frame times (µs) during page turns and the refresh interval, for `--turns`.
        pub frames: RefCell<Vec<Vec<i64>>>,
        pub refresh: std::cell::Cell<i64>,
        pub on_first_frame: RefCell<Option<FirstFrame>>,
        /// Called when the reading position changes.
        pub on_moved: RefCell<Option<Box<dyn Fn()>>>,
        /// Called to open a note or an external link.
        pub on_open: RefCell<Option<OnOpen>>,
        /// Scrolling not yet turned into a page turn, and whether this touchpad swipe already turned one.
        pub scroll: std::cell::Cell<(f64, bool)>,
        /// When the page last changed, so background work can wait for a pause in reading.
        pub last_move: std::cell::Cell<Option<std::time::Instant>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PageView {
        const NAME: &'static str = "ForzacPageView";
        type Type = super::PageView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for PageView {}

    impl WidgetImpl for PageView {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let view = self.obj();
            if view.relayout() {
                view.prefetch();
            }
            view.draw(snapshot);
            if let Some(callback) = self.on_first_frame.borrow_mut().take() {
                // After this frame is presented.
                let view = view.clone();
                let clock = WidgetExt::frame_clock(&view).unwrap();
                let handler = Rc::new(RefCell::new(None));
                let h = handler.clone();
                let callback = RefCell::new(Some(callback));
                *handler.borrow_mut() = Some(clock.connect_after_paint(move |clock| {
                    if let Some(id) = h.borrow_mut().take() {
                        clock.disconnect(id);
                    }
                    if let Some(callback) = callback.borrow_mut().take() {
                        callback(&view);
                    }
                }));
            }
        }
    }
}

glib::wrapper! {
    pub struct PageView(ObjectSubclass<imp::PageView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PageView {
    /// A view of `book` at `at`, in theme `theme` (index in `THEMES`) and text size `size_step` (index in `TEXT_SIZES`).
    pub fn new(book: Book, at: Locator, theme: usize, size_step: usize) -> Self {
        let view: Self = glib::Object::new();
        let hyphenator = Hyphenator::for_language(book.language.as_deref()).map(Arc::new);
        let chapter = at.chapter.min(book.chapters.len().saturating_sub(1));
        *view.imp().state.borrow_mut() = Some(State {
            book: Arc::new(book),
            hyphenator,
            fonts: Fonts::new(),
            chapter,
            layout: None,
            typesetter: None,
            neighbours: Vec::new(),
            worker: None,
            busy: false,
            textures: HashMap::new(),
            style: None,
            page: 0,
            locator: Locator { chapter, ..at },
            to_last: false,
            size_step: size_step.min(TEXT_SIZES.len() - 1),
            theme: theme.min(THEMES.len() - 1),
            nodes: HashMap::new(),
            areas: HashMap::new(),
            focus: None,
            back: Vec::new(),
            turn: None,
            mode: Mode::Slide,
            column: scroll::Column::default(),
        });
        view.set_hexpand(true);
        view.set_vexpand(true);
        view.set_focusable(true);
        view.set_can_focus(true);
        view.imp().last_move.set(Some(std::time::Instant::now())); // opening counts as a move

        let click = gtk::GestureClick::new();
        click.connect_released(glib::clone!(
            #[weak]
            view,
            move |_, _, x, y| view.click(x as f32, y as f32)
        ));
        view.add_controller(click);

        // "Slide": a wheel step turns a page, a touchpad swipe at most one. "Scroll": the wheel scrolls three lines a
        // step, smoothly; the touchpad moves the column and glides on after a flick.
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::KINETIC);
        scroll.connect_decelerate(glib::clone!(
            #[weak]
            view,
            move |_, _, vy| {
                if view.mode() == Mode::Scroll {
                    view.glide(vy as f32);
                }
            }
        ));
        scroll.connect_scroll_begin(glib::clone!(
            #[weak]
            view,
            move |_| view.imp().scroll.set((0.0, false))
        ));
        scroll.connect_scroll_end(glib::clone!(
            #[weak]
            view,
            move |_| view.imp().scroll.set((0.0, false))
        ));
        scroll.connect_scroll(glib::clone!(
            #[weak]
            view,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |c, _, dy| {
                if view.mode() == Mode::Scroll {
                    if c.unit() == gdk::ScrollUnit::Wheel {
                        view.scroll_by(dy as f32 * 3.0 * view.line_height());
                    } else {
                        view.scroll_pixels(dy as f32);
                    }
                    return glib::Propagation::Stop;
                }
                let (mut acc, mut turned) = view.imp().scroll.get();
                acc += dy;
                if c.unit() == gdk::ScrollUnit::Wheel {
                    while acc.abs() >= 1.0 {
                        view.turn(acc.signum() as i32);
                        acc -= acc.signum();
                    }
                } else if !turned && acc.abs() >= SWIPE {
                    view.turn(acc.signum() as i32);
                    turned = true;
                }
                view.imp().scroll.set((acc, turned));
                glib::Propagation::Stop
            }
        ));
        view.add_controller(scroll);
        view
    }

    /// Calls `f` to open a note or an external link.
    pub fn connect_open(&self, f: impl Fn(Open) + 'static) {
        *self.imp().on_open.borrow_mut() = Some(Box::new(f));
    }

    /// The contents: (title, level, place) of every titled section, or of every chapter when the book has none.
    pub fn contents(&self) -> Vec<(String, u32, Locator)> {
        self.with(|s| {
            if s.book.toc.is_empty() {
                s.book
                    .chapters
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| Some((c.title.clone()?, 1, Locator { chapter: i, ..Locator::default() })))
                    .collect()
            } else {
                s.book.toc.iter().map(|e| (e.title.clone(), e.level, e.target)).collect()
            }
        })
    }

    /// Note `id` typeset at `width` for the note panel: its node and height, in the current theme and text size.
    pub fn note(&self, id: &str, width: f32) -> Option<(gsk::RenderNode, f32)> {
        self.with(|s| {
            let text_size = TEXT_SIZES[s.size_step] * 0.9;
            let base = PageStyle::for_window(width, 100_000.0, text_size);
            let style = PageStyle { margin_h: 28.0, margin_top: 18.0, margin_bottom: 18.0, ..base };
            let mut typesetter = Typesetter::new(style, s.hyphenator.clone());
            let layout = layout_note(&s.book, id, &style, &mut typesetter)?;
            let content = layout.render_body(0, &style);
            let node = content_node(&content, s.theme, &mut s.fonts, &mut s.textures, &s.book)?;
            Some((node, style.margin_top + layout.height(0) + style.margin_bottom))
        })
    }

    /// `Tab` (`delta` 1) and `Shift+Tab` (−1): focus the next or previous note marker or link of the page.
    pub fn focus_next(&self, delta: i32) {
        let h = WidgetExt::height(self) as f32;
        self.with(|s| {
            let count = if s.mode == Mode::Scroll {
                stops(Self::visible_areas(s, h)).len() as i32
            } else {
                let Some(page) = Self::page_node(s, s.page).map(|_| s.page) else { return };
                s.areas.get(&page).map_or(0, |a| stops(a).len()) as i32
            };
            if count == 0 {
                return;
            }
            s.focus = Some(match s.focus {
                Some(f) => (f as i32 + delta).rem_euclid(count) as usize,
                None if delta > 0 => 0,
                None => count as usize - 1,
            });
        });
        self.queue_draw();
    }

    /// Removes the focus; false when nothing was focused.
    pub fn unfocus(&self) -> bool {
        let had = self.with(|s| s.focus.take().is_some());
        self.queue_draw();
        had
    }

    /// Opens the focused note marker or link; false when nothing is focused.
    pub fn open_focused(&self) -> bool {
        let h = WidgetExt::height(self) as f32;
        let target = self.with(|s| {
            let focus = s.focus?;
            let areas = if s.mode == Mode::Scroll { Self::visible_areas(s, h) } else { s.areas.get(&s.page)? };
            let (first, _) = *stops(areas).get(focus)?;
            Some(areas[first].target.clone())
        });
        match target {
            Some(t) => {
                self.open(t);
                true
            }
            None => false,
        }
    }

    fn click(&self, x: f32, y: f32) {
        let h = WidgetExt::height(self) as f32;
        let target = self.with(|s| {
            let areas = if s.mode == Mode::Scroll { Self::visible_areas(s, h) } else { s.areas.get(&s.page)? };
            // A little slack: markers are small.
            let hit =
                |a: &&Area| x >= a.rect.x - 4.0 && x <= a.rect.x + a.rect.w + 4.0 && y >= a.rect.y - 4.0 && y <= a.rect.y + a.rect.h + 4.0;
            areas.iter().find(hit).map(|a| a.target.clone())
        });
        if let Some(t) = target {
            self.open(t);
        }
    }

    fn open(&self, target: Target) {
        let action = self.with(|s| match target {
            Target::Note(id) => Some(Err(Open::Note(id))),
            Target::Link(key) => match s.book.links.get(&key)? {
                LinkTarget::Place { start, .. } => Some(Ok(*start)),
                LinkTarget::Note(id) => Some(Err(Open::Note(id.clone()))),
                LinkTarget::External(url) => Some(Err(Open::External(url.clone()))),
            },
        });
        match action {
            Some(Ok(at)) => self.go_to(at),
            Some(Err(open)) => {
                if let Some(f) = self.imp().on_open.borrow().as_ref() {
                    f(open);
                }
            }
            None => {}
        }
    }

    /// Jumps to `at`, remembering where the jump started (`back` returns there).
    pub fn go_to(&self, at: Locator) {
        self.with(|s| {
            s.back.push(s.locator);
            if s.back.len() > BACK_STACK {
                s.back.remove(0);
            }
        });
        self.show(at);
    }

    /// Returns to where the last jump started; false when there is none.
    pub fn back(&self) -> bool {
        let Some(at) = self.with(|s| s.back.pop()) else { return false };
        self.show(at);
        true
    }

    /// Shows the page with `at`, without the slide.
    fn show(&self, at: Locator) {
        self.with(|s| {
            let at = forzac_core::state::clamp(&s.book, at);
            s.chapter = at.chapter;
            s.locator = at;
            s.turn = None;
            s.focus = None;
            if let Some(layout) = s.layout.as_ref().filter(|l| l.chapter == at.chapter) {
                s.page = layout.page_of(at);
            }
            if s.mode == Mode::Scroll {
                Self::align_column(s);
            }
        });
        self.moved();
        self.queue_draw();
    }

    /// The body text's line height.
    fn line_height(&self) -> f32 {
        self.with(|s| s.typesetter.as_ref().map_or(30.0, |t| t.line_height))
    }

    /// Calls `f` whenever the reading position changes (to save it).
    pub fn connect_moved(&self, f: impl Fn() + 'static) {
        *self.imp().on_moved.borrow_mut() = Some(Box::new(f));
    }

    /// How long since the page last changed (since the view was created, if it has not).
    pub fn quiet_for(&self) -> std::time::Duration {
        self.imp().last_move.get().map_or(std::time::Duration::MAX, |t| t.elapsed())
    }

    fn moved(&self) {
        self.imp().last_move.set(Some(std::time::Instant::now()));
        if let Some(f) = self.imp().on_moved.borrow().as_ref() {
            f();
        }
    }

    /// The reading position: where the current page starts.
    pub fn locator(&self) -> Locator {
        self.with(|s| s.locator)
    }

    pub fn book(&self) -> Arc<Book> {
        self.with(|s| s.book.clone())
    }

    /// The theme and text size, as indices in `THEMES` and `TEXT_SIZES`.
    pub fn style(&self) -> (usize, usize) {
        self.with(|s| (s.theme, s.size_step))
    }

    /// Runs once, after the first frame with a page is presented (measurements, `--png`).
    pub fn connect_first_frame(&self, f: impl FnOnce(&PageView) + 'static) {
        *self.imp().on_first_frame.borrow_mut() = Some(Box::new(f));
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(self.imp().state.borrow_mut().as_mut().unwrap())
    }

    /// Typesets the current chapter when the size, text size or chapter changed (or takes it from the neighbours laid
    /// out in the background); keeps the reading position. True when the layout changed.
    fn relayout(&self) -> bool {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        self.with(|s| {
            let style = PageStyle::for_window(w, h, TEXT_SIZES[s.size_step]);
            let fresh = s.layout.as_ref().is_some_and(|l| l.chapter == s.chapter) && s.style == Some(style);
            if fresh {
                return false;
            }
            if s.style != Some(style) {
                s.typesetter = Some(Typesetter::new(style, s.hyphenator.clone()));
                s.style = Some(style);
                s.neighbours.clear();
                s.layout = None;
            }
            let layout = match s.neighbours.iter().position(|l| l.chapter == s.chapter) {
                Some(i) => s.neighbours.swap_remove(i),
                None => paginate(&s.book, s.chapter, &style, s.typesetter.as_mut().unwrap()),
            };
            if std::mem::take(&mut s.to_last) {
                s.page = layout.pages.len() - 1;
                s.locator = layout.page_start(s.page); // turned back into this chapter: its last page
            } else {
                s.page = layout.page_of(s.locator);
            }
            if let Some(old) = s.layout.replace(layout) {
                s.neighbours.push(old);
            }
            s.nodes.clear();
            s.areas.clear();
            s.focus = None;
            if s.mode == Mode::Scroll {
                s.column.tiles.clear();
                Self::align_column(s);
            }
            true
        })
    }

    /// Typesets the chapters next to the current one and decodes their images on a worker thread, one job at a time.
    fn prefetch(&self) {
        let job = self.with(|s| {
            let style = s.style?;
            let chapter = s.chapter;
            // "Scroll" shows the next chapter while the current one is still on screen: one more ahead.
            let ahead = if s.mode == Mode::Scroll { 2 } else { 1 };
            // Layouts no longer needed are freed on a worker: thousands of small allocations, not a frame's work.
            let (keep, old): (Vec<_>, Vec<_>) = s.neighbours.drain(..).partition(|l| (1..=ahead).contains(&l.chapter.abs_diff(chapter)));
            s.neighbours = keep;
            if !old.is_empty() {
                std::thread::spawn(move || drop(old));
            }
            let near = || (chapter.saturating_sub(1)..=chapter + 1).filter(|&c| c < s.book.chapters.len());
            let wanted: HashSet<String> = image_ids(&s.book, near());
            s.textures.retain(|id, _| wanted.contains(id));
            if s.busy {
                return None;
            }
            let chapters: Vec<usize> = [chapter + 1, chapter.wrapping_sub(1), if ahead > 1 { chapter + 2 } else { usize::MAX }]
                .into_iter()
                .filter(|&c| c < s.book.chapters.len() && !s.neighbours.iter().any(|l| l.chapter == c))
                .collect();
            let images: Vec<String> = wanted.into_iter().filter(|id| !s.textures.contains_key(id)).collect();
            if chapters.is_empty() && images.is_empty() {
                return None;
            }
            let typesetter = s.worker.take().unwrap_or_else(|| Typesetter::new(style, s.hyphenator.clone()));
            s.busy = true;
            Some((s.book.clone(), style, chapters, images, typesetter))
        });
        let Some((book, style, chapters, images, mut typesetter)) = job else { return };
        let view = self.downgrade();
        glib::spawn_future_local(async move {
            let done = gio::spawn_blocking(move || {
                if typesetter.style() != &style {
                    typesetter = Typesetter::new(style, typesetter.hyphenator());
                }
                let layouts = chapters.iter().map(|&c| paginate(&book, c, &style, &mut typesetter)).collect();
                let textures = images
                    .into_iter()
                    .map(|id| {
                        let texture = book.images.get(&id).and_then(|i| gdk::Texture::from_bytes(&glib::Bytes::from(&i.bytes[..])).ok());
                        (id, texture)
                    })
                    .collect();
                Prefetched { style, layouts, typesetter, textures }
            })
            .await;
            let (Some(view), Ok(done)) = (view.upgrade(), done) else { return };
            view.with(|s| {
                s.busy = false;
                s.textures.extend(done.textures);
                if s.style == Some(done.style) {
                    let current = s.layout.as_ref().map(|l| l.chapter);
                    s.neighbours.extend(done.layouts.into_iter().filter(|l| Some(l.chapter) != current));
                }
                s.worker = Some(done.typesetter);
            });
            view.prefetch();
        });
    }

    fn page_node(s: &mut State, page: usize) -> Option<gsk::RenderNode> {
        if let Some(n) = s.nodes.get(&page) {
            return Some(n.clone());
        }
        let (layout, style, t) = (s.layout.as_ref()?, s.style?, s.typesetter.as_mut()?);
        let content = layout.render(page, &style, t);
        let node = content_node(&content, s.theme, &mut s.fonts, &mut s.textures, &s.book)?;
        s.nodes.insert(page, node.clone());
        s.areas.insert(page, content.areas);
        Some(node)
    }

    fn draw(&self, snapshot: &gtk::Snapshot) {
        self.draw_page(snapshot);
        if self.mode() == Mode::Scroll && self.with(Self::placed) {
            self.prepare();
        }
    }

    fn draw_page(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        self.with(|s| {
            snapshot.append_color(&rgba(THEMES[s.theme].1), &graphene::Rect::new(0.0, 0.0, w, h));
            if s.mode == Mode::Scroll {
                Self::draw_column(s, snapshot, h);
                return;
            }
            let Some(current) = Self::page_node(s, s.page) else { return };
            let progress: f32 =
                s.turn.as_mut().map(|t| ((now - *t.start.get_or_insert(now)) as f32 / TURN as f32).clamp(0.0, 1.0)).unwrap_or(1.0);
            match &s.turn {
                Some(t) if progress < 1.0 => {
                    let p = 1.0 - (1.0 - progress).powi(3); // ease-out cubic
                    snapshot.save();
                    snapshot.translate(&graphene::Point::new(-t.dir * p * w, 0.0));
                    snapshot.append_node(&t.from);
                    snapshot.restore();
                    snapshot.save();
                    snapshot.translate(&graphene::Point::new(t.dir * (1.0 - p) * w, 0.0));
                    snapshot.append_node(&current);
                    snapshot.restore();
                }
                _ => {
                    s.turn = None;
                    snapshot.append_node(&current);
                    // The focused marker or link (all its lines): a soft box in the link ink, over the cached page.
                    let areas = s.areas.get(&s.page);
                    if let (Some(areas), Some(f)) = (areas, s.focus)
                        && let Some(&(first, last)) = stops(areas).get(f)
                    {
                        let mut ink = rgba(THEMES[s.theme].3);
                        ink.set_alpha(0.22);
                        for area in &areas[first..=last] {
                            let r = area.rect;
                            let bounds = graphene::Rect::new(r.x - 3.0, r.y - 2.0, r.w + 6.0, r.h + 4.0);
                            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(bounds, 4.0));
                            snapshot.append_color(&ink, &bounds);
                            snapshot.pop();
                        }
                    }
                }
            }
        });
    }

    /// The current page as one node, background included (for `--png`).
    pub fn current_page_node(&self) -> Option<gsk::RenderNode> {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        self.with(|s| {
            let page = Self::page_node(s, s.page)?;
            let snapshot = gtk::Snapshot::new();
            snapshot.append_color(&rgba(THEMES[s.theme].1), &graphene::Rect::new(0.0, 0.0, w, h));
            snapshot.append_node(&page);
            snapshot.to_node()
        })
    }

    /// "Slide": turns `delta` pages. "Scroll": moves by a screen less a line.
    pub fn turn(&self, delta: i32) {
        if self.mode() == Mode::Scroll {
            let h = WidgetExt::height(self) as f32;
            self.scroll_by(delta as f32 * (h - self.line_height()));
            return;
        }
        let animate = self.with(|s| {
            let Some(layout) = &s.layout else { return false };
            let from = s.nodes.get(&s.page).cloned();
            let target = s.page as i64 + delta as i64;
            if target >= 0 && (target as usize) < layout.pages.len() {
                s.page = target as usize;
                s.locator = layout.page_start(s.page);
            } else if delta > 0 && s.chapter + 1 < s.book.chapters.len() {
                s.chapter += 1;
                s.locator = Locator { chapter: s.chapter, ..Locator::default() };
            } else if delta < 0 && s.chapter > 0 {
                s.chapter -= 1;
                s.to_last = true;
                s.locator = Locator { chapter: s.chapter, ..Locator::default() };
            } else {
                return false;
            }
            s.turn = from.map(|from| Turn { from, dir: delta.signum() as f32, start: None });
            s.focus = None;
            true
        });
        if animate {
            self.moved();
            self.imp().frames.borrow_mut().push(Vec::new());
            self.add_tick_callback(move |view, clock| {
                view.queue_draw();
                if let Some(turn) = view.imp().frames.borrow_mut().last_mut() {
                    turn.push(clock.frame_time());
                }
                if let Some(t) = clock.current_timings() {
                    view.imp().refresh.set(t.refresh_interval());
                }
                let done = view.with(|s| s.turn.as_ref().is_none_or(|t| t.start.is_some_and(|start| clock.frame_time() - start >= TURN)));
                if done { glib::ControlFlow::Break } else { glib::ControlFlow::Continue }
            });
        }
        self.queue_draw();
    }

    /// "Slide": turns a page. "Scroll": moves by a line.
    pub fn line(&self, delta: i32) {
        if self.mode() == Mode::Scroll {
            self.scroll_by(delta as f32 * self.line_height());
        } else {
            self.turn(delta);
        }
    }

    /// "Slide": turns a page. "Scroll": moves by half a screen.
    pub fn half(&self, delta: i32) {
        if self.mode() == Mode::Scroll {
            self.scroll_by(delta as f32 * WidgetExt::height(self) as f32 / 2.0);
        } else {
            self.turn(delta);
        }
    }

    pub fn chapter(&self, delta: i32) {
        self.with(|s| {
            let target = (s.chapter as i64 + delta as i64).clamp(0, s.book.chapters.len() as i64 - 1) as usize;
            s.chapter = target;
            s.locator = Locator { chapter: target, ..Locator::default() };
            s.turn = None;
            s.focus = None;
            if s.mode == Mode::Scroll {
                Self::align_column(s);
            }
        });
        self.moved();
        self.queue_draw();
    }

    pub fn chapter_edge(&self, end: bool) {
        if self.mode() == Mode::Scroll {
            self.column_edge(end);
            return;
        }
        self.with(|s| {
            if let Some(layout) = &s.layout {
                s.page = if end { layout.pages.len() - 1 } else { 0 };
                s.locator = layout.page_start(s.page);
                s.turn = None;
                s.focus = None;
            }
        });
        self.moved();
        self.queue_draw();
    }

    /// Page `page` of the current chapter (the last one if there are fewer); in "Scroll" its first line at the top.
    pub fn go_to_page(&self, page: usize) {
        self.with(|s| {
            if let Some(layout) = &s.layout {
                s.page = page.min(layout.pages.len() - 1);
                s.locator = layout.page_start(s.page);
                s.turn = None;
            }
            if s.mode == Mode::Scroll {
                Self::align_column(s);
            }
        });
        self.queue_draw();
    }

    pub fn text_size(&self, delta: i32) {
        self.with(|s| s.size_step = (s.size_step as i32 + delta).clamp(0, TEXT_SIZES.len() as i32 - 1) as usize);
        self.queue_draw();
    }

    pub fn next_theme(&self, delta: i32) {
        self.with(|s| {
            s.theme = (s.theme as i32 + delta).rem_euclid(THEMES.len() as i32) as usize;
            s.nodes.clear(); // ink is baked into the nodes; only a redraw, no re-layout
            s.areas.clear();
            s.column.tiles.clear();
        });
        self.queue_draw();
    }

    /// Frame intervals of the page turns so far: (refresh interval µs, intervals µs).
    /// Frame times of each page turn, µs.
    pub fn turn_frames(&self) -> Vec<Vec<i64>> {
        self.imp().frames.borrow().clone()
    }

    pub fn turn_stats(&self) -> (i64, Vec<i64>) {
        let intervals = self.imp().frames.borrow().iter().flat_map(|t| t.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>()).collect();
        (self.imp().refresh.get(), intervals)
    }

    pub fn page_count(&self) -> usize {
        self.with(|s| s.layout.as_ref().map_or(0, |l| l.pages.len()))
    }
}

/// `Tab` stops: runs of consecutive areas with the same target (a link broken across lines is one stop), as index
/// ranges `(first, last)`.
fn stops(areas: &[Area]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, area) in areas.iter().enumerate() {
        match out.last_mut() {
            Some(stop) if areas[stop.1].target == area.target => stop.1 = i,
            _ => out.push((i, i)),
        }
    }
    out
}

/// The render node of a page's (or a note's) contents in `theme`: images, table rules and glyph runs.
fn content_node(
    content: &PageContent,
    theme: usize,
    fonts: &mut Fonts,
    textures: &mut HashMap<String, Option<gdk::Texture>>,
    book: &Book,
) -> Option<gsk::RenderNode> {
    let (_, _, text, link) = THEMES[theme];
    let mut secondary = rgba(text);
    secondary.set_alpha(SECONDARY);
    let snapshot = gtk::Snapshot::new();
    for image in &content.images {
        // Decoded on the worker; an image the worker has not reached yet (the first page) is decoded here.
        let texture = textures
            .entry(image.id.clone())
            .or_insert_with(|| book.images.get(&image.id).and_then(|i| gdk::Texture::from_bytes(&glib::Bytes::from(&i.bytes[..])).ok()));
        if let Some(texture) = texture {
            snapshot.append_scaled_texture(texture, gsk::ScalingFilter::Trilinear, &rect(image.rect));
        }
    }
    for rule in &content.rules {
        let mut ink = rgba(text);
        ink.set_alpha(RULES[rule.strength as usize]);
        snapshot.append_color(&ink, &rect(rule.rect));
    }
    for run in &content.runs {
        let ink = match run.ink {
            Ink::Text => rgba(text),
            Ink::Link => rgba(link),
            Ink::Secondary => secondary,
        };
        let color = fonts::font_is_color(run.font);
        let Some(&[x0, y0]) = run.positions.first() else { continue };
        let mut glyphs = gtk::pango::GlyphString::new();
        glyphs.set_size(run.glyphs.len() as i32);
        let units: Vec<i32> = run.positions.iter().map(|p| ((p[0] - x0) * 1024.0).round() as i32).collect();
        for (i, info) in glyphs.glyph_info_mut().iter_mut().enumerate() {
            info.set_glyph(run.glyphs[i] as u32);
            let g = info.geometry_mut();
            g.set_width(units.get(i + 1).map_or(0, |next| next - units[i]));
            g.set_x_offset(0);
            g.set_y_offset(((run.positions[i][1] - y0) * 1024.0).round() as i32);
            // `set_size` leaves the new entries uninitialised; a stray `is_color` bit makes GSK draw a glyph as a
            // colour glyph (invisible for a plain font). The bindings have no setter, so set the whole bitfield:
            // bit 0 cluster start, bit 1 colour (emoji).
            let attr = 1 | (color as u32) << 1;
            unsafe { (*(info as *mut gtk::pango::GlyphInfo).cast::<gtk::pango::ffi::PangoGlyphInfo>()).attr.is_cluster_start = attr };
        }
        let Some(font) = fonts.get(run.font, run.size) else { continue };
        if let Some(node) = gsk::TextNode::new(font, &glyphs, &ink, &graphene::Point::new(x0, y0)) {
            snapshot.append_node(node);
        }
    }
    snapshot.to_node()
}
