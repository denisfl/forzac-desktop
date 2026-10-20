//! Selecting text and highlights in the page view (`quotes`): a selection by words with the mouse or, like Vim's visual
//! mode, from the keyboard; the bar over it; highlights drawn under the text and baked into the cached pages and tiles.
//!
//! A selection keeps Locators (the cursor's word and the anchor's word), so turning, scrolling and re-layout keep it;
//! the words are taken from what is shown when they are needed.

use std::time::Duration;

use forzac_core::i18n::tr;
use forzac_core::{Area, Locator, Rect, Target, Word, selection};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk};

use super::{Mode, PageView, State, THEMES, rgba};

/// Android's marker colour (`markerColor`): the Forzac ochre at 32 %, 30 % on dark themes.
const MARKER: u32 = 0xC49A52;
/// The stroke of a new highlight: per line, at least, at most (µs).
const STROKE_LINE: i64 = 90_000;
const STROKE_MIN: i64 = 180_000;
const STROKE_MAX: i64 = 600_000;
/// The zone at the window's sides where a held drag turns the page, and how long it is held.
const EDGE: f32 = 24.0;
const EDGE_HOLD: Duration = Duration::from_millis(500);
/// How far a press moves before it selects, and how fast a drag outside the window scrolls ("Scroll"), per pixel out.
const DRAG_START: f32 = 4.0;
const DRAG_SCROLL: f32 = 8.0;
/// The bar: its fade-in (µs) and the room around its items.
const BAR_FADE: i64 = 120_000;
const BAR_GAP: f32 = 14.0;

/// A selection: the cursor's word and, once anchored, the anchor's word (`[start, end)` of each).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Selection {
    pub cursor: (Locator, Locator),
    pub anchor: Option<(Locator, Locator)>,
    /// Started from the keyboard: the cursor is shown.
    pub keyboard: bool,
}

impl Selection {
    pub fn range(&self) -> Option<(Locator, Locator)> {
        self.anchor.map(|a| selection::span(a, self.cursor))
    }
}

/// What the bar acts on, and its items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Bar {
    Selection,
    Highlight((Locator, Locator)),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Action {
    Highlight,
    Copy,
    Delete,
}

/// Where the cursor goes once the words of what is now shown are known (after a turn or a scroll).
#[derive(Debug, Clone, Copy)]
pub(super) enum Pending {
    /// The next (previous) word after the cursor's.
    Step(i32),
    /// The word on the next (previous) line nearest this middle.
    Line(i32, f32),
}

/// A drag with the mouse: where it was pressed, the last point, whether it selects yet, and the edge it is held at.
pub(super) struct Drag {
    press: (f32, f32),
    at: (f32, f32),
    pub started: bool,
    edge: Option<(i32, glib::SourceId)>,
    /// "Scroll": the column glides while the pointer is out of the window.
    gliding: bool,
}

#[derive(Default)]
pub(super) struct Texts {
    pub selection: Option<Selection>,
    pub pending: Option<Pending>,
    pub drag: Option<Drag>,
    /// The last press was a drag that selected: its release is not a click (whichever gesture hears it first).
    pub dragged: bool,
    /// The book's highlights in reading order, and the one drawing in.
    pub highlights: Vec<(Locator, Locator)>,
    pub reveal: Option<((Locator, Locator), Option<i64>)>,
    /// The bar shown, the frame time it appeared, and its items' rectangles (for clicks).
    pub bar: Option<Bar>,
    pub bar_since: Option<i64>,
    pub bar_items: Vec<(graphene::Rect, Action)>,
}

/// The marker colour in `theme`.
fn marker(theme: usize) -> gdk::RGBA {
    let mut c = rgba(MARKER);
    c.set_alpha(if THEMES[theme].paper < 0x808080 { 0.30 } else { 0.32 });
    c
}

/// Android's bands (`drawBands`): rounded, 2 px wider on each side; `progress` < 1 fills that share of their length, line
/// after line.
pub(super) fn draw_bands(snapshot: &gtk::Snapshot, rects: &[Rect], colour: &gdk::RGBA, dy: f32, progress: f32) {
    let total: f32 = rects.iter().map(|r| r.w + 4.0).sum();
    let mut left = total * progress.clamp(0.0, 1.0);
    for r in rects {
        if left <= 0.0 {
            break;
        }
        let w = (r.w + 4.0).min(left);
        left -= r.w + 4.0;
        let bounds = graphene::Rect::new(r.x - 2.0, r.y + dy, w, r.h);
        snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(bounds, 3.0));
        snapshot.append_color(colour, &bounds);
        snapshot.pop();
    }
}

