//! The "Scroll" mode of the page view: the book as one continuous column.
//!
//! The view keeps an offset into the current chapter's column (the column height at the window's top) and draws
//! tiles: slices of the column one view high, typeset by the core and cached as render nodes per chapter and tile.
//! The next chapter follows the current one after a gap; when the window's top passes into it, it becomes the current
//! chapter. Movements run on the frame clock and stop when they arrive, so nothing is drawn while the column is still.

use std::collections::HashMap;

use forzac_core::{Area, ChapterLayout, paginate};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, graphene, gsk};

use super::{Mode, PageView, State, THEMES, content_node, rgba, stops};

/// The longest a scroll movement takes, µs.
const MOTION: i64 = 150_000;

/// Gap between chapters in the column, in lines.
const CHAPTER_GAP: f32 = 4.0;

/// How fast a touchpad glide slows down: its velocity falls by e every 1/FRICTION seconds.
const FRICTION: f32 = 4.0;

/// A scroll movement from one offset to another; `start` is the frame time of its first frame.
pub(super) struct Motion {
    pub from: f32,
    pub to: f32,
    pub start: Option<i64>,
}

/// The scroll state of a view.
#[derive(Default)]
pub(super) struct Column {
    /// The height of the current chapter's column at the window's top.
    pub offset: f32,
    /// Tiles: render nodes and areas (tile coordinates) by chapter and tile index.
    pub tiles: HashMap<(usize, i64), (gsk::RenderNode, Vec<Area>)>,
    pub motion: Option<Motion>,
    /// A touchpad glide: velocity (pixels a second) and the time of its last frame.
    pub glide: Option<(f32, Option<i64>)>,
    /// The markers and links on screen (view coordinates), for `Tab` and clicks; computed when needed.
    pub visible: Vec<Area>,
    /// The offset is to be set from the reading position once the layout is ready.
    pub align: bool,
    /// A tick callback is running.
    pub ticking: bool,
    /// The column was just placed from the reading position.
    pub placed: bool,
}

fn gap(s: &State) -> f32 {
    s.typesetter.as_ref().map_or(30.0, |t| t.line_height) * CHAPTER_GAP
}

/// The layout of `chapter`: the current one, a neighbour laid out in the background, or (when `now`) laid out now.
fn layout(s: &mut State, chapter: usize, now: bool) -> Option<&ChapterLayout> {
    if s.layout.as_ref().is_some_and(|l| l.chapter == chapter) {
        return s.layout.as_ref();
    }
    if !now && !s.neighbours.iter().any(|l| l.chapter == chapter) {
        return None;
    }
    if !s.neighbours.iter().any(|l| l.chapter == chapter) {
        let (style, t) = (s.style?, s.typesetter.as_mut()?);
        if chapter >= s.book.chapters.len() {
            return None;
        }
        s.neighbours.push(paginate(&s.book, chapter, &style, t));
    }
    s.neighbours.iter().find(|l| l.chapter == chapter)
}

fn layout_of(s: &mut State, chapter: usize) -> Option<&ChapterLayout> {
    layout(s, chapter, true)
}

/// Makes `chapter` (a neighbour) the current chapter, keeping the old one as a neighbour.
fn make_current(s: &mut State, chapter: usize) {
    if layout_of(s, chapter).is_none() {
        return;
    }
    let i = s.neighbours.iter().position(|l| l.chapter == chapter).unwrap();
    let layout = s.neighbours.swap_remove(i);
    if let Some(old) = s.layout.replace(layout) {
        s.neighbours.push(old);
    }
    s.chapter = chapter;
    // Tiles of chapters left behind are dropped in idle time, not in the frame that crosses the boundary.
    let old: Vec<_> = s.column.tiles.extract_if(|(c, _), _| c.abs_diff(chapter) > 1).collect();
    if !old.is_empty() {
        glib::idle_add_local_once(move || drop(old));
    }
}

