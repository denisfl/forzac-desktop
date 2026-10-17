//! The reading menu over the page, as the web reader's (`#bar-top`, `#bar-bottom`) and Android's `ReaderMenu`: at the
//! top back to the library, the section and the book, the contents; at the bottom a slider over the whole book with
//! the chapters' ticks, "Page N of M", the time left in the chapter and "Aa". A click in the middle third of the page or
//! `Enter` opens it; a click on the page or `Esc` closes it; `←`/`→`/`h`/`l` move the slider and `Enter` goes there.
//! The bars fade and slide in (CSS transitions); the page under them is not drawn again.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use forzac_core::i18n::{fill, tr};
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, prelude::*};

use crate::icons::{self, Icon};
use crate::page_view::{PageView, THEMES, rgba};

/// How long the bars take to fade out (the CSS transition), after which they are hidden.
const CLOSE: Duration = Duration::from_millis(150);

/// The widest the bottom bar's content gets, and its least side margin (the web reader's `.bar-bottom`).
const CONTENT_WIDTH: i32 = 640;
const SIDE: i32 = 20;

/// What the menu's buttons do, given by the window.
pub struct Actions {
    pub back: Box<dyn Fn()>,
    pub contents: Box<dyn Fn()>,
    pub settings: Box<dyn Fn()>,
}

pub struct Menu {
    top: gtk::Box,
    bottom: gtk::Box,
    content: gtk::Box,
    section: gtk::Label,
    book: gtk::Label,
    slider: Slider,
    page: gtk::Label,
    hint: gtk::Label,
    /// The book page the slider shows while the reader moves it; `None`: the reading position's.
    scrub: Cell<Option<usize>>,
    open: Rc<Cell<bool>>,
    /// Hides the bars after they faded out; `None` once it fired.
    hide: Rc<RefCell<Option<glib::SourceId>>>,
    actions: RefCell<Option<Actions>>,
    view: glib::WeakRef<PageView>,
}

/// A 48 px round button with a Lucide icon and a tooltip.
fn icon_button(paths: icons::Paths, tooltip: &str) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(&Icon::new(paths, 24)));
    b.add_css_class("bar-icon");
    b.set_tooltip_text(Some(tooltip));
    b.set_focusable(false);
    b
}

fn label(classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::new(None);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    l.set_single_line_mode(true);
    for c in classes {
        l.add_css_class(c);
    }
    l
}

impl Menu {
    pub fn new(overlay: &gtk::Overlay, view: &PageView) -> Rc<Self> {
        crate::fonts::add_interface_fonts();
        // The top bar: back, the section above the book, the contents.
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        top.add_css_class("forzac-panel");
        top.add_css_class("reader-bar");
        top.add_css_class("bar-top");
        top.set_valign(gtk::Align::Start);
        let back = icon_button(icons::CHEVRON_LEFT, tr("Back to library"));
        let titles = gtk::Box::new(gtk::Orientation::Vertical, 0);
        titles.set_hexpand(true);
        titles.set_valign(gtk::Align::Center);
        titles.set_margin_start(8);
        titles.set_margin_end(8);
        let section = label(&["bar-section"]);
        let book = label(&["bar-book", "muted"]);
        titles.append(&section);
        titles.append(&book);
        let contents = icon_button(icons::LIST, tr("Contents"));
        top.append(&back);
        top.append(&titles);
        top.append(&contents);

        // The bottom bar: the slider, then the page and the hint beside "Aa".
        let bottom = gtk::Box::new(gtk::Orientation::Vertical, 0);
        bottom.add_css_class("forzac-panel");
        bottom.add_css_class("reader-bar");
        bottom.add_css_class("bar-bottom");
        bottom.set_valign(gtk::Align::End);
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let slider = Slider::new();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let info = gtk::Box::new(gtk::Orientation::Vertical, 3);
        info.set_hexpand(true);
        info.set_valign(gtk::Align::Center);
        let page = label(&["bar-page"]);
        let hint = label(&["bar-hint", "muted"]);
        page.set_xalign(0.0);
        hint.set_xalign(0.0);
        info.append(&page);
        info.append(&hint);
        let aa = gtk::Button::with_label("Aa");
        aa.add_css_class("bar-aa");
        aa.set_tooltip_text(Some(tr("Text settings")));
        aa.set_focusable(false);
        aa.set_valign(gtk::Align::Center);
        row.append(&info);
        row.append(&aa);
        content.append(&slider);
        content.append(&row);
        bottom.append(&content);

        for bar in [&top, &bottom] {
            bar.set_visible(false);
            overlay.add_overlay(bar);
        }
        let menu = Rc::new(Menu {
            top,
            bottom,
            content,
            section,
            book,
            slider,
            page,
            hint,
            scrub: Cell::new(None),
            open: Rc::new(Cell::new(false)),
            hide: Rc::default(),
            actions: RefCell::new(None),
            view: view.downgrade(),
        });
        let act = |f: fn(&Actions)| {
            let menu = Rc::downgrade(&menu);
            move |_: &gtk::Button| {
                if let Some(menu) = menu.upgrade()
                    && let Some(actions) = menu.actions.borrow().as_ref()
                {
                    f(actions);
                }
            }
        };
        back.connect_clicked(act(|a| (a.back)()));
        contents.connect_clicked(act(|a| (a.contents)()));
        aa.connect_clicked(act(|a| (a.settings)()));

        // Pressing on the slider or dragging the thumb shows that page; releasing goes there.
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

        // A count arriving while the menu is open shows the slider at once.
        let m = Rc::downgrade(&menu);
        view.connect_pages(move || {
            if let Some(m) = m.upgrade().filter(|m| m.open.get()) {
                m.show_place();
            }
        });
        menu
    }