/// The highlights of `chapter` (but the one drawing in) as bands under a page's or tile's content, from `rects_of`.
pub(super) fn baked(s: &State, chapter: usize, rects_of: impl Fn(Locator, Locator) -> Vec<Rect>) -> Option<gsk::RenderNode> {
    let revealing = s.text.reveal.map(|r| r.0);
    let colour = marker(s.theme);
    let snapshot = gtk::Snapshot::new();
    let mut any = false;
    for &q in s.text.highlights.iter().filter(|q| q.0.chapter <= chapter && q.1.chapter >= chapter && Some(**q) != revealing) {
        let rects = rects_of(q.0, q.1);
        any |= !rects.is_empty();
        draw_bands(&snapshot, &rects, &colour, 0.0, 1.0);
    }
    if any { snapshot.to_node() } else { None }
}

/// `areas` with the highlights' line rectangles (from `rects_of`) added in reading order: each before the first area
/// after its first line.
pub(super) fn with_highlights(
    s: &State,
    chapter: usize,
    mut areas: Vec<Area>,
    rects_of: impl Fn(Locator, Locator) -> Vec<Rect>,
) -> Vec<Area> {
    for &(from, to) in s.text.highlights.iter().filter(|q| q.0.chapter <= chapter && q.1.chapter >= chapter) {
        let rects = rects_of(from, to);
        let Some(first) = rects.first() else { continue };
        let after = |a: &Area| {
            a.rect.y > first.y + first.h / 2.0 || (a.rect.y + a.rect.h > first.y && a.rect.y < first.y + first.h && a.rect.x > first.x)
        };
        let at = areas.iter().position(after).unwrap_or(areas.len());
        let target = Target::Highlight(from);
        areas.splice(at..at, rects.iter().map(|r| Area { rect: *r, target: target.clone() }));
    }
    areas
}

impl PageView {
    /// Sets the book's highlights (from the reading state) without saving them.
    pub fn set_highlights(&self, highlights: Vec<(Locator, Locator)>) {
        self.with(|s| {
            s.text.highlights = highlights;
            Self::drop_pages(s);
        });
        self.queue_draw();
    }

    /// Calls `f` with the highlights whenever they change.
    pub fn connect_highlights(&self, f: impl Fn(&[(Locator, Locator)]) + 'static) {
        *self.imp().on_highlights.borrow_mut() = Some(Box::new(f));
    }

    /// The highlights as the list shows them.
    pub fn highlight_entries(&self) -> Vec<forzac_core::quotes::Entry> {
        self.with(|s| forzac_core::quotes::entries(&s.book, &s.text.highlights))
    }

    /// All highlights in Android's export format.
    pub fn highlights_export(&self) -> String {
        self.with(|s| forzac_core::quotes::export(&s.book, &forzac_core::quotes::entries(&s.book, &s.text.highlights)))
    }

    /// Deletes the highlight `range` (from the list or its bar), without animation.
    pub fn delete_highlight(&self, range: (Locator, Locator)) {
        self.with(|s| {
            s.text.highlights.retain(|q| *q != range);
            s.focus = None;
            if s.text.bar == Some(super::selection::Bar::Highlight(range)) {
                s.text.bar = None;
            }
            Self::drop_pages(s);
        });
        self.saved_highlights();
        self.queue_draw();
    }

    /// The text of `range` as copied.
    pub fn text_of(&self, range: (Locator, Locator)) -> String {
        self.with(|s| s.book.text(range.0, range.1, usize::MAX))
    }

    /// Puts `text` on the clipboard.
    pub fn copy(&self, text: &str) {
        self.clipboard().set_text(text);
    }

    /// Whether text is selected, or the selection mode is on: the page's keys go to `selection_key`.
    pub fn selecting(&self) -> bool {
        self.with(|s| s.text.selection.is_some() || s.text.bar.is_some())
    }

    fn saved_highlights(&self) {
        let highlights = self.with(|s| s.text.highlights.clone());
        if let Some(f) = self.imp().on_highlights.borrow().as_ref() {
            f(&highlights);
        }
    }

    /// Cached pages and tiles have the highlights baked in: drawn again when they change.
    fn drop_pages(s: &mut State) {
        s.nodes.clear();
        s.areas.clear();
        s.column.tiles.clear();
    }

