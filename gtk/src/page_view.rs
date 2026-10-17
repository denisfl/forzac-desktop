//! The reading surface: draws pages typeset by the core as GSK text, texture and colour nodes, caches each page as a
//! render node and slides between them on the compositor's frame clock. Draws nothing while idle.
//!
//! The neighbouring chapters are typeset and their images decoded on a worker thread, so a turn across a chapter
//! boundary only draws.
//!
//! Note markers, links, wide images and tables of the page can be focused (`Tab`) and opened; jumps keep a back stack.
//!
//! A marked page (or screen) shows a bookmark ribbon over it, which drops in when the reader sets the bookmark.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use forzac_core::hyphen::Hyphenator;
use forzac_core::model::LinkTarget;
use forzac_core::search::Hit;
use forzac_core::settings::{Setting, Settings, Turning};
use forzac_core::{
    Area, Block, Book, ChapterLayout, Ink, Locator, PageContent, PageStyle, Rect, Target, Typesetter, end_page, fonts, layout_note,
    layout_preview, layout_table, paginate,
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

/// A theme's colours (the Android app's `docs/design.md`): the page's paper, ink and link ink; the panels' secondary
/// text, lines and background.
#[derive(Clone, Copy)]
pub struct Palette {
    pub paper: u32,
    pub ink: u32,
    pub muted: u32,
    pub line: u32,
    pub sheet: u32,
    pub link: u32,
}

/// The six themes, in the order of `settings::THEMES`.
pub const THEMES: [Palette; 6] = [
    Palette { paper: 0xFAFAFA, ink: 0x1C1C1C, muted: 0x6E6E6E, line: 0xE5E5E5, sheet: 0xFFFFFF, link: 0x94621F },
    Palette { paper: 0xF4ECD8, ink: 0x5B4636, muted: 0x7A6650, line: 0xE2D6BC, sheet: 0xFBF5E8, link: 0x8A5A1C },
    Palette { paper: 0x121212, ink: 0xCFCFCF, muted: 0x8E8E8E, line: 0x333333, sheet: 0x1E1E1E, link: 0xC49A52 },
    Palette { paper: 0xFAF4ED, ink: 0x575279, muted: 0x6E6A86, line: 0xE6DDD6, sheet: 0xFFFAF3, link: 0x8F5E1D },
    Palette { paper: 0x1F1F28, ink: 0xDCD7BA, muted: 0x9A9585, line: 0x363646, sheet: 0x2A2A37, link: 0xDCA561 },
    Palette { paper: 0x2D353B, ink: 0xD3C6AA, muted: 0x9DA9A0, line: 0x475258, sheet: 0x343F44, link: 0xDBBC7F },
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

/// Theme cross-fade and "← Back" pill fade durations, µs.
const THEME_FADE: i64 = 200_000;
const PILL_FADE: i64 = 150_000;

/// The pill after a jump, as on Android: it goes when the reader turns this many pages (or screens) away.
const PILL_PAGES: i32 = 2;
const PILL_TEXT: &str = "← Back · Ctrl+O";

/// The bookmark ribbon (Android's): its width, the length below the window's top edge, the part tucked above it, the
/// swallowtail's depth; bordeaux on light and on dark themes.
const RIBBON_WIDTH: f32 = 16.0;
const RIBBON_LENGTH: f32 = 44.0;
const RIBBON_TUCK: f32 = 12.0;
const RIBBON_TAIL: f32 = RIBBON_WIDTH * 0.5625;
const RIBBON_LIGHT: u32 = 0x8E2F2A;
const RIBBON_DARK: u32 = 0xB5483F;
/// The drop's spring (Android's: damping ratio 0.5, stiffness 280 s⁻²) and the rise's duration, µs.
const RIBBON_DAMPING: f32 = 0.5;
const RIBBON_STIFFNESS: f32 = 280.0;
const RIBBON_RISE: i64 = 200_000;

/// What opening a note marker, a link, an image or a table asks of the window (jumps inside the book the view does
/// itself).
pub enum Open {
    Note(String),
    /// A link's short target, shown in the note sheet (`Book::preview`).
    Preview(String),
    External(String),
    /// An image (by id) or a table (by its first row) for the view (`viewer`).
    Image(String),
    Table(Locator),
}

pub(crate) fn rgba(rgb: u32) -> gdk::RGBA {
    gdk::RGBA::new(((rgb >> 16) & 0xFF) as f32 / 255.0, ((rgb >> 8) & 0xFF) as f32 / 255.0, (rgb & 0xFF) as f32 / 255.0, 1.0)
}

fn rect(r: Rect) -> graphene::Rect {
    graphene::Rect::new(r.x, r.y, r.w, r.h)
}

/// Image `id` of `book` decoded; `None` when it cannot be.
fn decode(book: &Book, id: &str) -> Option<gdk::Texture> {
    book.images.get(id).and_then(|i| gdk::Texture::from_bytes(&glib::Bytes::from(&i.bytes[..])).ok())
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

/// Saves the bookmarks after they change.
type OnBookmarks = Box<dyn Fn(&[Locator])>;

/// Runs once, after the first frame with a page is presented.
type FirstFrame = Box<dyn FnOnce(&PageView)>;

/// Where the last jump arrived, for the "← Back" pill, and the pages turned since ("Slide").
struct Pill {
    arrived: Locator,
    turns: i32,
}

/// The ribbon dropping in (a bookmark set) or going up (removed) on the page or screen `range`; `start` is the frame
/// time of its first frame.
struct Ribbon {
    drop: bool,
    range: (Locator, Locator),
    start: Option<i64>,
}

struct Turn {
    from: gsk::RenderNode,
    /// Whether the page sliding away has a bookmark: its ribbon slides with it.
    from_marked: bool,
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
    /// The reader's settings; the page mode is `mode`.
    settings: Settings,
    /// Whether the desktop prefers a dark style, for a theme that follows it.
    system_dark: bool,
    /// The theme shown, an index in `THEMES`.
    theme: usize,
    nodes: HashMap<usize, gsk::RenderNode>,
    /// Note markers and links of the cached pages.
    areas: HashMap<usize, Vec<Area>>,
    /// The focused area of the current page (`Tab`).
    focus: Option<usize>,
    /// Where jumps started, the latest last.
    back: Vec<Locator>,
    /// The "← Back" pill after a jump; `None` once the reader returned or read on.
    pill: Option<Pill>,
    /// Whether the pill is shown, and the frame time its fade in or out started.
    pill_shown: bool,
    pill_change: Option<i64>,
    /// Where the pill was drawn, for clicks.
    pill_rect: Option<graphene::Rect>,
    /// "Slide": the end page after the book's last page is shown.
    at_end: bool,
    /// "Scroll": the end screen after the last paragraph fills the window.
    column_end: bool,
    /// The end page for the current style and theme.
    end: Option<gsk::RenderNode>,
    /// The frame before a theme change, cross-fading into the new colours, and the fade's first frame time.
    fade: Option<(gsk::RenderNode, Option<i64>)>,
    /// The matches of the last search gone to, and the one highlighted (`n`/`N` move it; `None`: no highlight).
    search: Option<(Rc<Vec<Hit>>, Option<usize>)>,
    /// The book's bookmarks in reading order, and the ribbon's movement after one was set or removed.
    bookmarks: Vec<Locator>,
    ribbon: Option<Ribbon>,
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
        /// Frame times (µs) during page turns (and scrolls, ribbon drops) and the refresh interval, for `--turns`.
        pub frames: RefCell<Vec<Vec<i64>>>,
        pub refresh: std::cell::Cell<i64>,
        pub on_first_frame: RefCell<Option<FirstFrame>>,
        /// Called when the reading position changes.
        pub on_moved: RefCell<Option<Box<dyn Fn()>>>,
        /// Called to open a note or an external link.
        pub on_open: RefCell<Option<OnOpen>>,
        /// Called with the bookmarks after they change (to save them).
        pub on_bookmarks: RefCell<Option<OnBookmarks>>,
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
    /// A view of `book` at `at` with `settings`, on a desktop that prefers a dark style or not.
    pub fn new(book: Book, at: Locator, settings: Settings, system_dark: bool) -> Self {
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
            settings,
            system_dark,
            theme: settings.theme_index(system_dark),
            nodes: HashMap::new(),
            areas: HashMap::new(),
            focus: None,
            back: Vec::new(),
            pill: None,
            pill_shown: false,
            pill_change: None,
            pill_rect: None,
            at_end: false,
            column_end: false,
            end: None,
            fade: None,
            search: None,
            bookmarks: Vec::new(),
            ribbon: None,
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
        self.with(|s| s.book.contents().into_iter().map(|e| (e.title, e.level, e.target)).collect())
    }

    /// Note `id` typeset at `width` for the note panel: its node and height, in the current theme and text size.
    pub fn note(&self, id: &str, width: f32) -> Option<(gsk::RenderNode, f32)> {
        self.with(|s| {
            let style = s.settings.note_style(width);
            let mut typesetter = Typesetter::new(style, s.hyphenator.clone());
            let layout = layout_note(&s.book, id, &style, &mut typesetter)?;
            let content = layout.render_body(0, &style);
            let node = content_node(&content, s.theme, &mut s.fonts, &mut s.textures, &s.book)?;
            Some((node, style.margin_top + layout.height(0) + style.margin_bottom))
        })
    }

    /// The short target of link `key` typeset for the note sheet `width` wide, as `note` does a note.
    pub fn preview(&self, key: &str, width: f32) -> Option<(gsk::RenderNode, f32)> {
        self.with(|s| {
            let style = s.settings.note_style(width);
            let mut typesetter = Typesetter::new(style, s.hyphenator.clone());
            let layout = layout_preview(&s.book, key, &style, &mut typesetter)?;
            let content = layout.render_body(0, &style);
            let node = content_node(&content, s.theme, &mut s.fonts, &mut s.textures, &s.book)?;
            Some((node, style.margin_top + layout.height(0) + style.margin_bottom))
        })
    }

    /// `Tab` (`delta` 1) and `Shift+Tab` (−1): focus the next or previous note marker, link, image or table of the page.
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

    /// Opens the focused note marker, link, image or table; false when nothing is focused.
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
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        if self.with(|s| s.pill_shown && s.pill_rect.is_some_and(|r| r.contains_point(&graphene::Point::new(x, y)))) {
            self.back();
            return;
        }
        let target = self.with(|s| {
            let areas = if s.mode == Mode::Scroll { Self::visible_areas(s, h) } else { s.areas.get(&s.page)? };
            // A little slack: markers are small. A marker or link in a table goes before the table.
            let hit =
                |a: &&Area| x >= a.rect.x - 4.0 && x <= a.rect.x + a.rect.w + 4.0 && y >= a.rect.y - 4.0 && y <= a.rect.y + a.rect.h + 4.0;
            areas.iter().filter(hit).min_by_key(|a| a.target.is_viewable()).map(|a| a.target.clone())
        });
        let corner = self.with(|s| s.style.is_some_and(|st| y < st.margin_top && x >= st.ribbon().1));
        if let Some(t) = target {
            self.open(t);
        } else if corner {
            self.toggle_bookmark();
        } else if self.mode() == Mode::Slide {
            // The side thirds turn the page, as taps do on Android; the middle does nothing.
            if x < w / 3.0 {
                self.turn(-1);
            } else if x > w * 2.0 / 3.0 {
                self.turn(1);
            }
        }
    }

    /// Opens a note marker, a link, an image or a table: a jump inside the book here, the rest through `connect_open`.
    pub fn open(&self, target: Target) {
        let action = self.with(|s| match target {
            Target::Note(id) => Some(Err(Open::Note(id))),
            Target::Image(id) => Some(Err(Open::Image(id))),
            Target::Table(at) => Some(Err(Open::Table(at))),
            Target::Link(key) => match s.book.links.get(&key)? {
                LinkTarget::Place { .. } if s.book.preview(&key).is_some() => Some(Err(Open::Preview(key))),
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

    /// Whether opening `target` jumps to another place in the book, rather than showing something over the page.
    pub fn jumps(&self, target: &Target) -> bool {
        self.with(|s| match target {
            Target::Link(key) => matches!(s.book.links.get(key), Some(LinkTarget::Place { .. })) && s.book.preview(key).is_none(),
            _ => false,
        })
    }

    /// Image `id` decoded, from the pages' cache when a page has shown it.
    pub fn texture(&self, id: &str) -> Option<gdk::Texture> {
        self.with(|s| s.textures.entry(id.to_owned()).or_insert_with(|| decode(&s.book, id)).clone())
    }

    /// The table whose first row is `at` typeset for the view at `zoom` times the text size: its node in the current
    /// theme, its width and height, and the areas of its note markers and links.
    pub fn table_view(&self, at: Locator, zoom: f32) -> Option<(gsk::RenderNode, f32, f32, Vec<Area>)> {
        self.with(|s| {
            let mut typesetter = Typesetter::new(s.settings.table_style(zoom), s.hyphenator.clone());
            let view = layout_table(&s.book, at, &mut typesetter)?;
            let node = content_node(&view.content, s.theme, &mut s.fonts, &mut s.textures, &s.book)?;
            Some((node, view.width, view.height, view.content.areas))
        })
    }

    /// Jumps to `at`, remembering where the jump started (`back` returns there).
    pub fn go_to(&self, at: Locator) {
        self.with(|s| {
            s.back.push(s.locator);
            if s.back.len() > BACK_STACK {
                s.back.remove(0);
            }
            s.pill = Some(Pill { arrived: forzac_core::state::clamp(&s.book, at), turns: 0 });
        });
        self.show(at);
    }

    /// Goes to match `i` of `hits` (remembered for `back`) and highlights it.
    pub fn go_to_match(&self, hits: Rc<Vec<Hit>>, i: usize) {
        let Some(at) = hits.get(i).map(|h| h.start) else { return };
        self.with(|s| s.search = Some((hits, Some(i))));
        self.go_to(at);
    }

    /// Goes to the next (`delta` 1) or previous (−1) match of the last search, if there is one.
    pub fn next_match(&self, delta: i32) {
        let at = self.with(|s| {
            let (hits, current) = s.search.as_mut()?;
            // Without a highlight: from the reading position.
            let here = s.locator;
            let i = match *current {
                Some(i) => i as i64 + delta as i64,
                None if delta > 0 => hits.iter().position(|h| h.start >= here)? as i64,
                None => hits.iter().rposition(|h| h.start < here)? as i64,
            };
            let hit = hits.get(usize::try_from(i).ok()?)?;
            *current = Some(i as usize);
            Some(hit.start)
        });
        if let Some(at) = at {
            self.show(at);
        }
    }

    /// Removes the match highlight; false when there was none.
    pub fn clear_match(&self) -> bool {
        let had = self.with(|s| s.search.as_mut().and_then(|(_, current)| current.take()).is_some());
        self.queue_draw();
        had
    }

    /// The highlighted match, if it is in `chapter`.
    fn highlighted(s: &State, chapter: usize) -> Option<(Locator, Locator)> {
        let (hits, current) = s.search.as_ref()?;
        let hit = hits.get((*current)?)?;
        (hit.start.chapter == chapter).then_some((hit.start, hit.end))
    }

    /// Draws `rects` (a match) as soft bars in the ink, Android's selection colour.
    fn draw_marks(s: &State, snapshot: &gtk::Snapshot, rects: &[forzac_core::Rect], dy: f32) {
        let mut ink = rgba(THEMES[s.theme].ink);
        ink.set_alpha(0.14);
        for r in rects {
            let bounds = graphene::Rect::new(r.x - 2.0, r.y + dy, r.w + 4.0, r.h);
            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(bounds, 4.0));
            snapshot.append_color(&ink, &bounds);
            snapshot.pop();
        }
    }

    /// Returns to where the last jump started; false when there is none.
    pub fn back(&self) -> bool {
        let Some(at) = self.with(|s| {
            s.pill = None;
            s.back.pop()
        }) else {
            return false;
        };
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
            s.at_end = false;
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

    /// Sets the book's bookmarks (from the reading state) without saving them.
    pub fn set_bookmarks(&self, marks: Vec<Locator>) {
        self.with(|s| s.bookmarks = marks);
        self.queue_draw();
    }

    /// Calls `f` with the bookmarks whenever they change.
    pub fn connect_bookmarks(&self, f: impl Fn(&[Locator]) + 'static) {
        *self.imp().on_bookmarks.borrow_mut() = Some(Box::new(f));
    }

    /// The bookmarks as the list shows them.
    pub fn bookmark_entries(&self) -> Vec<forzac_core::bookmarks::Entry> {
        self.with(|s| forzac_core::bookmarks::entries(&s.book, &s.bookmarks))
    }

    /// `b`: sets a bookmark on the page (or screen) shown, or removes its bookmarks; the ribbon drops in or goes up.
    pub fn toggle_bookmark(&self) {
        let h = WidgetExt::height(self) as f32;
        let changed = self.with(|s| {
            let range = Self::shown_range(s, h)?;
            let drop = forzac_core::bookmarks::toggle(&mut s.bookmarks, range.0, range.1);
            s.ribbon = Some(Ribbon { drop, range, start: None });
            Some(())
        });
        if changed.is_none() {
            return;
        }
        self.saved_bookmarks();
        self.imp().frames.borrow_mut().push(Vec::new());
        self.add_tick_callback(|view, clock| {
            view.queue_draw();
            // Frame times for `--toggles`, as for page turns.
            if let Some(frames) = view.imp().frames.borrow_mut().last_mut() {
                frames.push(clock.frame_time());
            }
            if view.with(|s| s.ribbon.is_some()) { glib::ControlFlow::Continue } else { glib::ControlFlow::Break }
        });
    }

    /// Removes the bookmark at `at` (from the list), without animation.
    pub fn remove_bookmark(&self, at: Locator) {
        self.with(|s| s.bookmarks.retain(|m| *m != at));
        self.saved_bookmarks();
        self.queue_draw();
    }

    fn saved_bookmarks(&self) {
        let marks = self.with(|s| s.bookmarks.clone());
        if let Some(f) = self.imp().on_bookmarks.borrow().as_ref() {
            f(&marks);
        }
    }

    /// The range of text shown: the page in "Slide" (none on the end page), the screen in "Scroll".
    fn shown_range(s: &mut State, h: f32) -> Option<(Locator, Locator)> {
        if s.mode == Mode::Scroll {
            return Self::screen_range(s, h);
        }
        let layout = s.layout.as_ref().filter(|_| !s.at_end)?;
        Some((layout.page_start(s.page), layout.page_end(s.page)))
    }

    fn shown_marked(s: &mut State, h: f32) -> bool {
        Self::shown_range(s, h).is_some_and(|(from, to)| forzac_core::bookmarks::marked(&s.bookmarks, from, to))
    }

    /// Where the ribbon hangs now: 0 at rest, negative while it moves above; `None` when it is not drawn. Ends a
    /// movement that has finished or whose page is no longer shown.
    fn ribbon_offset(s: &mut State, h: f32, now: i64, refresh: i64) -> Option<f32> {
        let marked = Self::shown_marked(s, h);
        let Some(ribbon) = &mut s.ribbon else { return marked.then_some(0.0) };
        // Just out of the window: every frame of the movement changes what is shown.
        let hidden = -RIBBON_LENGTH;
        // The first frame already moves, as a scroll movement's does: a frame that draws what is on screen presents
        // nothing and the next one comes a refresh late.
        let t = (now - *ribbon.start.get_or_insert(now - refresh)) as f32 / 1e6;
        let offset = if ribbon.drop {
            // A damped spring from `hidden` to rest (underdamped: the small bounce).
            let w = RIBBON_STIFFNESS.sqrt();
            let decay = RIBBON_DAMPING * w;
            let wd = w * (1.0 - RIBBON_DAMPING * RIBBON_DAMPING).sqrt();
            let envelope = (-decay * t).exp();
            (envelope * -hidden >= 0.5).then(|| hidden * envelope * ((wd * t).cos() + decay / wd * (wd * t).sin()))
        } else {
            let p = t * 1e6 / RIBBON_RISE as f32;
            (p < 1.0).then_some(hidden * p * p)
        };
        if offset.is_none() || Self::shown_range(s, h) != s.ribbon.as_ref().map(|r| r.range) {
            s.ribbon = None;
            return marked.then_some(0.0);
        }
        offset
    }

    /// The ribbon hanging `offset` below its rest, in `theme`, beside the text column of `style`.
    fn draw_ribbon(snapshot: &gtk::Snapshot, theme: usize, style: &PageStyle, offset: f32) {
        let palette = THEMES[theme];
        let colour = rgba(if palette.paper < 0x808080 { RIBBON_DARK } else { RIBBON_LIGHT });
        let x = style.ribbon().0 - RIBBON_WIDTH / 2.0;
        let (top, bottom) = (offset - RIBBON_TUCK, offset + RIBBON_LENGTH);
        let path = gsk::PathBuilder::new();
        path.move_to(x, top);
        path.line_to(x + RIBBON_WIDTH, top);
        path.line_to(x + RIBBON_WIDTH, bottom);
        path.line_to(x + RIBBON_WIDTH / 2.0, bottom - RIBBON_TAIL);
        path.line_to(x, bottom);
        path.close();
        snapshot.push_shadow(&[gsk::Shadow::new(gdk::RGBA::new(0.0, 0.0, 0.0, 0.3), 0.0, 1.5, 3.0)]);
        snapshot.append_fill(&path.to_path(), gsk::FillRule::Winding, &colour);
        snapshot.pop();
    }

    /// The body text's line height.
    pub fn line_height(&self) -> f32 {
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

    /// Whether the book's end page (or end screen) is shown: the position is saved as finished.
    pub fn finished(&self) -> bool {
        self.with(|s| if s.mode == Mode::Scroll { s.column_end } else { s.at_end })
    }

    pub fn book(&self) -> Arc<Book> {
        self.with(|s| s.book.clone())
    }

    /// The theme shown, an index in `THEMES`.
    pub fn theme(&self) -> usize {
        self.with(|s| s.theme)
    }

    pub fn system_dark(&self) -> bool {
        self.with(|s| s.system_dark)
    }

    pub fn settings(&self) -> Settings {
        self.with(|s| Settings { turning: if s.mode == Mode::Scroll { Turning::Scroll } else { Turning::Slide }, ..s.settings })
    }

    /// Applies `settings`: a new theme only redraws; the page style changes re-typeset at the next frame, keeping the
    /// reading position; the page mode switches keeping the top line.
    pub fn set_settings(&self, settings: Settings) {
        if settings.turning != self.settings().turning {
            self.toggle_mode();
        }
        let old = self.theme_changes(settings.theme_index(self.system_dark()));
        self.with(|s| {
            s.settings = settings;
            Self::show_theme(s);
        });
        self.fade_from(old);
        self.queue_draw();
    }

    /// The frame as drawn now, when the theme is about to become `theme`, so the change can cross-fade from it.
    fn theme_changes(&self, theme: usize) -> Option<gsk::RenderNode> {
        if theme == self.theme() || WidgetExt::width(self) <= 0 {
            return None;
        }
        let snapshot = gtk::Snapshot::new();
        self.draw_frame(&snapshot);
        snapshot.to_node()
    }

    /// Cross-fades from `old` (the frame before a theme change) to the new colours on the frame clock.
    fn fade_from(&self, old: Option<gsk::RenderNode>) {
        let Some(old) = old else { return };
        self.with(|s| s.fade = Some((old, None)));
        self.add_tick_callback(|view, _| {
            view.queue_draw();
            if view.with(|s| s.fade.is_some()) { glib::ControlFlow::Continue } else { glib::ControlFlow::Break }
        });
    }

    /// Switches `setting` to its next (`delta` 1) or previous (−1) value.
    pub fn step(&self, setting: Setting, delta: i32) {
        let mut settings = self.settings();
        settings.step(setting, delta, self.system_dark());
        self.set_settings(settings);
    }

    /// The desktop's light or dark preference changed.
    pub fn set_system_dark(&self, dark: bool) {
        let old = self.theme_changes(self.settings().theme_index(dark));
        self.with(|s| {
            s.system_dark = dark;
            Self::show_theme(s);
        });
        self.fade_from(old);
        self.queue_draw();
    }

    fn show_theme(s: &mut State) {
        let theme = s.settings.theme_index(s.system_dark);
        if theme != s.theme {
            s.theme = theme;
            s.nodes.clear(); // ink is baked into the nodes; only a redraw, no re-layout
            s.areas.clear();
            s.column.tiles.clear();
            s.end = None;
        }
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
            let style = s.settings.page_style(w, h);
            let fresh = s.layout.as_ref().is_some_and(|l| l.chapter == s.chapter) && s.style == Some(style);
            if fresh {
                return false;
            }
            if s.style != Some(style) {
                s.typesetter = Some(Typesetter::new(style, s.hyphenator.clone()));
                s.style = Some(style);
                s.neighbours.clear();
                s.layout = None;
                s.end = None;
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
                let textures = images.into_iter().map(|id| (id.clone(), decode(&book, &id))).collect();
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

    /// The end page's node for the current style and theme.
    pub(super) fn end_node(s: &mut State) -> Option<gsk::RenderNode> {
        if s.end.is_none() {
            let (style, t) = (s.style?, s.typesetter.as_mut()?);
            let content = end_page(&s.book, &style, t);
            s.end = content_node(&content, s.theme, &mut s.fonts, &mut s.textures, &s.book);
        }
        s.end.clone()
    }

    fn draw(&self, snapshot: &gtk::Snapshot) {
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        match self.with(|s| s.fade.as_mut().map(|(old, start)| (old.clone(), *start.get_or_insert(now)))) {
            Some((old, start)) if now - start < THEME_FADE => {
                let frame = gtk::Snapshot::new();
                self.draw_frame(&frame);
                let new = frame.to_node().unwrap_or_else(|| gsk::ContainerNode::new(&[]).upcast());
                let p = (now - start) as f32 / THEME_FADE as f32;
                snapshot.append_node(gsk::CrossFadeNode::new(&old, &new, p));
            }
            fade => {
                if fade.is_some() {
                    self.with(|s| s.fade = None);
                }
                self.draw_frame(snapshot);
            }
        }
        if self.mode() == Mode::Scroll && self.with(Self::placed) {
            self.prepare();
        }
    }

    /// The page (or the column) and the pill over it.
    fn draw_frame(&self, snapshot: &gtk::Snapshot) {
        self.draw_page(snapshot);
        self.draw_pill(snapshot);
    }

    /// The "← Back · Ctrl+O" pill over the foot while the last jump is fresh: the ink as its fill, the paper as its text
    /// (Android's `ReturnPill`), fading in and out.
    fn draw_pill(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        let (alpha, fading, palette, margin_bottom) = self.with(|s| {
            // Read on: two pages (or screens) away from where the jump arrived.
            let far = match (&s.pill, s.mode) {
                (Some(p), Mode::Slide) => p.turns.abs() >= PILL_PAGES,
                (Some(p), Mode::Scroll) => match s.layout.as_ref().filter(|l| l.chapter == p.arrived.chapter) {
                    Some(l) => (l.flow_y(s.locator) - l.flow_y(p.arrived)).abs() >= PILL_PAGES as f32 * h,
                    None => s.chapter.abs_diff(p.arrived.chapter) > 0 && s.column.motion.is_none(),
                },
                (None, _) => false,
            };
            if far {
                s.pill = None;
            }
            let visible = s.pill.is_some() && !s.back.is_empty() && !s.at_end && !s.column_end;
            if visible != s.pill_shown {
                s.pill_shown = visible;
                s.pill_change = Some(now);
            }
            let p = s.pill_change.map_or(1.0, |t| ((now - t) as f32 / PILL_FADE as f32).clamp(0.0, 1.0));
            if p >= 1.0 {
                s.pill_change = None;
            }
            let alpha = if s.pill_shown { p } else { 1.0 - p };
            (alpha, s.pill_change.is_some(), THEMES[s.theme], s.style.map_or(56.0, |st| st.margin_bottom))
        });
        if fading {
            self.add_tick_callback(|view, _| {
                view.queue_draw();
                glib::ControlFlow::Break
            });
        }
        if alpha <= 0.0 {
            self.with(|s| s.pill_rect = None);
            return;
        }
        crate::fonts::add_interface_fonts();
        let layout = self.create_pango_layout(Some(PILL_TEXT));
        layout.set_font_description(Some(&gtk::pango::FontDescription::from_string("Golos Text 14px")));
        let (tw, th) = layout.pixel_size();
        let (pw, ph) = (tw as f32 + 36.0, th as f32 + 20.0);
        // Centred on the foot, as Android's pill replaces the page number.
        let rect = graphene::Rect::new((w - pw) / 2.0, h - margin_bottom / 2.0 - ph / 2.0, pw, ph);
        let mut fill = rgba(palette.ink);
        fill.set_alpha(alpha);
        let mut text = rgba(palette.paper);
        text.set_alpha(alpha);
        snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(rect, ph / 2.0));
        snapshot.append_color(&fill, &rect);
        snapshot.pop();
        snapshot.save();
        snapshot.translate(&graphene::Point::new(rect.x() + 18.0, rect.y() + 10.0));
        snapshot.append_layout(&layout, &text);
        snapshot.restore();
        self.with(|s| s.pill_rect = Some(rect));
    }

    fn draw_page(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        let clock = WidgetExt::frame_clock(self);
        let now = clock.as_ref().map_or(0, |c| c.frame_time());
        let refresh = clock.and_then(|c| c.current_timings()).map_or(8_333, |t| t.refresh_interval()).clamp(4_000, 17_000);
        self.with(|s| {
            snapshot.append_color(&rgba(THEMES[s.theme].paper), &graphene::Rect::new(0.0, 0.0, w, h));
            if s.mode == Mode::Scroll {
                Self::draw_column(s, snapshot, h);
                if let (Some(offset), Some(style)) = (Self::ribbon_offset(s, h, now, refresh), &s.style) {
                    Self::draw_ribbon(snapshot, s.theme, style, offset);
                }
                return;
            }
            let current = if s.at_end { Self::end_node(s) } else { Self::page_node(s, s.page) };
            let Some(current) = current else { return };
            let progress: f32 =
                s.turn.as_mut().map(|t| ((now - *t.start.get_or_insert(now)) as f32 / TURN as f32).clamp(0.0, 1.0)).unwrap_or(1.0);
            let style = s.style;
            match &s.turn {
                Some(t) if progress < 1.0 => {
                    let p = 1.0 - (1.0 - progress).powi(3); // ease-out cubic
                    let (dir, from, from_marked) = (t.dir, t.from.clone(), t.from_marked);
                    let marked = Self::shown_marked(s, h);
                    // Each page with its ribbon.
                    for (node, ribbon, x) in [(&from, from_marked, -dir * p * w), (&current, marked, dir * (1.0 - p) * w)] {
                        snapshot.save();
                        snapshot.translate(&graphene::Point::new(x, 0.0));
                        snapshot.append_node(node);
                        if let (true, Some(style)) = (ribbon, &style) {
                            Self::draw_ribbon(snapshot, s.theme, style, 0.0);
                        }
                        snapshot.restore();
                    }
                }
                _ if s.at_end => {
                    s.turn = None;
                    snapshot.append_node(&current);
                }
                _ => {
                    s.turn = None;
                    snapshot.append_node(&current);
                    if let (Some(offset), Some(style)) = (Self::ribbon_offset(s, h, now, refresh), &style) {
                        Self::draw_ribbon(snapshot, s.theme, style, offset);
                    }
                    if let (Some((from, to)), Some(layout), Some(style)) = (Self::highlighted(s, s.chapter), &s.layout, &s.style) {
                        Self::draw_marks(s, snapshot, &layout.marks(s.page, style, from, to), 0.0);
                    }
                    // The focused stop, over the cached page.
                    let areas = s.areas.get(&s.page);
                    if let (Some(areas), Some(f)) = (areas, s.focus)
                        && let Some(&stop) = stops(areas).get(f)
                    {
                        draw_focus(snapshot, s.theme, areas, stop);
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
            snapshot.append_color(&rgba(THEMES[s.theme].paper), &graphene::Rect::new(0.0, 0.0, w, h));
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
        let h = WidgetExt::height(self) as f32;
        let animate = self.with(|s| {
            let from_marked = Self::shown_marked(s, h);
            let Some(layout) = &s.layout else { return false };
            if s.at_end {
                // Back from the end page to the last page of the text; forward does nothing.
                if delta > 0 {
                    return false;
                }
                s.turn = s.end.clone().map(|from| Turn { from, from_marked: false, dir: -1.0, start: None });
                s.at_end = false;
                return true;
            }
            let from = s.nodes.get(&s.page).cloned();
            let target = s.page as i64 + delta as i64;
            if let Some(p) = &mut s.pill {
                p.turns += delta;
            }
            if delta > 0 && target as usize >= layout.pages.len() && s.chapter + 1 == s.book.chapters.len() {
                // Past the book's last page: the end page; the position stays at the last page.
                s.at_end = true;
                s.turn = from.map(|from| Turn { from, from_marked, dir: 1.0, start: None });
                s.focus = None;
                return true;
            }
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
            s.turn = from.map(|from| Turn { from, from_marked, dir: delta.signum() as f32, start: None });
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
            s.at_end = false;
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
                s.at_end = false;
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
/// ranges `(first, last)`. An image or a table is one stop however many areas it has (a table cut by the "Scroll"
/// tiles has one per tile).
pub(crate) fn stops(areas: &[Area]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (i, area) in areas.iter().enumerate() {
        if let Some(stop) = out.last_mut().filter(|s| areas[s.1].target == area.target) {
            stop.1 = i;
        } else if !(area.target.is_viewable() && out.iter().any(|s| areas[s.0].target == area.target)) {
            out.push((i, i));
        }
    }
    out
}

/// The focused stop `(first, last)` of `areas`: a marker or link as a soft box in the link ink over each of its lines;
/// an image or a table as a rounded outline in the link ink around all its areas.
pub(crate) fn draw_focus(snapshot: &gtk::Snapshot, theme: usize, areas: &[Area], (first, last): (usize, usize)) {
    let mut ink = rgba(THEMES[theme].link);
    let target = &areas[first].target;
    if target.is_viewable() {
        let Some(r) = areas.iter().filter(|a| a.target == *target).map(|a| rect(a.rect)).reduce(|a, b| a.union(&b)) else { return };
        ink.set_alpha(0.7);
        let bounds = graphene::Rect::new(r.x() - 6.0, r.y() - 6.0, r.width() + 12.0, r.height() + 12.0);
        snapshot.append_border(&gsk::RoundedRect::from_rect(bounds, 6.0), &[2.0; 4], &[ink; 4]);
        return;
    }
    ink.set_alpha(0.22);
    for area in &areas[first..=last] {
        let r = area.rect;
        let bounds = graphene::Rect::new(r.x - 3.0, r.y - 2.0, r.w + 6.0, r.h + 4.0);
        snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(bounds, 4.0));
        snapshot.append_color(&ink, &bounds);
        snapshot.pop();
    }
}

/// The render node of a page's (or a note's, or a table's) contents in `theme`: images, table rules and glyph runs.
fn content_node(
    content: &PageContent,
    theme: usize,
    fonts: &mut Fonts,
    textures: &mut HashMap<String, Option<gdk::Texture>>,
    book: &Book,
) -> Option<gsk::RenderNode> {
    let Palette { ink: text, link, .. } = THEMES[theme];
    let mut secondary = rgba(text);
    secondary.set_alpha(SECONDARY);
    let snapshot = gtk::Snapshot::new();
    for image in &content.images {
        // Decoded on the worker; an image the worker has not reached yet (the first page) is decoded here.
        let texture = textures.entry(image.id.clone()).or_insert_with(|| decode(book, &image.id));
        if let Some(texture) = texture {
            snapshot.append_scaled_texture(texture, gsk::ScalingFilter::Trilinear, &rect(image.rect));
        }
    }
    for rule in &content.rules {
        let mut ink = rgba(text);
        ink.set_alpha(RULES[rule.strength as usize]);
        snapshot.append_color(&ink, &rect(rule.rect));
    }
    for stroke in &content.strokes {
        let ink = match stroke.ink {
            Ink::Text => rgba(text),
            Ink::Link => rgba(link),
            Ink::Secondary => secondary,
        };
        snapshot.append_color(&ink, &rect(stroke.rect));
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