/// Moves to the chapter the window's top is in and keeps the offset inside the book; true when the chapter changed.
fn normalize(s: &mut State, height: f32) -> bool {
    let start = s.chapter;
    loop {
        let Some(length) = s.layout.as_ref().map(|l| l.flow_height() + gap(s)) else { return false };
        if s.column.offset >= length && s.chapter + 1 < s.book.chapters.len() {
            make_current(s, s.chapter + 1);
            shift(s, -length);
        } else if s.column.offset < 0.0 && s.chapter > 0 {
            let chapter = s.chapter - 1;
            let g = gap(s);
            let Some(previous) = layout_of(s, chapter).map(|l| l.flow_height() + g) else { return false };
            make_current(s, chapter);
            shift(s, previous);
        } else {
            break;
        }
    }
    // The ends of the book.
    let end = s.layout.as_ref().map_or(0.0, |l| (l.flow_height() - height).max(0.0));
    let last = s.chapter + 1 == s.book.chapters.len();
    let clamped = if last { s.column.offset.clamp(0.0, end) } else { s.column.offset.max(0.0) };
    if clamped != s.column.offset {
        s.column.offset = clamped;
        s.column.glide = None;
        if let Some(m) = &mut s.column.motion {
            m.to = m.to.clamp(if s.chapter == 0 { 0.0 } else { f32::MIN }, if last { end } else { f32::MAX });
        }
    }
    s.chapter != start
}

/// Moves the offset and a running movement by `by` (when the current chapter changes under them).
fn shift(s: &mut State, by: f32) {
    s.column.offset += by;
    if let Some(m) = &mut s.column.motion {
        m.from += by;
        m.to += by;
    }
}

impl PageView {
    /// Switches between "Slide" and "Scroll", keeping the line at the top of the text; returns the new mode.
    pub fn toggle_mode(&self) -> Mode {
        let mode = self.with(|s| {
            let margin = s.style.map_or(0.0, |st| st.margin_top);
            match s.mode {
                Mode::Slide => {
                    s.mode = Mode::Scroll;
                    s.column = Default::default();
                    if let Some(l) = &s.layout {
                        s.column.offset = l.flow_y(s.locator) - margin;
                    }
                }
                Mode::Scroll => {
                    s.mode = Mode::Slide;
                    if let Some(l) = &s.layout {
                        s.locator = l.flow_locator(s.column.offset + margin);
                        s.page = l.page_of(s.locator);
                    }
                    s.column = Default::default();
                }
            }
            s.turn = None;
            s.focus = None;
            s.mode
        });
        self.moved();
        self.queue_draw();
        mode
    }

    pub fn mode(&self) -> Mode {
        self.with(|s| s.mode)
    }

    /// Sets the mode before the view is shown.
    pub fn set_mode(&self, mode: Mode) {
        self.with(|s| {
            s.mode = mode;
            s.column.align = true;
        });
    }

    /// "Scroll": moves the column by `dy` pixels with a smooth movement; a movement running continues from where it is.
    pub(super) fn scroll_by(&self, dy: f32) {
        self.with(|s| {
            s.column.glide = None;
            let to = s.column.motion.as_ref().map_or(s.column.offset, |m| m.to) + dy;
            s.column.motion = Some(Motion { from: s.column.offset, to, start: None });
            s.focus = None;
        });
        self.imp().frames.borrow_mut().push(Vec::new());
        self.tick();
        self.queue_draw();
    }

    /// "Scroll": moves the column by `dy` pixels at once (touchpad).
    pub(super) fn scroll_pixels(&self, dy: f32) {
        let h = WidgetExt::height(self) as f32;
        self.with(|s| {
            s.column.motion = None;
            s.column.glide = None;
            s.column.offset += dy;
            s.focus = None;
            normalize(s, h);
        });
        self.settle();
        self.queue_draw();
    }

    /// "Scroll": glides on after a touchpad flick at `velocity` pixels a second.
    pub(super) fn glide(&self, velocity: f32) {
        self.with(|s| s.column.glide = Some((velocity, None)));
        self.imp().frames.borrow_mut().push(Vec::new());
        self.tick();
    }