    /// The words shown, in view coordinates: the page's, or in "Scroll" those on lines wholly in the window.
    pub(super) fn shown_words(s: &mut State, h: f32) -> Vec<Word> {
        let Some(style) = s.style else { return Vec::new() };
        if s.mode == Mode::Scroll {
            let mut out = Vec::new();
            for (chapter, top) in Self::screen_chapters(s, h) {
                if let Some(l) = super::scroll::layout_if_ready(s, chapter) {
                    out.extend(
                        l.flow_words(-top, h - top, &style, &s.book)
                            .into_iter()
                            .filter(|w| w.rect.y >= -0.5 && w.rect.y + w.rect.h <= h + 0.5),
                    );
                }
            }
            return out;
        }
        match &s.layout {
            Some(l) if l.chapter == s.chapter && !s.at_end => {
                // A spread's two pages: the right one's words where it is drawn.
                let mut out = Vec::new();
                for (page, dx) in Self::shown_pages(s) {
                    out.extend(l.words(page, &style, &s.book).into_iter().map(|w| Word { rect: Rect { x: w.rect.x + dx, ..w.rect }, ..w }));
                }
                out
            }
            _ => Vec::new(),
        }
    }

    /// Rectangles over `[from, to)` shown, in view coordinates.
    pub(super) fn shown_rects(s: &mut State, h: f32, from: Locator, to: Locator) -> Vec<Rect> {
        let Some(style) = s.style else { return Vec::new() };
        if s.mode == Mode::Scroll {
            let mut out = Vec::new();
            for (chapter, top) in Self::screen_chapters(s, h) {
                if let Some(l) = super::scroll::layout_if_ready(s, chapter) {
                    out.extend(l.flow_marks(-top, h - top, &style, from, to));
                }
            }
            return out;
        }
        match &s.layout {
            Some(l) if l.chapter == s.chapter && !s.at_end => Self::shown_marks(s, from, to),
            _ => Vec::new(),
        }
    }

    // Mouse.

    /// A press at `(x, y)`: a drag may follow. On a handle, the drag moves that end.
    pub(super) fn drag_begin(&self, x: f32, y: f32) {
        self.with(|s| {
            s.text.drag = Some(Drag { press: (x, y), at: (x, y), started: false, edge: None, gliding: false });
            s.text.dragged = false;
        });
    }

    /// The pointer moved to `(x, y)` while pressed: past a few pixels the drag selects from the pressed word to this one.
    pub(super) fn drag_update(&self, x: f32, y: f32) {
        let h = WidgetExt::height(self) as f32;
        let w = WidgetExt::width(self) as f32;
        let edge = self.with(|s| {
            let words = Self::shown_words(s, h);
            let drag = s.text.drag.as_mut()?;
            drag.at = (x, y);
            if !drag.started {
                if (x - drag.press.0).hypot(y - drag.press.1) < DRAG_START {
                    return None;
                }
                drag.started = true;
                let press = drag.press;
                let grabbed = Self::grab(s, h, press);
                if !grabbed {
                    let i = selection::word_at(&words, press.0, press.1)?;
                    let word = (words[i].start, words[i].end);
                    s.text.selection = Some(Selection { cursor: word, anchor: Some(word), keyboard: false });
                }
                s.text.bar = None;
            }
            if let Some(i) = selection::word_at(&words, x, y)
                && let Some(sel) = &mut s.text.selection
            {
                sel.cursor = (words[i].start, words[i].end);
            }
            // The zone the pointer is in: a side of the window, or above or below the text.
            let margin = s.style.map_or((0.0, 0.0), |st| (st.margin_top, st.margin_bottom));
            Some(match s.mode {
                Mode::Slide if x > w - EDGE || y > h - margin.1 => 1,
                Mode::Slide if x < EDGE || y < margin.0 => -1,
                Mode::Scroll if y > h || y < 0.0 => 2,
                _ => 0,
            })
        });
        match edge {
            Some(2) => self.drag_glide(),
            Some(dir) => self.drag_edge(dir),
            None => {}
        }
        self.queue_draw();
    }

    /// The press at `point` is on a handle: the selection's other end becomes the anchor, so the drag moves this one.
    fn grab(s: &mut State, h: f32, point: (f32, f32)) -> bool {
        let Some(sel) = s.text.selection else { return false };
        let Some((from, to)) = sel.range() else { return false };
        let rects = Self::shown_rects(s, h, from, to);
        let (Some(first), Some(last)) = (rects.first(), rects.last()) else { return false };
        let near = |x: f32, y: f32| (point.0 - x).hypot(point.1 - y) < 14.0;
        let (Some(anchor), cursor) = (sel.anchor, sel.cursor) else { return false };
        let (early, late) = if anchor.0 <= cursor.0 { (anchor, cursor) } else { (cursor, anchor) };
        let grab = if near(first.x, first.y + first.h / 2.0) {
            Some((early, late))
        } else if near(last.x + last.w, last.y + last.h / 2.0) {
            Some((late, early))
        } else {
            None
        };
        if let (Some((moving, fixed)), Some(sel)) = (grab, &mut s.text.selection) {
            sel.cursor = moving;
            sel.anchor = Some(fixed);
        }
        grab.is_some()
    }