    pub fn set_actions(&self, actions: Actions) {
        *self.actions.borrow_mut() = Some(actions);
    }

    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    pub fn open(&self) {
        let Some(view) = self.view.upgrade() else { return };
        if self.open.replace(true) {
            return;
        }
        view.dismiss();
        if let Some(id) = self.hide.borrow_mut().take() {
            id.remove();
        }
        self.scrub.set(None);
        // At most 640 px of content, centred, at least 20 px from the sides.
        let width = view.width();
        let side = SIDE.max((width - CONTENT_WIDTH) / 2);
        self.content.set_margin_start(side);
        self.content.set_margin_end(side);
        self.show_place();
        for bar in [&self.top, &self.bottom] {
            bar.set_visible(true);
        }
        // The transition needs a frame with the closed style first.
        let (top, bottom, open) = (self.top.clone(), self.bottom.clone(), self.open.clone());
        self.top.add_tick_callback(move |_, _| {
            if open.get() {
                top.add_css_class("open");
                bottom.add_css_class("open");
            }
            glib::ControlFlow::Break
        });
    }

    /// Closes the menu; false when it was not open.
    pub fn close(&self) -> bool {
        if !self.open.replace(false) {
            return false;
        }
        self.scrub.set(None);
        for bar in [&self.top, &self.bottom] {
            bar.remove_css_class("open");
        }
        let (top, bottom, hide) = (self.top.downgrade(), self.bottom.downgrade(), self.hide.clone());
        let id = glib::timeout_add_local_once(CLOSE, move || {
            hide.borrow_mut().take(); // fired: the id is no longer valid
            for bar in [top.upgrade(), bottom.upgrade()].into_iter().flatten() {
                bar.set_visible(false);
            }
        });
        if let Some(old) = self.hide.replace(Some(id)) {
            old.remove();
        }
        true
    }

    /// The keys while the menu is open: `←`/`h` and `→`/`l` move the slider a page, `Enter` goes to the page shown
    /// (or closes the menu when it is the reader's), `Esc` closes. True when the key was the menu's; any other key
    /// closes the menu and does what it does.
    pub fn key(&self, key: gdk::Key) -> bool {
        if !self.open.get() {
            return false;
        }
        match key {
            gdk::Key::Left | gdk::Key::h | gdk::Key::KP_Left => self.step(-1),
            gdk::Key::Right | gdk::Key::l | gdk::Key::KP_Right => self.step(1),
            gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::ISO_Enter => self.go(),
            gdk::Key::Escape => {
                self.close();
            }
            // Modifiers alone (Shift for `?`) wait for the key they modify.
            gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::ISO_Level3_Shift
            | gdk::Key::Caps_Lock => {}
            _ => {
                self.close();
                return false;
            }
        }
        true
    }

    /// The book page the reader is on, when the pages are counted.
    fn current(&self) -> Option<usize> {
        let view = self.view.upgrade()?;
        Some(view.book_pages()?.page_of(view.locator()))
    }

    fn step(&self, delta: i64) {
        let Some(pages) = self.view.upgrade().and_then(|v| v.book_pages()) else { return };
        let Some(from) = self.scrub.get().or_else(|| self.current()) else { return };
        let to = (from as i64 + delta).clamp(0, pages.total() as i64 - 1) as usize;
        self.scrub_to(to);
    }

    fn scrub_to(&self, page: usize) {
        if self.view.upgrade().and_then(|v| v.book_pages()).is_none() {
            return;
        }
        self.scrub.set(Some(page));
        self.show_place();
    }