    /// Runs the movement or glide on the frame clock until it ends.
    fn tick(&self) {
        if self.with(|s| std::mem::replace(&mut s.column.ticking, true)) {
            return;
        }
        self.add_tick_callback(|view, clock| {
            let now = clock.frame_time();
            if let Some(frames) = view.imp().frames.borrow_mut().last_mut() {
                frames.push(now);
            }
            if let Some(t) = clock.current_timings() {
                view.imp().refresh.set(t.refresh_interval());
            }
            let h = WidgetExt::height(view) as f32;
            // The first frame of a movement already moves (as if it had started a refresh earlier): a frame that draws
            // what is on screen presents nothing and the next one comes a refresh late.
            let refresh = clock.current_timings().map_or(8_333, |t| t.refresh_interval()).clamp(4_000, 17_000);
            let (running, chapter_changed) = view.with(|s| {
                if let Some(m) = &mut s.column.motion {
                    let start = *m.start.get_or_insert(now - refresh);
                    let p = ((now - start) as f32 / MOTION as f32).clamp(0.0, 1.0);
                    let eased = 1.0 - (1.0 - p).powi(3); // ease-out cubic
                    s.column.offset = m.from + (m.to - m.from) * eased;
                    if p >= 1.0 {
                        s.column.motion = None;
                    }
                } else if let Some((v, last)) = &mut s.column.glide {
                    let dt = last.map_or(0.0, |l| (now - l) as f32 / 1e6);
                    *last = Some(now);
                    s.column.offset += *v * dt;
                    *v *= (-FRICTION * dt).exp();
                    if v.abs() < 20.0 {
                        s.column.glide = None;
                    }
                }
                let changed = normalize(s, h);
                let running = s.column.motion.is_some() || s.column.glide.is_some();
                if !running {
                    s.column.ticking = false;
                }
                (running, changed)
            });
            view.queue_draw();
            if chapter_changed {
                // Out of the frame: the next background job starts in idle time.
                let v = view.downgrade();
                glib::idle_add_local_once(move || {
                    if let Some(v) = v.upgrade() {
                        v.prefetch();
                    }
                });
            }
            if running {
                glib::ControlFlow::Continue
            } else {
                view.settle();
                glib::ControlFlow::Break
            }
        });
    }

    /// The column has stopped: the reading position is its top line.
    fn settle(&self) {
        self.with(|s| {
            let margin = s.style.map_or(0.0, |st| st.margin_top);
            if let Some(l) = &s.layout {
                s.locator = l.flow_locator(s.column.offset + margin);
            }
        });
        self.moved();
        self.prepare();
    }

    /// Builds the tiles a screen above and below in idle time, so the next movement only draws.
    pub(super) fn prepare(&self) {
        let view = self.downgrade();
        glib::idle_add_local_once(move || {
            if let Some(view) = view.upgrade() {
                let (w, h) = (WidgetExt::width(&view) as f32, WidgetExt::height(&view) as f32);
                let fresh = view.with(|s| {
                    let mut fresh = Vec::new();
                    if s.column.motion.is_none() && s.column.glide.is_none() {
                        let before: std::collections::HashSet<(usize, i64)> = s.column.tiles.keys().copied().collect();
                        for_tiles(s, -h, 2.0 * h, h, true, |_, _, _| {});
                        fresh = s.column.tiles.iter().filter(|(k, _)| !before.contains(k)).map(|(_, (n, _))| n.clone()).collect();
                    }
                    fresh
                });
                // Drawn once off screen: the renderer rasterises their glyphs now, not in the frame that shows them.
                if let Some(renderer) = view.native().and_then(|n| n.renderer()) {
                    let scale = view.scale_factor() as f32;
                    for node in fresh {
                        let scaled = gsk::TransformNode::new(&node, Some(&gsk::Transform::new().scale(scale, scale)));
                        let _ = renderer.render_texture(&scaled, Some(&graphene::Rect::new(0.0, 0.0, w * scale, h * scale)));
                    }
                }
            }
        });
    }

    /// True once after the column was placed (a first display, a jump): the tiles around are then prepared.
    pub(super) fn placed(s: &mut State) -> bool {
        std::mem::take(&mut s.column.placed)
    }

    /// The markers and links on screen, in view coordinates (for `Tab` and clicks).
    pub(super) fn visible_areas(s: &mut State, h: f32) -> &[Area] {
        let mut visible = Vec::new();
        for_tiles(s, 0.0, h, h, true, |y, _, areas| {
            visible.extend(
                areas
                    .iter()
                    .map(|a| Area { rect: forzac_core::Rect { y: a.rect.y + y, ..a.rect }, target: a.target.clone() })
                    .filter(|a| a.rect.y + a.rect.h > 0.0 && a.rect.y < h),
            );
        });
        s.column.visible = visible;
        &s.column.visible
    }