    /// "Slide": while the drag is held in the zone `dir` (+1 forward, −1 back), the page turns every half second.
    fn drag_edge(&self, dir: i32) {
        let current = self.with(|s| s.text.drag.as_ref().and_then(|d| d.edge.as_ref().map(|e| e.0)));
        if current == Some(dir) || (current.is_none() && dir == 0) {
            return;
        }
        if let Some(Some((_, id))) = self.with(|s| s.text.drag.as_mut().map(|d| d.edge.take())) {
            id.remove();
        }
        if dir == 0 {
            return;
        }
        let view = self.downgrade();
        let id = glib::timeout_add_local(EDGE_HOLD, move || {
            let Some(view) = view.upgrade() else { return glib::ControlFlow::Break };
            view.turn(dir);
            // Once the new page is drawn, the cursor follows the pointer on it.
            view.add_tick_callback(|view, _| {
                if let Some((x, y)) = view.with(|s| s.text.drag.as_ref().map(|d| d.at)) {
                    view.drag_update(x, y);
                }
                glib::ControlFlow::Break
            });
            glib::ControlFlow::Continue
        });
        self.with(|s| {
            if let Some(d) = &mut s.text.drag {
                d.edge = Some((dir, id));
            }
        });
    }

    /// "Scroll": the column moves while the pointer is above or below the window, faster the further out.
    fn drag_glide(&self) {
        if self.with(|s| s.text.drag.as_mut().is_none_or(|d| std::mem::replace(&mut d.gliding, true))) {
            return;
        }
        self.add_tick_callback(|view, clock| {
            let h = WidgetExt::height(view) as f32;
            let refresh = clock.current_timings().map_or(8_333, |t| t.refresh_interval()) as f32 / 1e6;
            let Some((x, y)) = view.with(|s| s.text.drag.as_ref().map(|d| d.at)) else { return glib::ControlFlow::Break };
            let out = if y > h {
                y - h
            } else if y < 0.0 {
                y
            } else {
                0.0
            };
            if out == 0.0 {
                view.with(|s| {
                    if let Some(d) = &mut s.text.drag {
                        d.gliding = false;
                    }
                });
                return glib::ControlFlow::Break;
            }
            view.scroll_pixels(out * DRAG_SCROLL * refresh);
            view.drag_update(x, y.clamp(0.0, h - 1.0));
            glib::ControlFlow::Continue
        });
    }

    /// The press ended: a drag that selected shows the bar. True when it was a drag (the click is not a click).
    pub(super) fn drag_end(&self) -> bool {
        let drag = self.with(|s| s.text.drag.take());
        let Some(mut drag) = drag else { return false };
        self.with(|s| s.text.dragged = drag.started);
        if let Some((_, id)) = drag.edge.take() {
            id.remove();
        }
        if drag.started {
            self.with(|s| {
                if s.text.selection.is_some_and(|sel| sel.range().is_some()) {
                    s.text.bar = Some(Bar::Selection);
                    s.text.bar_since = None;
                }
            });
            self.queue_draw();
        }
        drag.started
    }

    /// A click (the `n`th in a row) at `(x, y)` on the page; true when the selection used it.
    pub(super) fn selection_click(&self, n: i32, x: f32, y: f32, shift: bool) -> bool {
        let (h, w) = (WidgetExt::height(self) as f32, WidgetExt::width(self) as f32);
        if let Some(action) =
            self.with(|s| s.text.bar_items.iter().find(|(r, _)| r.contains_point(&graphene::Point::new(x, y))).map(|i| i.1))
        {
            self.act(action);
            return true;
        }
        let consumed = self.with(|s| {
            let words = Self::shown_words(s, h);
            let word = selection::word_at(&words, x, y).map(|i| (words[i].start, words[i].end));
            // A double or triple click selects where a click does not turn the page.
            let middle = s.mode == Mode::Scroll || (x >= w / 3.0 && x <= w * 2.0 / 3.0);
            match (n, word) {
                (2, Some(word)) if middle => {
                    s.text.selection = Some(Selection { cursor: word, anchor: Some(word), keyboard: false });
                }
                (3, Some(word)) if middle => {
                    let (from, to) = s.book.block_range(word.0);
                    s.text.selection = Some(Selection { cursor: (to, to), anchor: Some((from, from)), keyboard: false });
                }
                (_, Some(word)) if shift && s.text.selection.is_some() => {
                    let sel = s.text.selection.as_mut().unwrap();
                    sel.anchor.get_or_insert(sel.cursor);
                    sel.cursor = word;
                }
                _ if s.text.selection.is_some() || s.text.bar.is_some() => {
                    // A click away from the selection clears it, and does nothing else.
                    s.text.selection = None;
                    s.text.bar = None;
                    s.text.pending = None;
                    return true;
                }
                _ => return false,
            }
            s.text.bar = Some(Bar::Selection);
            s.text.bar_since = None;
            true
        });
        if consumed {
            self.queue_draw();
        }
        consumed
    }

