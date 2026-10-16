//! The reading surface: draws pages typeset by the core as GSK text nodes, caches each page as a render node and
//! slides between them on the compositor's frame clock. Draws nothing while idle.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use forzac_core::hyphen::Hyphenator;
use forzac_core::{Book, ChapterLayout, Locator, PageStyle, Typesetter, paginate};
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, prelude::*};

use crate::fonts::Fonts;

/// Text size presets in logical pixels: steps, not a slider.
const TEXT_SIZES: [f32; 7] = [16.0, 18.0, 20.0, 22.0, 24.0, 27.0, 30.0];

/// Background and ink of the six themes (docs: the Android app's palettes).
pub const THEMES: [(&str, u32, u32); 6] = [
    ("Light", 0xFAFAFA, 0x1C1C1C),
    ("Sepia", 0xF4ECD8, 0x5B4636),
    ("Dark", 0x121212, 0xCFCFCF),
    ("Dawn", 0xFAF4ED, 0x575279),
    ("Wave", 0x1F1F28, 0xDCD7BA),
    ("Forest", 0x2D353B, 0xD3C6AA),
];

/// Page turn duration, µs.
const TURN: i64 = 200_000;

fn rgba(rgb: u32) -> gdk::RGBA {
    gdk::RGBA::new(((rgb >> 16) & 0xFF) as f32 / 255.0, ((rgb >> 8) & 0xFF) as f32 / 255.0, (rgb & 0xFF) as f32 / 255.0, 1.0)
}

struct Turn {
    from: gsk::RenderNode,
    /// +1 forward (the new page comes from the right), −1 back.
    dir: f32,
    /// Frame time of the first frame of the animation; `None` until it is drawn. The frame clock's time at the key
    /// press is stale after a pause, which would make the first frame jump ahead.
    start: Option<i64>,
}

pub struct State {
    book: Book,
    hyphenator: Option<Rc<Hyphenator>>,
    fonts: Fonts,
    chapter: usize,
    layout: Option<ChapterLayout>,
    typesetter: Option<Typesetter>,
    style: Option<PageStyle>,
    page: usize,
    /// The reading position; pages are derived from it after every re-layout.
    locator: Locator,
    to_last: bool,
    size_step: usize,
    theme: usize,
    nodes: HashMap<usize, gsk::RenderNode>,
    turn: Option<Turn>,
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct PageView {
        pub state: RefCell<Option<State>>,
        /// Frame times (µs) during page turns and the refresh interval, for `--turns`.
        pub frames: RefCell<Vec<Vec<i64>>>,
        pub refresh: std::cell::Cell<i64>,
        pub on_first_frame: RefCell<Option<Box<dyn FnOnce(&super::PageView)>>>,
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
            view.relayout();
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
        @implements gtk::Buildable, gtk::ConstraintTarget;
}

impl PageView {
    pub fn new(book: Book, chapter: usize) -> Self {
        let view: Self = glib::Object::new();
        let hyphenator = Hyphenator::for_language(book.language.as_deref()).map(Rc::new);
        let chapter = chapter.min(book.chapters.len().saturating_sub(1));
        *view.imp().state.borrow_mut() = Some(State {
            book,
            hyphenator,
            fonts: Fonts::new(),
            chapter,
            layout: None,
            typesetter: None,
            style: None,
            page: 0,
            locator: Locator { chapter, ..Locator::default() },
            to_last: false,
            size_step: 2,
            theme: 0,
            nodes: HashMap::new(),
            turn: None,
        });
        view.set_hexpand(true);
        view.set_vexpand(true);
        view.set_focusable(true);
        view
    }

    /// Runs once, after the first frame with a page is presented (measurements, `--png`).
    pub fn connect_first_frame(&self, f: impl FnOnce(&PageView) + 'static) {
        *self.imp().on_first_frame.borrow_mut() = Some(Box::new(f));
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(self.imp().state.borrow_mut().as_mut().unwrap())
    }

    /// Typesets the current chapter when the size, text size or chapter changed; keeps the reading position.
    fn relayout(&self) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        self.with(|s| {
            let style = PageStyle::for_window(w, h, TEXT_SIZES[s.size_step]);
            let fresh = s.layout.as_ref().is_some_and(|l| l.chapter == s.chapter) && s.style == Some(style);
            if fresh {
                return;
            }
            if s.style != Some(style) {
                s.typesetter = Some(Typesetter::new(style, s.hyphenator.clone()));
                s.style = Some(style);
            }
            let layout = paginate(&s.book, s.chapter, &style, s.typesetter.as_mut().unwrap());
            s.page = if std::mem::take(&mut s.to_last) { layout.pages.len() - 1 } else { layout.page_of(s.locator) };
            s.layout = Some(layout);
            s.nodes.clear();
        });
    }

    fn page_node(s: &mut State, page: usize) -> Option<gsk::RenderNode> {
        if let Some(n) = s.nodes.get(&page) {
            return Some(n.clone());
        }
        let (layout, style, t) = (s.layout.as_ref()?, s.style?, s.typesetter.as_mut()?);
        let runs = layout.render(page, &style, t);
        let ink = rgba(THEMES[s.theme].2);
        let snapshot = gtk::Snapshot::new();
        for run in &runs {
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
                // `set_size` leaves the new entries uninitialised; a stray `is_color` bit makes GSK draw the glyph as a
                // colour glyph (invisible here). The bindings have no setter, so set the bitfield: cluster start only.
                unsafe { (*(info as *mut gtk::pango::GlyphInfo).cast::<gtk::pango::ffi::PangoGlyphInfo>()).attr.is_cluster_start = 1 };
            }
            let font = s.fonts.get(run.face, run.size);
            if let Some(node) = gsk::TextNode::new(font, &glyphs, &ink, &graphene::Point::new(x0, y0)) {
                snapshot.append_node(node);
            }
        }
        let node = snapshot.to_node()?;
        s.nodes.insert(page, node.clone());
        Some(node)
    }

    fn draw(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        self.with(|s| {
            snapshot.append_color(&rgba(THEMES[s.theme].1), &graphene::Rect::new(0.0, 0.0, w, h));
            let Some(current) = Self::page_node(s, s.page) else { return };
            let progress: f32 = s
                .turn
                .as_mut()
                .map(|t| ((now - *t.start.get_or_insert(now)) as f32 / TURN as f32).clamp(0.0, 1.0))
                .unwrap_or(1.0);
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

    pub fn turn(&self, delta: i32) {
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
            true
        });
        if animate {
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

    pub fn chapter(&self, delta: i32) {
        self.with(|s| {
            let target = (s.chapter as i64 + delta as i64).clamp(0, s.book.chapters.len() as i64 - 1) as usize;
            s.chapter = target;
            s.locator = Locator { chapter: target, ..Locator::default() };
            s.turn = None;
        });
        self.queue_draw();
    }

    pub fn chapter_edge(&self, end: bool) {
        self.with(|s| {
            if let Some(layout) = &s.layout {
                s.page = if end { layout.pages.len() - 1 } else { 0 };
                s.locator = layout.page_start(s.page);
                s.turn = None;
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