    /// Draws the column: the tiles of the current chapter and of the following ones that reach into the window.
    pub(super) fn draw_column(s: &mut State, snapshot: &gtk::Snapshot, h: f32) {
        let (Some(style), true) = (s.style, s.layout.is_some()) else { return };
        if std::mem::take(&mut s.column.align) {
            let at = s.locator;
            if let Some(l) = &s.layout {
                s.column.offset = l.flow_y(at) - style.margin_top;
            }
            s.column.placed = true;
        }
        normalize(s, h);
        // While the column moves, a chapter not laid out yet is left for the background (drawn when it arrives).
        let still = s.column.motion.is_none() && s.column.glide.is_none();
        for_tiles(s, 0.0, h, h, still, |y, node, _| {
            snapshot.save();
            snapshot.translate(&graphene::Point::new(0.0, y));
            snapshot.append_node(node);
            snapshot.restore();
        });
        // The focused marker or link.
        if s.focus.is_some() {
            let areas = Self::visible_areas(s, h).to_vec();
            if let Some((first, last)) = s.focus.and_then(|f| stops(&areas).get(f).copied()) {
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

    /// "Scroll": jumps to the top line or the end of the current chapter.
    pub(super) fn column_edge(&self, end: bool) {
        let h = WidgetExt::height(self) as f32;
        self.with(|s| {
            s.column.motion = None;
            s.column.glide = None;
            s.column.offset = if end { s.layout.as_ref().map_or(0.0, |l| (l.flow_height() - h).max(0.0)) } else { 0.0 };
            s.focus = None;
        });
        self.settle();
        self.queue_draw();
    }

    /// "Scroll": sets the offset from the reading position (after a jump).
    pub(super) fn align_column(s: &mut State) {
        s.column.motion = None;
        s.column.glide = None;
        s.column.align = true;
    }
}

/// Calls `f(y, node, areas)` for every tile reaching into the window's range `[from, to)` (window coordinates; the
/// window is `h` high and the tiles are as high), building tiles that are not cached; `y` is the tile's top on screen.
/// A chapter not laid out yet is laid out now only when `now`.
fn for_tiles(s: &mut State, from: f32, to: f32, h: f32, now: bool, mut f: impl FnMut(f32, &gsk::RenderNode, &[Area])) {
    let tile = h.max(1.0).round();
    let mut chapter = s.chapter;
    // Where this chapter's column starts on screen, on a whole pixel: text moved by fractions of a pixel would have its
    // glyphs rasterised again for every fraction, in the frames of a movement.
    let mut top = -s.column.offset.round();
    while top < to && chapter < s.book.chapters.len() {
        let Some(length) = layout(s, chapter, now).map(|l| l.flow_height()) else { break };
        // From one tile above the range: its last line may hang into it.
        let first = ((from - top) / tile).floor() as i64 - 1;
        let last = ((to - top) / tile).floor() as i64;
        for k in first.max(0)..=last {
            if k as f32 * tile >= length {
                break;
            }
            if build_tile(s, chapter, k, tile)
                && let Some((node, areas)) = s.column.tiles.get(&(chapter, k))
            {
                f(top + k as f32 * tile, node, areas);
            }
        }
        top += length + gap(s);
        chapter += 1;
    }
}

/// Builds tile `k` of `chapter`'s column unless cached; false when it cannot be built.
fn build_tile(s: &mut State, chapter: usize, k: i64, tile: f32) -> bool {
    if s.column.tiles.contains_key(&(chapter, k)) {
        return true;
    }
    let Some(style) = s.style else { return false };
    let Some(content) = layout_of(s, chapter).map(|l| l.render_flow(k as f32 * tile, (k + 1) as f32 * tile, &style)) else { return false };
    let node =
        content_node(&content, s.theme, &mut s.fonts, &mut s.textures, &s.book).unwrap_or_else(|| gsk::ContainerNode::new(&[]).upcast());
    s.column.tiles.insert((chapter, k), (node, content.areas));
    true
}