    /// `--mouse`: a mouse event as the gestures report it — `down`, `drag`, `up` (a drag's end, else a click) or `click`
    /// (the `n`th in a row) at `(x, y)`.
    pub fn test_mouse(&self, kind: &str, x: f32, y: f32, n: i32) {
        match kind {
            "down" => self.drag_begin(x, y),
            "drag" => self.drag_update(x, y),
            "up" => {
                if !self.drag_end() {
                    self.click(n, x, y, false);
                }
            }
            "click" => {
                self.drag_begin(x, y);
                self.drag_end();
                self.click(n, x, y, false);
            }
            _ => {}
        }
    }

    /// Shows the bar of the highlight that starts at `start` (a click or `Enter` on it).
    pub(super) fn open_highlight(&self, start: Locator) {
        self.with(|s| {
            if let Some(&q) = s.text.highlights.iter().find(|q| q.0 == start) {
                s.text.selection = None;
                s.text.bar = Some(Bar::Highlight(q));
                s.text.bar_since = None;
            }
        });
        self.queue_draw();
    }

    // Keyboard.

    /// `v`: the selection mode, with a word cursor on the first word shown.
    pub fn start_selection(&self) {
        let h = WidgetExt::height(self) as f32;
        self.with(|s| {
            let words = Self::shown_words(s, h);
            if let Some(w) = words.first() {
                s.text.selection = Some(Selection { cursor: (w.start, w.end), anchor: None, keyboard: true });
                s.text.bar = None;
                s.focus = None;
            }
        });
        self.queue_draw();
    }

    /// A key while selecting (`latin` is its Latin letter or the key itself); true when it was used. While the mode is on
    /// every key is used; the bar of a highlight takes `d`, `c` and `Esc`.
    pub fn selection_key(&self, key: gdk::Key, ctrl: bool) -> bool {
        use gdk::Key;
        let bar = self.with(|s| s.text.bar);
        if let Some(Bar::Highlight(_)) = bar {
            match key {
                Key::d | Key::Delete => self.act(Action::Delete),
                Key::c => self.act(Action::Copy),
                Key::Escape => self.dismiss(),
                _ => {
                    self.dismiss();
                    return false;
                }
            }
            return true;
        }
        if !self.with(|s| s.text.selection.is_some()) {
            return false;
        }
        match key {
            Key::c if ctrl => self.act(Action::Copy),
            _ if ctrl => {}
            Key::h | Key::Left => self.move_cursor(Pending::Step(-1)),
            Key::l | Key::Right => self.move_cursor(Pending::Step(1)),
            Key::j | Key::Down => self.move_cursor(Pending::Line(1, f32::NAN)),
            Key::k | Key::Up => self.move_cursor(Pending::Line(-1, f32::NAN)),
            Key::v => self.with(|s| {
                if let Some(sel) = &mut s.text.selection {
                    sel.keyboard = true;
                    sel.anchor = if sel.anchor.is_some() { None } else { Some(sel.cursor) };
                    s.text.bar = sel.anchor.map(|_| Bar::Selection);
                    s.text.bar_since = None;
                }
            }),
            Key::o => self.with(|s| {
                if let Some(sel) = &mut s.text.selection
                    && let Some(anchor) = sel.anchor
                {
                    sel.anchor = Some(sel.cursor);
                    sel.cursor = anchor;
                    sel.keyboard = true;
                }
            }),
            Key::Return | Key::KP_Enter => self.act(Action::Highlight),
            Key::c => self.act(Action::Copy),
            Key::Escape => self.with(|s| {
                let sel = s.text.selection.as_mut().unwrap();
                if sel.anchor.is_some() && sel.keyboard {
                    sel.anchor = None;
                    s.text.bar = None;
                } else {
                    s.text.selection = None;
                    s.text.bar = None;
                    s.text.pending = None;
                }
            }),
            _ => {}
        }
        self.queue_draw();
        true
    }