    /// Goes to the page the slider shows (remembered for `Ctrl+O`) and closes the menu.
    fn go(&self) {
        let target = self.scrub.take().filter(|&p| Some(p) != self.current());
        let view = self.view.upgrade();
        self.close();
        if let (Some(view), Some(page)) = (view, target)
            && let Some(pages) = view.book_pages()
        {
            view.go_to(pages.start(page));
        }
    }

    /// The labels and the slider for the reading position, or for the page the slider is moved to.
    fn show_place(&self) {
        let Some(view) = self.view.upgrade() else { return };
        let book = view.book();
        let here = view.locator();
        self.section.set_text(&book.section_at(here));
        self.book.set_text(&book.title);
        let pages = view.book_pages();
        self.slider.set_visible(pages.is_some());
        self.page.set_visible(pages.is_some());
        if let Some(pages) = &pages {
            let shown = self.scrub.get().unwrap_or_else(|| pages.page_of(here));
            self.page.set_text(&fill(tr("Page {n} of {m}"), &[("n", &(shown + 1).to_string()), ("m", &pages.total().to_string())]));
            self.slider.set(pages.total(), shown, pages.chapter_starts(), view.theme());
        }
        let hint = match (self.scrub.get(), &pages) {
            (Some(page), Some(pages)) => book.section_at(pages.start(page)),
            _ => match book.minutes_left(here) {
                0 => tr("Less than a minute left in chapter").to_owned(),
                n => fill(tr("{n} min left in chapter"), &[("n", &n.to_string())]),
            },
        };
        self.hint.set_text(&hint);
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Slider {
        pub count: Cell<usize>,
        pub page: Cell<usize>,
        pub ticks: RefCell<Vec<usize>>,
        pub theme: Cell<usize>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Slider {
        const NAME: &'static str = "ForzacSlider";
        type Type = super::Slider;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Slider {}

    impl WidgetImpl for Slider {
        fn measure(&self, orientation: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Vertical { (28, 28, -1, -1) } else { (0, 0, -1, -1) }
        }

        /// As Android's `ProgressSlider`: a 2 px track in the line colour, a 1 × 8 px tick at each chapter's first
        /// page, the part read in the ink, a 16 px thumb.
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let (w, h) = (widget.width() as f32, widget.height() as f32);
            let palette = THEMES[self.theme.get()];
            let (radius, y) = (8.0, h / 2.0);
            let x_of =
                |p: usize| radius + if self.count.get() <= 1 { 0.0 } else { (w - 2.0 * radius) * p as f32 / (self.count.get() - 1) as f32 };
            snapshot.append_color(&rgba(palette.line), &graphene::Rect::new(radius, y - 1.0, w - 2.0 * radius, 2.0));
            let mut tick = rgba(palette.muted);
            tick.set_alpha(0.6);
            for &start in self.ticks.borrow().iter().filter(|&&s| s > 0) {
                snapshot.append_color(&tick, &graphene::Rect::new(x_of(start).round() - 0.5, y - 4.0, 1.0, 8.0));
            }
            let x = x_of(self.page.get());
            let ink = rgba(palette.ink);
            snapshot.append_color(&ink, &graphene::Rect::new(radius, y - 1.0, x - radius, 2.0));
            let thumb = graphene::Rect::new(x - radius, y - radius, 2.0 * radius, 2.0 * radius);
            snapshot.push_rounded_clip(&gsk::RoundedRect::from_rect(thumb, radius));
            snapshot.append_color(&ink, &thumb);
            snapshot.pop();
        }
    }
}

glib::wrapper! {
    pub struct Slider(ObjectSubclass<imp::Slider>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Slider {
    fn new() -> Self {
        let slider: Self = glib::Object::new();
        slider.set_hexpand(true);
        slider.set_cursor_from_name(Some("pointer"));
        slider
    }

    fn set(&self, count: usize, page: usize, ticks: &[usize], theme: usize) {
        let imp = self.imp();
        imp.count.set(count);
        imp.page.set(page);
        imp.theme.set(theme);
        if imp.ticks.borrow().as_slice() != ticks {
            *imp.ticks.borrow_mut() = ticks.to_vec();
        }
        self.queue_draw();
    }

    /// The book page under `x`.
    fn page_at(&self, x: f32) -> usize {
        let count = self.imp().count.get();
        if count <= 1 {
            return 0;
        }
        let fraction = ((x - 8.0) / (self.width() as f32 - 16.0)).clamp(0.0, 1.0);
        (fraction * (count - 1) as f32).round() as usize
    }
}
