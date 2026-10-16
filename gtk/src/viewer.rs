//! The view of an image or a table filling the window (`viewer`): opened from the page (`Tab` and `Enter`, or a click),
//! it zooms (`+`, `−`, `0`, `Ctrl`+wheel, a pinch) and pans (arrows, `h`/`j`/`k`/`l`, the wheel, the touchpad, a drag)
//! within the edges, and closes (`Esc`, `q`, the close button) back to the unchanged page. It draws only while something
//! moves: an image is the page's texture scaled, a table one render node typeset at the view's size and moved.

use std::cell::{Cell, RefCell};

use forzac_core::{Area, Locator, Target};
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, prelude::*};

use crate::icons::{self, Icon};
use crate::page_view::{PageView, THEMES, draw_focus, rgba, stops};
use crate::panels::shortcut;

/// Zoom factor of a key press or a wheel step; the limits for images (times the fitted size) and tables (times the
/// text size, in steps of 5 %).
const STEP: f32 = 1.25;
const IMAGE_ZOOM: (f32, f32) = (1.0, 5.0);
const TABLE_ZOOM: (f32, f32) = (0.5, 3.0);
/// Key movements and image zooms; the fades in and out, as on Android (µs).
const MOVE: i64 = 150_000;
const FADE_IN: i64 = 200_000;
const FADE_OUT: i64 = 150_000;
/// Space around a table: at the sides and below, and above (under the close button).
const PAD: f32 = 24.0;
const PAD_TOP: f32 = 56.0;

/// What the view shows.
#[derive(Clone)]
pub enum Subject {
    Image(String),
    /// A table, by its first row.
    Table(Locator),
}

/// A table typeset for the view at one zoom step and theme.
struct Table {
    node: gsk::RenderNode,
    size: (f32, f32),
    areas: Vec<Area>,
    zoom: f32,
    theme: usize,
}

/// A movement of the content's corner (and, for an image, its zoom) on the frame clock.
struct Motion {
    from: ([f32; 2], f32),
    to: ([f32; 2], f32),
    start: Option<i64>,
}

pub struct State {
    page: PageView,
    subject: Subject,
    texture: Option<gdk::Texture>,
    table: Option<Table>,
    /// An image: times the fitted size; a table: times the text size.
    zoom: f32,
    /// The content's top left corner in the view.
    pos: [f32; 2],
    /// The view's size when the content was placed; (0, 0) before the first frame.
    size: (f32, f32),
    motion: Option<Motion>,
    /// The focused marker or link of a table (`Tab`), an index into its stops.
    focus: Option<usize>,
    /// The corner when a drag began, the zoom when a pinch began, where the pointer is.
    drag: [f32; 2],
    pinch: f32,
    pointer: [f32; 2],
}

impl State {
    fn image(&self) -> bool {
        matches!(self.subject, Subject::Image(_))
    }

    fn limits(&self) -> (f32, f32) {
        if self.image() { IMAGE_ZOOM } else { TABLE_ZOOM }
    }

    /// Left, top, right and bottom space kept around the content.
    fn padding(&self) -> [f32; 4] {
        if self.image() { [0.0; 4] } else { [PAD, PAD_TOP, PAD, PAD] }
    }

    /// The content's size at `zoom`: an image fitted to the view, then zoomed; a table as typeset (scaled for a zoom it
    /// has not been typeset at yet).
    fn content(&self, zoom: f32) -> (f32, f32) {
        let (w, h) = self.size;
        if let Some(t) = &self.texture {
            let (iw, ih) = (t.width().max(1) as f32, t.height().max(1) as f32);
            let fit = (w / iw).min(h / ih);
            (iw * fit * zoom, ih * fit * zoom)
        } else if let Some(t) = &self.table {
            (t.size.0 * zoom / t.zoom, t.size.1 * zoom / t.zoom)
        } else {
            (0.0, 0.0)
        }
    }