    /// Moves the cursor; past the words shown it turns the page ("Slide") or scrolls a line ("Scroll") and goes on.
    fn move_cursor(&self, how: Pending) {
        let h = WidgetExt::height(self) as f32;
        let outside = self.with(|s| {
            let mut how = how;
            if let (Pending::Line(d, m), Some(sel)) = (how, s.text.selection)
                && m.is_nan()
            {
                // The cursor's middle, for the line it moves to.
                let words = Self::shown_words(s, h);
                let middle = selection::find(&words, sel.cursor.0).map_or(0.0, |i| words[i].rect.x + words[i].rect.w / 2.0);
                how = Pending::Line(d, middle);
            }
            s.text.pending = Some(how);
            s.text.selection.as_mut().unwrap().keyboard = true;
            Self::resolve_pending(s, h);
            s.text.pending.map(|p| match p {
                Pending::Step(d) | Pending::Line(d, _) => d,
            })
        });
        if let Some(dir) = outside {
            if self.mode() == Mode::Scroll {
                self.scroll_by(dir as f32 * self.line_height());
            } else {
                self.turn(dir);
                if self.with(|s| s.at_end) {
                    self.with(|s| s.text.pending = None);
                    self.turn(-dir);
                }
            }
        }
    }

    /// Moves the cursor as `pending` says when the words shown allow it; keeps it pending when they do not yet (the
    /// page or the column is still to come).
    pub(super) fn resolve_pending(s: &mut State, h: f32) {
        let (Some(pending), Some(sel)) = (s.text.pending, s.text.selection) else { return };
        let words = Self::shown_words(s, h);
        if words.is_empty() {
            return;
        }
        let here = words.iter().position(|w| w.start == sel.cursor.0);
        let next = match (pending, here) {
            (Pending::Step(d), Some(i)) => selection::step(&words, i, d),
            (Pending::Line(d, _), Some(i)) => selection::line_step(&words, i, d),
            // On a new page: its first (or last) word, or the one on its first (last) line nearest the middle.
            (Pending::Step(d), None) => Some(if d > 0 { 0 } else { words.len() - 1 }),
            (Pending::Line(d, middle), None) => {
                let line = if d > 0 { words[0].rect.y } else { words[words.len() - 1].rect.y };
                let distance = |w: &Word| (w.rect.x + w.rect.w / 2.0 - middle).abs();
                (0..words.len())
                    .filter(|&i| (words[i].rect.y - line).abs() < 0.5)
                    .min_by(|&a, &b| distance(&words[a]).total_cmp(&distance(&words[b])))
            }
        };
        if let Some(i) = next {
            // A hyphenated word moved onto from the right: its first piece.
            let i = words.iter().position(|w| w.start == words[i].start).unwrap_or(i);
            s.text.selection.as_mut().unwrap().cursor = (words[i].start, words[i].end);
            s.text.pending = None;
        }
    }

    // Acting.

    fn act(&self, action: Action) {
        let (bar, sel) = self.with(|s| (s.text.bar, s.text.selection));
        let range = match bar {
            Some(Bar::Highlight(q)) => Some(q),
            _ => sel.and_then(|s| s.range()),
        };
        let Some(range) = range else { return };
        match action {
            Action::Copy => self.copy(&self.text_of(range)),
            Action::Highlight => {
                self.with(|s| {
                    if !s.text.highlights.contains(&range) {
                        let i = s.text.highlights.partition_point(|q| *q < range);
                        s.text.highlights.insert(i, range);
                    }
                    s.text.reveal = Some((range, None));
                    Self::drop_pages(s);
                });
                self.saved_highlights();
                self.imp().frames.borrow_mut().push(Vec::new());
                self.add_tick_callback(|view, clock| {
                    view.queue_draw();
                    // Frame times for `--strokes`, as for page turns.
                    if let Some(frames) = view.imp().frames.borrow_mut().last_mut() {
                        frames.push(clock.frame_time());
                    }
                    if view.with(|s| s.text.reveal.is_some()) { glib::ControlFlow::Continue } else { glib::ControlFlow::Break }
                });
            }
            Action::Delete => {
                self.delete_highlight(range);
                return;
            }
        }
        self.dismiss();
    }

    /// `--strokes`: highlights the next paragraph shown that has no highlight yet.
    pub fn highlight_next(&self) {
        let h = WidgetExt::height(self) as f32;
        let next = self.with(|s| {
            let words = Self::shown_words(s, h);
            let w = words.iter().find(|w| !s.text.highlights.iter().any(|q| q.0 <= w.start && w.start < q.1))?;
            let (from, to) = s.book.block_range(w.start);
            s.text.selection = Some(Selection { cursor: (to, to), anchor: Some((from, from)), keyboard: false });
            Some(())
        });
        if next.is_some() {
            self.act(Action::Highlight);
        }
    }

    /// Ends the selection and closes the bar.
    pub fn dismiss(&self) {
        self.with(|s| {
            s.text.selection = None;
            s.text.bar = None;
            s.text.pending = None;
        });
        self.queue_draw();
    }

    // Drawing.

    /// The stroke of a new highlight under the page or column, at `now`; when it is done, the pages are baked with it.
    pub(super) fn draw_reveal(&self, s: &mut State, snapshot: &gtk::Snapshot, h: f32, now: i64) {
        let Some((range, _)) = s.text.reveal else { return };
        let rects = Self::shown_rects(s, h, range.0, range.1);
        let lines = rects.len().max(1) as i64;
        let duration = (lines * STROKE_LINE).clamp(STROKE_MIN, STROKE_MAX);
        let start = *s.text.reveal.as_mut().unwrap().1.get_or_insert(now);
        let p = ((now - start) as f32 / duration as f32).clamp(0.0, 1.0);
        let eased = if p < 0.5 { 4.0 * p * p * p } else { 1.0 - (-2.0 * p + 2.0).powi(3) / 2.0 };
        draw_bands(snapshot, &rects, &marker(s.theme), 0.0, eased);
        if p >= 1.0 {
            // Baked in idle time: the frame that drops the caches draws the same picture.
            let view = self.downgrade();
            glib::idle_add_local_once(move || {
                if let Some(view) = view.upgrade() {
                    view.with(|s| {
                        if s.text.reveal.is_some_and(|r| r.0 == range) {
                            s.text.reveal = None;
                            Self::drop_pages(s);
                        }
                    });
                    view.queue_draw();
                }
            });
        }
    }