    /// `pos` for content of `size` kept within the edges: centred in a direction where it is smaller than the view (less
    /// the padding), else no gap between its edge and the padding.
    fn clamp(&self, pos: [f32; 2], size: (f32, f32)) -> [f32; 2] {
        let [l, t, r, b] = self.padding();
        let axis = |p: f32, c: f32, view: f32, lo: f32, hi: f32| {
            if c + lo + hi <= view { lo + (view - lo - hi - c) / 2.0 } else { p.clamp(view - hi - c, lo) }
        };
        [axis(pos[0], size.0, self.size.0, l, r), axis(pos[1], size.1, self.size.1, t, b)]
    }

    /// Where a movement in progress is going, else where the content is.
    fn target(&self) -> ([f32; 2], f32) {
        self.motion.as_ref().map_or((self.pos, self.zoom), |m| m.to)
    }

    /// The content's corner and zoom at frame time `now`, ending the movement when it is over.
    fn at(&mut self, now: i64) -> ([f32; 2], f32) {
        let Some(m) = &mut self.motion else { return (self.pos, self.zoom) };
        let p = ((now - *m.start.get_or_insert(now)) as f32 / MOVE as f32).clamp(0.0, 1.0);
        if p >= 1.0 {
            (self.pos, self.zoom) = m.to;
            self.motion = None;
            return (self.pos, self.zoom);
        }
        let e = 1.0 - (1.0 - p).powi(3); // ease-out cubic
        let mix = |a: f32, b: f32| a + (b - a) * e;
        let ((from, fz), (to, tz)) = (m.from, m.to);
        ([mix(from[0], to[0]), mix(from[1], to[1])], mix(fz, tz))
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Canvas {
        pub state: RefCell<Option<State>>,
        /// Set when the view starts closing: it no longer acts.
        pub closing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Canvas {
        const NAME: &'static str = "ForzacViewer";
        type Type = super::Canvas;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Canvas {}

    impl WidgetImpl for Canvas {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.obj().draw(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct Canvas(ObjectSubclass<imp::Canvas>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

/// Opens `subject` over the page in `overlay`; `None` when it cannot be shown (an image that cannot be decoded).
/// The view is the returned widget, until it closes.
pub fn open(overlay: &gtk::Overlay, page: &PageView, subject: Subject) -> Option<Canvas> {
    let (texture, table) = match &subject {
        Subject::Image(id) => (Some(page.texture(id)?), None),
        Subject::Table(at) => {
            let (node, w, h, areas) = page.table_view(*at, 1.0)?;
            (None, Some(Table { node, size: (w, h), areas, zoom: 1.0, theme: page.theme() }))
        }
    };
    let canvas: Canvas = glib::Object::new();
    *canvas.imp().state.borrow_mut() = Some(State {
        page: page.clone(),
        subject,
        texture,
        table,
        zoom: 1.0,
        pos: [0.0; 2],
        size: (0.0, 0.0),
        motion: None,
        focus: None,
        drag: [0.0; 2],
        pinch: 1.0,
        pointer: [0.0; 2],
    });
    canvas.set_focusable(true);
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    canvas.controllers();

    // The close button: the panels' "×" on a circle of the sheet colour, over the top right corner.
    let close = gtk::Button::new();
    close.set_child(Some(&Icon::new(icons::X, 22)));
    close.set_size_request(44, 44);
    close.set_focusable(false);
    close.add_css_class("viewer-close");
    let corner = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    corner.add_css_class("forzac-panel");
    corner.set_halign(gtk::Align::End);
    corner.set_valign(gtk::Align::Start);
    corner.set_margin_top(8);
    corner.set_margin_end(8);
    corner.append(&close);
    let c = canvas.downgrade();
    close.connect_clicked(move |_| {
        if let Some(c) = c.upgrade() {
            c.close();
        }
    });

    let root = gtk::Overlay::new();
    root.set_child(Some(&canvas));
    root.add_overlay(&corner);
    root.set_opacity(0.0);
    overlay.add_overlay(&root);
    canvas.grab_focus();
    fade(root.upcast_ref(), true, |_| {});
    Some(canvas)
}

/// Fades `widget` in or out on the frame clock, then calls `done`.
fn fade(widget: &gtk::Widget, show: bool, done: impl FnOnce(&gtk::Widget) + 'static) {
    let duration = if show { FADE_IN } else { FADE_OUT };
    let start = Cell::new(None);
    let done = RefCell::new(Some(done));
    widget.add_tick_callback(move |w, clock| {
        let now = clock.frame_time();
        let begun = start.get().unwrap_or(now);
        start.set(Some(begun));
        let t = ((now - begun) as f64 / duration as f64).min(1.0);
        w.set_opacity(if show { t } else { 1.0 - t });
        if t < 1.0 {
            return glib::ControlFlow::Continue;
        }
        if let Some(f) = done.borrow_mut().take() {
            f(w);
        }
        glib::ControlFlow::Break
    });
}

impl Canvas {
    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(self.imp().state.borrow_mut().as_mut().unwrap())
    }

    /// Fades the view out and removes it; the page gets the focus back.
    pub fn close(&self) {
        if self.imp().closing.replace(true) {
            return;
        }
        let page = self.with(|s| s.page.clone());
        page.grab_focus();
        let Some(root) = self.parent() else { return };
        root.set_can_target(false);
        fade(&root, false, |root| {
            if let Some(overlay) = root.parent().and_then(|p| p.downcast::<gtk::Overlay>().ok()) {
                overlay.remove_overlay(root);
            }
        });
    }

    fn controllers(&self) {
        let keys = gtk::ShortcutController::new();
        let v = self.downgrade();
        let on = move |f: fn(&Canvas)| {
            let v = v.clone();
            move || {
                if let Some(v) = v.upgrade().filter(|v| !v.imp().closing.get()) {
                    f(&v);
                }
            }
        };
        shortcut(&keys, "Escape|q", on(|v| v.close()));
        shortcut(&keys, "plus|equal|KP_Add", on(|v| v.zoom_by(STEP, None)));
        shortcut(&keys, "minus|KP_Subtract", on(|v| v.zoom_by(1.0 / STEP, None)));
        shortcut(&keys, "0|KP_0", on(|v| v.fit(true)));
        shortcut(&keys, "h|Left", on(|v| v.pan_part(-0.125, 0.0)));
        shortcut(&keys, "l|Right", on(|v| v.pan_part(0.125, 0.0)));
        shortcut(&keys, "k|Up", on(|v| v.pan_part(0.0, -0.125)));
        shortcut(&keys, "j|Down", on(|v| v.pan_part(0.0, 0.125)));
        shortcut(&keys, "space|Page_Down", on(|v| v.pan_part(0.0, 0.9)));
        shortcut(&keys, "<Shift>space|Page_Up", on(|v| v.pan_part(0.0, -0.9)));
        shortcut(&keys, "g|Home", on(|v| v.pan(0.0, f32::NEG_INFINITY, true)));
        shortcut(&keys, "<Shift>g|End", on(|v| v.pan(0.0, f32::INFINITY, true)));
        shortcut(&keys, "Tab", on(|v| v.focus_next(1)));
        shortcut(&keys, "<Shift>Tab|<Shift>ISO_Left_Tab", on(|v| v.focus_next(-1)));
        shortcut(&keys, "Return|KP_Enter", on(|v| v.open_focused()));
        self.add_controller(keys);

        let motion = gtk::EventControllerMotion::new();
        let v = self.downgrade();
        motion.connect_motion(move |_, x, y| {
            if let Some(v) = v.upgrade() {
                v.with(|s| s.pointer = [x as f32, y as f32]);
            }
        });
        self.add_controller(motion);

        // A click on a marker or link in a table opens it; a click on an image closes the view, as a tap on Android.
        let click = gtk::GestureClick::new();
        let v = self.downgrade();
        click.connect_released(move |_, _, x, y| {
            if let Some(v) = v.upgrade().filter(|v| !v.imp().closing.get()) {
                v.click(x as f32, y as f32);
            }
        });
        self.add_controller(click);

        let drag = gtk::GestureDrag::new();
        let v = self.downgrade();
        drag.connect_drag_begin(move |_, _, _| {
            if let Some(v) = v.upgrade() {
                v.with(|s| {
                    s.motion = None;
                    s.drag = s.pos;
                });
            }
        });
        let v = self.downgrade();
        drag.connect_drag_update(move |_, dx, dy| {
            if let Some(v) = v.upgrade() {
                v.with(|s| {
                    let size = s.content(s.zoom);
                    s.pos = s.clamp([s.drag[0] + dx as f32, s.drag[1] + dy as f32], size);
                });
                v.queue_draw();
            }
        });
        self.add_controller(drag);

        let pinch = gtk::GestureZoom::new();
        let v = self.downgrade();
        pinch.connect_begin(move |_, _| {
            if let Some(v) = v.upgrade() {
                v.with(|s| s.pinch = s.target().1);
            }
        });
        let v = self.downgrade();
        pinch.connect_scale_changed(move |g, scale| {
            if let Some(v) = v.upgrade() {
                let at = g.bounding_box_center().map(|(x, y)| [x as f32, y as f32]);
                let zoom = v.with(|s| s.pinch) * scale as f32;
                v.zoom_to(zoom, at, false);
            }
        });
        self.add_controller(pinch);

        // The wheel pans by three lines a step (sideways with Shift), the touchpad with the fingers; with Ctrl both zoom
        // around the pointer.
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        let v = self.downgrade();
        scroll.connect_scroll(move |c, dx, dy| {
            let Some(v) = v.upgrade() else { return glib::Propagation::Proceed };
            let state = c.current_event_state();
            let at = Some(v.with(|s| s.pointer));
            let (dx, dy) = (dx as f32, dy as f32);
            if c.unit() == gdk::ScrollUnit::Wheel {
                if state.contains(gdk::ModifierType::CONTROL_MASK) {
                    v.zoom_by(STEP.powf(-dy), at);
                } else {
                    let line = 3.0 * v.with(|s| s.page.line_height());
                    let (dx, dy) = if state.contains(gdk::ModifierType::SHIFT_MASK) && dx == 0.0 { (dy, 0.0) } else { (dx, dy) };
                    v.pan(dx * line, dy * line, true);
                }
            } else if state.contains(gdk::ModifierType::CONTROL_MASK) {
                let zoom = v.with(|s| s.target().1) * (-dy / 200.0).exp();
                v.zoom_to(zoom, at, false);
            } else {
                v.pan(dx, dy, false);
            }
            glib::Propagation::Stop
        });
        self.add_controller(scroll);
    }

    /// Moves the content to `pos` at `zoom` (kept within the edges), smoothly or at once.
    fn move_to(&self, pos: [f32; 2], zoom: f32, animate: bool) {
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        let moving = self.with(|s| {
            let to = (s.clamp(pos, s.content(zoom)), zoom);
            let from = s.at(now);
            if animate && (from.0 != to.0 || from.1 != to.1) {
                s.motion = Some(Motion { from, to, start: None });
                true
            } else {
                (s.pos, s.zoom) = to;
                s.motion = None;
                false
            }
        });
        if moving {
            self.add_tick_callback(|v, _| {
                v.queue_draw();
                if v.with(|s| s.motion.is_some()) { glib::ControlFlow::Continue } else { glib::ControlFlow::Break }
            });
        }
        self.queue_draw();
    }

    /// Moves the view over the content by `dx`, `dy` (the content the other way).
    fn pan(&self, dx: f32, dy: f32, animate: bool) {
        let (pos, zoom) = self.with(|s| s.target());
        // An infinite step goes to the edge: the clamp stops it there.
        let far = |p: f32, d: f32| if d.is_infinite() { -d.signum() * 1e9 } else { p - d };
        self.move_to([far(pos[0], dx), far(pos[1], dy)], zoom, animate);
    }

    /// Pans by a share of the view's width and height, smoothly.
    fn pan_part(&self, x: f32, y: f32) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        self.pan(x * w, y * h, true);
    }

    /// Zooms by `factor` around `at` (the view's centre when `None`); keys move smoothly.
    fn zoom_by(&self, factor: f32, at: Option<[f32; 2]>) {
        let zoom = self.with(|s| s.target().1) * factor;
        self.zoom_to(zoom, at, at.is_none());
    }

    /// Zooms to `zoom` (within the limits; a table in 5 % steps, typeset again) keeping the content's point at `at`.
    fn zoom_to(&self, zoom: f32, at: Option<[f32; 2]>, animate: bool) {
        let (pos, old, image, (lo, hi)) = self.with(|s| (s.target().0, s.target().1, s.image(), s.limits()));
        let zoom = if image { zoom.clamp(lo, hi) } else { ((zoom * 20.0).round() / 20.0).clamp(lo, hi) };
        if zoom == old {
            return;
        }
        let at = at.unwrap_or([WidgetExt::width(self) as f32 / 2.0, WidgetExt::height(self) as f32 / 2.0]);
        let (before, after) = self.with(|s| (s.content(old), s.content(zoom)));
        if !image {
            self.typeset(zoom);
        }
        let after = if image { after } else { self.with(|s| s.content(zoom)) };
        let k = after.0 / before.0.max(1.0);
        let pos = [at[0] - (at[0] - pos[0]) * k, at[1] - (at[1] - pos[1]) * k];
        // A table typeset at a new size cannot morph from the old one: it moves at once.
        self.move_to(pos, zoom, animate && image);
    }

    /// The whole image fitted and centred again; a table at the text size from its top left corner.
    fn fit(&self, animate: bool) {
        let image = self.with(|s| s.image());
        if !image {
            self.typeset(1.0);
        }
        self.move_to([PAD, PAD_TOP], 1.0, animate && image);
    }

    /// Typesets the table at `zoom` in the page's theme, unless it is already.
    fn typeset(&self, zoom: f32) {
        let (page, at, theme, done) = self.with(|s| {
            let at = if let Subject::Table(at) = s.subject { at } else { Locator::default() };
            let theme = s.page.theme();
            (s.page.clone(), at, theme, s.table.as_ref().is_some_and(|t| t.zoom == zoom && t.theme == theme))
        });
        if done {
            return;
        }
        if let Some((node, w, h, areas)) = page.table_view(at, zoom) {
            self.with(|s| s.table = Some(Table { node, size: (w, h), areas, zoom, theme }));
        }
    }

    /// `Tab` in a table: the next or previous marker or link, panned into the view.
    fn focus_next(&self, delta: i32) {
        let stop = self.with(|s| {
            let areas = &s.table.as_ref()?.areas;
            let all = stops(areas);
            if all.is_empty() {
                return None;
            }
            let count = all.len() as i32;
            let f = match s.focus {
                Some(f) => (f as i32 + delta).rem_euclid(count) as usize,
                None if delta > 0 => 0,
                None => all.len() - 1,
            };
            s.focus = Some(f);
            let (first, last) = all[f];
            areas[first..=last].iter().map(|a| a.rect).reduce(|a, b| forzac_core::Rect {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
                w: (a.x + a.w).max(b.x + b.w) - a.x.min(b.x),
                h: (a.y + a.h).max(b.y + b.h) - a.y.min(b.y),
            })
        });
        let Some(r) = stop else { return };
        // Pan the least that brings the stop inside the view, less the padding.
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        let (pos, _) = self.with(|s| s.target());
        let into = |start: f32, len: f32, lo: f32, hi: f32| {
            if start < lo {
                start - lo
            } else if start + len > hi {
                (start + len - hi).min(start - lo)
            } else {
                0.0
            }
        };
        let dx = into(pos[0] + r.x, r.w, PAD, w - PAD);
        let dy = into(pos[1] + r.y, r.h, PAD_TOP, h - PAD);
        self.pan(dx, dy, true);
    }

    fn open_focused(&self) {
        let target = self.with(|s| {
            let areas = &s.table.as_ref()?.areas;
            let (first, _) = *stops(areas).get(s.focus?)?;
            Some(areas[first].target.clone())
        });
        if let Some(t) = target {
            self.activate(t);
        }
    }

    fn click(&self, x: f32, y: f32) {
        if self.with(|s| s.image()) {
            self.close();
            return;
        }
        let target = self.with(|s| {
            let [px, py] = s.pos;
            let hit = |a: &&Area| {
                let r = a.rect;
                x >= px + r.x - 4.0 && x <= px + r.x + r.w + 4.0 && y >= py + r.y - 4.0 && y <= py + r.y + r.h + 4.0
            };
            s.table.as_ref()?.areas.iter().find(hit).map(|a| a.target.clone())
        });
        if let Some(t) = target {
            self.activate(t);
        }
    }

    /// Opens a marker or link of the table: a note or a short target in the sheet over the view, a place in the book
    /// after closing the view.
    fn activate(&self, target: Target) {
        let page = self.with(|s| s.page.clone());
        if page.jumps(&target) {
            self.close();
        }
        page.open(target);
    }

    fn draw(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (WidgetExt::width(self) as f32, WidgetExt::height(self) as f32);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let now = WidgetExt::frame_clock(self).map_or(0, |c| c.frame_time());
        let image = self.with(|s| s.image());
        if !image {
            // A theme changed while the view is open.
            let zoom = self.with(|s| s.table.as_ref().map_or(1.0, |t| t.zoom));
            self.typeset(zoom);
        }
        let first = self.with(|s| s.size == (0.0, 0.0));
        let resized = self.with(|s| {
            let old = s.size;
            if old == (w, h) {
                return false;
            }
            // A new size keeps the content's point at the view's centre.
            let size = s.content(s.zoom);
            let centre = [(old.0 / 2.0 - s.pos[0]) / size.0.max(1.0), (old.1 / 2.0 - s.pos[1]) / size.1.max(1.0)];
            s.size = (w, h);
            s.motion = None;
            let size = s.content(s.zoom);
            s.pos = s.clamp([w / 2.0 - centre[0] * size.0, h / 2.0 - centre[1] * size.1], size);
            true
        });
        if first {
            self.fit(false);
        } else if resized {
            self.queue_draw();
        }
        self.with(|s| {
            let theme = s.page.theme();
            snapshot.append_color(&rgba(THEMES[theme].paper), &graphene::Rect::new(0.0, 0.0, w, h));
            let (pos, zoom) = s.at(now);
            if let Some(texture) = &s.texture {
                let size = s.content(zoom);
                let filter = if size.0 > texture.width() as f32 { gsk::ScalingFilter::Linear } else { gsk::ScalingFilter::Trilinear };
                snapshot.append_scaled_texture(texture, filter, &graphene::Rect::new(pos[0], pos[1], size.0, size.1));
            } else if let Some(t) = &s.table {
                snapshot.save();
                snapshot.translate(&graphene::Point::new(pos[0], pos[1]));
                snapshot.append_node(&t.node);
                if let Some(stop) = s.focus.and_then(|f| stops(&t.areas).get(f).copied()) {
                    draw_focus(snapshot, theme, &t.areas, stop);
                }
                snapshot.restore();
            }
        });
    }
}