    /// The selection over the page: bands in the ink at 14 %, the handles, the keyboard's cursor.
    pub(super) fn draw_selection(&self, s: &mut State, snapshot: &gtk::Snapshot, h: f32) {
        Self::resolve_pending(s, h);
        let Some(sel) = s.text.selection else { return };
        let link = rgba(THEMES[s.theme].link);
        if let Some((from, to)) = sel.range() {
            let rects = Self::shown_rects(s, h, from, to);
            Self::draw_marks(s, snapshot, &rects, 0.0);
            // Handles: a bar the line's height and a knob above the start, below the end (Android's).
            // Only the ends shown get a handle (a selection may go on over the page's edge).
            let words = Self::shown_words(s, h);
            if let Some(r) = rects.first().filter(|_| words.iter().any(|w| w.start <= from && from < w.end)) {
                handle(snapshot, &link, r.x, r.y, r.h, true);
            }
            if let Some(r) = rects.last().filter(|_| words.iter().any(|w| w.start < to && to <= w.end)) {
                handle(snapshot, &link, r.x + r.w, r.y, r.h, false);
            }
        }
        if sel.keyboard {
            let mut ink = link;
            ink.set_alpha(0.22);
            for r in Self::shown_rects(s, h, sel.cursor.0, sel.cursor.1) {
                let bounds = graphene::Rect::new(r.x - 3.0, r.y + 2.0, r.w + 6.0, r.h - 4.0);
                snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(bounds, 4.0));
                snapshot.append_color(&ink, &bounds);
                snapshot.pop();
            }
        }
    }

    /// The bar over the selection or a highlight: the ink as its fill, the paper as its labels (Android's `SelectionBar`).
    pub(super) fn draw_bar(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        let shown = self.with(|s| {
            s.text.bar_items.clear();
            let bar = s.text.bar?;
            let (from, to) = match bar {
                Bar::Selection => s.text.selection.and_then(|sel| sel.range())?,
                Bar::Highlight(q) => q,
            };
            if s.text.drag.as_ref().is_some_and(|d| d.started) {
                return None; // not while dragging, as on Android
            }
            let rects = Self::shown_rects(s, h, from, to);
            let bounds = rects.iter().copied().reduce(|a, b| {
                let (x, y) = (a.x.min(b.x), a.y.min(b.y));
                Rect { x, y, w: (a.x + a.w).max(b.x + b.w) - x, h: (a.y + a.h).max(b.y + b.h) - y }
            })?;
            let since = *s.text.bar_since.get_or_insert(now);
            let items: Vec<(&str, Action)> = match bar {
                Bar::Selection => vec![(tr("Highlight"), Action::Highlight), (tr("Copy"), Action::Copy)],
                Bar::Highlight(_) => vec![(tr("Delete"), Action::Delete), (tr("Copy"), Action::Copy)],
            };
            Some((bounds, items, THEMES[s.theme], ((now - since) as f32 / BAR_FADE as f32).clamp(0.0, 1.0)))
        });
        let Some((bounds, items, palette, alpha)) = shown else { return };
        if alpha < 1.0 {
            self.add_tick_callback(|view, _| {
                view.queue_draw();
                glib::ControlFlow::Break
            });
        }
        crate::fonts::add_interface_fonts();
        let font = gtk::pango::FontDescription::from_string("Golos Text Medium 15px");
        let layouts: Vec<gtk::pango::Layout> = items
            .iter()
            .map(|(label, _)| {
                let l = self.create_pango_layout(Some(label));
                l.set_font_description(Some(&font));
                l
            })
            .collect();
        let widths: Vec<f32> = layouts.iter().map(|l| l.pixel_size().0 as f32 + 32.0).collect();
        let bar_h = layouts.first().map_or(20.0, |l| l.pixel_size().1 as f32) + 24.0;
        let bar_w: f32 = widths.iter().sum::<f32>() + (items.len() - 1) as f32;
        let above = bounds.y - BAR_GAP - bar_h;
        let y = if above > 8.0 { above } else { bounds.y + bounds.h + BAR_GAP + 8.0 };
        let x = (bounds.x + bounds.w / 2.0 - bar_w / 2.0).clamp(8.0, (w - bar_w - 8.0).max(8.0));
        let rect = graphene::Rect::new(x, y, bar_w, bar_h);
        let mut fill = rgba(palette.ink);
        fill.set_alpha(alpha);
        let mut label = rgba(palette.paper);
        label.set_alpha(alpha);
        let mut rule = rgba(palette.paper);
        rule.set_alpha(0.2 * alpha);
        let clip = gsk::RoundedRect::from_rect(rect, 14.0);
        snapshot.append_outset_shadow(&clip, &gdk::RGBA::new(0.0, 0.0, 0.0, 0.18 * alpha), 0.0, 4.0, 0.0, 10.0);
        snapshot.push_rounded_clip(&clip);
        snapshot.append_color(&fill, &rect);
        snapshot.pop();
        let mut left = x;
        let mut hits = Vec::new();
        for (i, ((layout, width), (_, action))) in layouts.iter().zip(&widths).zip(&items).enumerate() {
            if i > 0 {
                snapshot.append_color(&rule, &graphene::Rect::new(left, y + (bar_h - 20.0) / 2.0, 1.0, 20.0));
                left += 1.0;
            }
            snapshot.save();
            snapshot.translate(&graphene::Point::new(left + 16.0, y + 12.0));
            snapshot.append_layout(layout, &label);
            snapshot.restore();
            hits.push((graphene::Rect::new(left, y, *width, bar_h), *action));
            left += width;
        }
        self.with(|s| s.text.bar_items = hits);
    }
}

/// A selection handle at `(x, y)` of a line `h` high: a 2 px bar and a knob above (`start`) or below it.
fn handle(snapshot: &gtk::Snapshot, colour: &gdk::RGBA, x: f32, y: f32, h: f32, start: bool) {
    const KNOB: f32 = 5.5;
    snapshot.append_color(colour, &graphene::Rect::new(x - 1.0, y, 2.0, h));
    let cy = if start { y - KNOB * 0.6 } else { y + h + KNOB * 0.6 };
    let knob = graphene::Rect::new(x - KNOB, cy - KNOB, 2.0 * KNOB, 2.0 * KNOB);
    snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(knob, KNOB));
    snapshot.append_color(colour, &knob);
    snapshot.pop();
}

/// A key's Latin letter in the first layout (so the selection keys work in a Russian layout, as the shortcuts do).
pub(crate) fn latin(key: gdk::Key, code: u32) -> gdk::Key {
    if key.to_unicode().is_none_or(|c| c.is_ascii()) {
        return key.to_lower();
    }
    gdk::Display::default()
        .and_then(|d| d.map_keycode(code))
        .and_then(|keys| keys.into_iter().find(|(k, _)| k.group() == 0 && k.level() == 0).map(|(_, v)| v))
        .unwrap_or(key)
        .to_lower()
}
