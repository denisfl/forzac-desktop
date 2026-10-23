//! The library (`library`): the Books folder's books and the reading state's as the web reader's shelf of covers, with
//! the Android app's "Continue reading" card, generated covers, search and a question before a book (its file) goes to
//! the Trash; cover thumbnails are cached in `$XDG_CACHE_HOME/forzac/covers`, one file per book id (empty: the book has none).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use forzac_core::i18n::{self, fill, tr};
use forzac_core::library::{self, CoverStyle, View};
use forzac_core::model::Series;
use forzac_core::settings::Settings;
use forzac_core::state::{Entry, Store};
use forzac_core::update;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, graphene, gsk, pango, prelude::*};

use crate::icons::{self, Icon};
use crate::page_view::{Palette, THEMES, rgba};

/// The column's widest, as the web's `.library`.
const COLUMN: i32 = 1120;
/// A cover's narrowest and widest on the shelf.
const COVER_MIN: i32 = 130;
const COVER_MAX: i32 = 176;
/// Cached thumbnails fit this box: 176 points at 2×.
const THUMB: (i32, i32) = (352, 528);
/// Marbled paper is drawn at half a wide cover's size; it is soft anyway.
const MARBLE: (usize, usize) = (88, 132);
/// The empty state puts the sample page under the text below this width, as the web's media query.
const SPLIT: i32 = 860;

// region cover cache

fn cache_dir() -> PathBuf {
    glib::user_cache_dir().join("forzac").join("covers")
}

/// The cached thumbnail of the book `id`: a JPEG (a PNG for a cover with transparency), or an empty file when the book
/// has no cover.
fn thumbnail(id: &str) -> PathBuf {
    cache_dir().join(id)
}

/// Writes the thumbnail of a cover (`bytes`), or the empty marker for none; on a worker thread.
fn write_thumbnail(id: &str, bytes: Option<&[u8]>) {
    use gtk::gdk_pixbuf::PixbufLoader;
    let scaled = bytes.and_then(|bytes| {
        let loader = PixbufLoader::new();
        loader.connect_size_prepared(|l, w, h| {
            let s = (THUMB.0 as f64 / w as f64).min(THUMB.1 as f64 / h as f64).min(1.0);
            l.set_size(((w as f64 * s).round() as i32).max(1), ((h as f64 * s).round() as i32).max(1));
        });
        loader.write(bytes).ok()?;
        loader.close().ok()?;
        loader.pixbuf()
    });
    let path = thumbnail(id);
    let tmp = path.with_extension("tmp");
    let _ = std::fs::create_dir_all(cache_dir());
    let written = match scaled {
        Some(p) if p.has_alpha() => p.savev(&tmp, "png", &[]).is_ok(),
        Some(p) => p.savev(&tmp, "jpeg", &[("quality", "85")]).is_ok(),
        None => std::fs::write(&tmp, b"").is_ok(),
    };
    if written {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// Caches the cover of an opened book, if it is not cached yet, on a worker thread.
pub fn cache_cover(book: &forzac_core::Book) {
    if thumbnail(&book.id).exists() {
        return;
    }
    let id = book.id.clone();
    let bytes = book.cover.as_ref().and_then(|c| book.images.get(c)).map(|i| i.bytes.clone());
    drop(gio::spawn_blocking(move || write_thumbnail(&id, bytes.as_deref())));
}

// endregion

// region widgets

/// What a cover shows.
#[derive(Clone, Default)]
pub enum Thumb {
    /// Its picture is being loaded or made: the line colour, as the web's `.cover` until the image arrives.
    #[default]
    Waiting,
    Picture(gdk::Texture),
    /// No picture: a generated cover.
    Generated,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Kind {
    /// One child in the middle, at most `COLUMN` wide, with side padding.
    #[default]
    Column,
    /// The shelf: a grid of items in columns `COVER_MIN` to `COVER_MAX` wide.
    Shelf,
    /// The empty state: the text and the sample page side by side, or one under the other when narrow.
    Split,
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Layout {
        pub kind: Cell<Kind>,
        /// The shelf's column count at the last allocation.
        pub columns: Cell<usize>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Layout {
        const NAME: &'static str = "ForzacLibraryLayout";
        type Type = super::Layout;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Layout {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Layout {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (min, nat) = self.obj().measure_layout(orientation, for_size);
            (min, nat.max(min), -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            self.obj().allocate(width, height);
        }
    }

    #[derive(Default)]
    pub struct Cover {
        pub style: Cell<Option<CoverStyle>>,
        pub title: RefCell<String>,
        pub authors: RefCell<String>,
        pub thumb: RefCell<Thumb>,
        pub marble: RefCell<Option<gdk::Texture>>,
        /// A fixed size (the card's); else as wide as given, 2 : 3.
        pub size: Cell<Option<(i32, i32)>>,
        pub selected: Cell<bool>,
        pub hover: Cell<bool>,
        pub line: Cell<u32>,
        pub ink: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Cover {
        const NAME: &'static str = "ForzacCover";
        type Type = super::Cover;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Cover {}

    impl WidgetImpl for Cover {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if let Some((w, h)) = self.size.get() {
                let v = if orientation == gtk::Orientation::Horizontal { w } else { h };
                return (v, v, -1, -1);
            }
            match orientation {
                gtk::Orientation::Horizontal => (0, COVER_MAX, -1, -1),
                _ => {
                    let h = (if for_size > 0 { for_size } else { COVER_MAX } as f32 * 1.5).round() as i32;
                    (h, h, -1, -1)
                }
            }
        }

        fn snapshot(&self, s: &gtk::Snapshot) {
            self.obj().draw(s);
        }
    }

    #[derive(Default)]
    pub struct Sample {
        pub sheet: Cell<u32>,
        pub ink: Cell<u32>,
        pub muted: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Sample {
        const NAME: &'static str = "ForzacSamplePage";
        type Type = super::Sample;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Sample {}

    impl WidgetImpl for Sample {
        fn measure(&self, orientation: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            // The web's `.empty-book`: 360 : 520, with room for the turned page under it and the shadow.
            let v = if orientation == gtk::Orientation::Horizontal { Sample::WIDTH + 24 } else { Sample::HEIGHT + 48 };
            (v, v, -1, -1)
        }

        fn snapshot(&self, s: &gtk::Snapshot) {
            self.obj().draw(s);
        }
    }

    impl Sample {
        pub const WIDTH: i32 = 300;
        pub const HEIGHT: i32 = 433;
    }
}

glib::wrapper! {
    /// The library's layouts: the centred column, the shelf's grid, the empty state's two parts.
    pub struct Layout(ObjectSubclass<imp::Layout>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Layout {
    fn new(kind: Kind) -> Self {
        let layout: Self = glib::Object::new();
        layout.imp().kind.set(kind);
        layout
    }

    fn add(&self, child: &impl IsA<gtk::Widget>) {
        child.set_parent(self);
    }

    fn children(&self) -> Vec<gtk::Widget> {
        let mut out = Vec::new();
        let mut child = self.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            out.push(c);
        }
        out
    }

    /// The column's side padding and inner width for a width: the web's `clamp(20px, 4vw, 48px)`.
    fn column(width: i32) -> (i32, i32) {
        let pad = (width * 4 / 100).clamp(20, 48);
        let inner = (width - 2 * pad).clamp(0, COLUMN);
        ((width - inner) / 2, inner)
    }

    /// The shelf's columns, their width and the gap across for a width.
    fn grid(width: i32) -> (usize, i32, i32) {
        let gap = (width * 26 / 1000).clamp(20, 32);
        let min = (width * 14 / 100).clamp(COVER_MIN, COVER_MAX);
        let columns = ((width + gap) / (min + gap)).max(1);
        let column = ((width - gap * (columns - 1)) / columns).min(COVER_MAX.max(width / columns.max(1)));
        (columns as usize, column, gap)
    }

    /// The shelf's rows: their items' heights, for a column width.
    fn rows(children: &[gtk::Widget], columns: usize, column: i32) -> Vec<i32> {
        children.chunks(columns).map(|row| row.iter().map(|c| c.measure(gtk::Orientation::Vertical, column).1).max().unwrap_or(0)).collect()
    }

    fn measure_layout(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32) {
        let children = self.children();
        let horizontal = orientation == gtk::Orientation::Horizontal;
        match self.imp().kind.get() {
            Kind::Column => {
                let Some(child) = children.first() else { return (0, 0) };
                if horizontal {
                    let (min, nat, ..) = child.measure(orientation, -1);
                    (min + 40, nat.min(COLUMN) + 96)
                } else {
                    let width = if for_size > 0 { Self::column(for_size).1 } else { COLUMN };
                    let (min, nat, ..) = child.measure(orientation, width);
                    (min + 84, nat + 84) // the web's padding: 20 above, 64 below
                }
            }
            Kind::Shelf => {
                if horizontal {
                    return (COVER_MIN, 6 * COVER_MAX + 5 * 32);
                }
                let width = if for_size > 0 { for_size } else { 6 * COVER_MAX + 5 * 32 };
                let (columns, column, _) = Self::grid(width);
                let rows = Self::rows(&children, columns, column);
                let h = rows.iter().sum::<i32>() + 36 * (rows.len() as i32 - 1).max(0);
                (h, h)
            }
            Kind::Split => {
                let [text, page] = &children[..] else { return (0, 0) };
                let (_, page_w, ..) = page.measure(gtk::Orientation::Horizontal, -1);
                if horizontal {
                    let (min, nat, ..) = text.measure(orientation, -1);
                    return (min.max(page_w), nat + page_w + 112);
                }
                let width = if for_size > 0 { for_size } else { SPLIT };
                let (_, page_h, ..) = page.measure(orientation, -1);
                if width >= SPLIT {
                    let gap = (width * 7 / 100).clamp(40, 112);
                    let (_, text_h, ..) = text.measure(orientation, width - gap - page_w);
                    let h = text_h.max(page_h);
                    (h, h)
                } else {
                    let (_, text_h, ..) = text.measure(orientation, width);
                    (text_h + 40 + page_h, text_h + 40 + page_h)
                }
            }
        }
    }

    fn allocate(&self, width: i32, height: i32) {
        let children = self.children();
        let place = |c: &gtk::Widget, x: i32, y: i32, w: i32, h: i32| c.size_allocate(&gtk::Allocation::new(x, y, w, h), -1);
        match self.imp().kind.get() {
            Kind::Column => {
                if let Some(child) = children.first() {
                    let (x, inner) = Self::column(width);
                    place(child, x, 20, inner, (height - 84).max(0));
                }
            }
            Kind::Shelf => {
                let (columns, column, gap) = Self::grid(width);
                self.imp().columns.set(columns);
                let mut y = 0;
                for (row, h) in children.chunks(columns).zip(Self::rows(&children, columns, column)) {
                    for (i, c) in row.iter().enumerate() {
                        place(c, i as i32 * (column + gap), y, column, h);
                    }
                    y += h + 36;
                }
            }
            Kind::Split => {
                let [text, page] = &children[..] else { return };
                let (_, page_w, ..) = page.measure(gtk::Orientation::Horizontal, -1);
                let (_, page_h, ..) = page.measure(gtk::Orientation::Vertical, -1);
                if width >= SPLIT {
                    let gap = (width * 7 / 100).clamp(40, 112);
                    let text_w = width - gap - page_w;
                    let (_, text_h, ..) = text.measure(gtk::Orientation::Vertical, text_w);
                    place(text, 0, (height - text_h).max(0) / 2, text_w, text_h);
                    place(page, width - page_w, (height - page_h).max(0) / 2, page_w, page_h);
                } else {
                    let (_, text_h, ..) = text.measure(gtk::Orientation::Vertical, width);
                    place(text, 0, 0, width, text_h);
                    place(page, (width - page_w).max(0) / 2, text_h + 40, page_w, page_h);
                }
            }
        }
    }
}

glib::wrapper! {
    /// A book's cover: its picture filling the box, or a generated one (the Android app's `GeneratedCover`).
    pub struct Cover(ObjectSubclass<imp::Cover>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

fn black(alpha: f32) -> gdk::RGBA {
    gdk::RGBA::new(0.0, 0.0, 0.0, alpha)
}

/// A font description at `px` pixels.
fn font(family: &str, px: f32) -> pango::FontDescription {
    let mut f = pango::FontDescription::from_string(family);
    f.set_absolute_size(px as f64 * pango::SCALE as f64);
    f
}

impl Cover {
    fn new(book: &Book, thumb: Thumb, palette: Palette, size: Option<(i32, i32)>) -> Self {
        let cover: Self = glib::Object::new();
        let imp = cover.imp();
        imp.style.set(Some(library::cover_style(&book.id, &book.title)));
        *imp.title.borrow_mut() = book.title.clone();
        *imp.authors.borrow_mut() = book.authors.clone();
        *imp.thumb.borrow_mut() = thumb;
        imp.size.set(size);
        imp.line.set(palette.line);
        imp.ink.set(palette.ink);
        cover.set_overflow(gtk::Overflow::Visible);
        cover
    }

    fn set_thumb(&self, thumb: Thumb) {
        *self.imp().thumb.borrow_mut() = thumb;
        self.queue_draw();
    }

    fn set_marble(&self, texture: gdk::Texture) {
        *self.imp().marble.borrow_mut() = Some(texture);
        self.queue_draw();
    }

    fn set_selected(&self, selected: bool) {
        if self.imp().selected.replace(selected) != selected {
            self.queue_draw();
        }
    }

    fn set_hover(&self, hover: bool) {
        if self.imp().hover.replace(hover) != hover {
            self.queue_draw();
        }
    }

    fn draw(&self, s: &gtk::Snapshot) {
        let imp = self.imp();
        let (w, h) = (self.width() as f32, self.height() as f32);
        if w < 1.0 || h < 1.0 {
            return;
        }
        if imp.hover.get() {
            s.translate(&graphene::Point::new(0.0, -3.0)); // the web's `li:hover .cover`
        }
        // Android's `BookShape`: nearly square at the spine, rounded at the edge.
        let bounds = graphene::Rect::new(0.0, 0.0, w, h);
        let (spine, edge) = (graphene::Size::new(2.0, 2.0), graphene::Size::new(w * 0.06, w * 0.06));
        let shape = gsk::RoundedRect::new(bounds, spine, edge, edge, spine);
        s.append_outset_shadow(&shape, &black(0.12), 0.0, 1.0, 0.0, 2.0);
        s.append_outset_shadow(&shape, &black(0.10), 0.0, 6.0, 0.0, 18.0);
        if imp.selected.get() {
            let grow = |r: graphene::Size| graphene::Size::new(r.width() + 5.0, r.height() + 5.0);
            let ring = gsk::RoundedRect::new(
                graphene::Rect::new(-5.0, -5.0, w + 10.0, h + 10.0),
                grow(spine),
                grow(edge),
                grow(edge),
                grow(spine),
            );
            s.append_border(&ring, &[2.0; 4], &[rgba(imp.ink.get()); 4]);
        }
        s.push_rounded_clip(&shape);
        match &*imp.thumb.borrow() {
            Thumb::Picture(t) => {
                let (tw, th) = (t.width() as f32, t.height() as f32);
                let scale = (w / tw).max(h / th);
                let rect = graphene::Rect::new((w - tw * scale) / 2.0, (h - th * scale) / 2.0, tw * scale, th * scale);
                s.append_scaled_texture(t, gsk::ScalingFilter::Trilinear, &rect);
            }
            Thumb::Waiting => s.append_color(&rgba(imp.line.get()), &bounds),
            Thumb::Generated => self.draw_generated(s, w, h),
        }
        s.pop();
    }

    /// Cloth or marbled paper, a darker spine and the paper label with the title and the authors (`Covers.kt`).
    fn draw_generated(&self, s: &gtk::Snapshot, w: f32, h: f32) {
        let imp = self.imp();
        let Some(style) = imp.style.get() else { return };
        let bounds = graphene::Rect::new(0.0, 0.0, w, h);
        s.append_color(&rgba(style.cloth), &bounds);
        if let Some(marble) = &*imp.marble.borrow() {
            s.append_scaled_texture(marble, gsk::ScalingFilter::Linear, &bounds);
        }
        s.append_color(&rgba(style.spine), &graphene::Rect::new(0.0, 0.0, w * 0.09, h));

        let base = w * 0.115;
        let title = imp.title.borrow();
        let text_w = w * 0.65;
        // The longest word fits a line whole: a hyphen in a short title looks accidental. Down to 0.6 of the size.
        let longest = title.split(' ').max_by_key(|word| word.chars().count()).unwrap_or("");
        let probe = self.create_pango_layout(Some(longest));
        probe.set_font_description(Some(&font("PT Serif Bold", base * style.title_scale)));
        let word_w = probe.pixel_size().0 as f32;
        let room = text_w * 0.92;
        let fit = if word_w > room { (room / word_w).max(0.6) } else { 1.0 };
        let title_layout = self.create_pango_layout(Some(&title));
        title_layout.set_font_description(Some(&font("PT Serif Bold", base * style.title_scale * fit)));
        let authors = imp.authors.borrow().to_uppercase();
        let author_layout = (!authors.trim().is_empty()).then(|| {
            let l = self.create_pango_layout(Some(&authors));
            l.set_font_description(Some(&font("Golos Text", base * 0.46)));
            let attrs = pango::AttrList::new();
            attrs.insert(pango::AttrInt::new_letter_spacing((base * 0.05 * pango::SCALE as f32) as i32));
            l.set_attributes(Some(&attrs));
            (l, 2)
        });
        for (l, lines) in [(&title_layout, style.title_lines as i32)].into_iter().chain(author_layout.iter().map(|(l, n)| (l, *n))) {
            l.set_width((text_w * pango::SCALE as f32) as i32);
            l.set_wrap(pango::WrapMode::WordChar);
            l.set_alignment(pango::Alignment::Center);
            l.set_ellipsize(pango::EllipsizeMode::End);
            l.set_height(-lines);
        }
        let title_h = title_layout.pixel_size().1 as f32;
        let authors_h = author_layout.as_ref().map_or(0.0, |(l, _)| w * 0.035 + l.pixel_size().1 as f32);
        let (pad_x, pad_y) = (w * 0.05, w * 0.06);
        let label = graphene::Rect::new(w * 0.17, h * 0.2, w * 0.75, pad_y * 2.0 + title_h + authors_h);
        s.append_color(&rgba(library::LABEL_PAPER), &label);
        let frame = w * 0.008 + 0.5;
        s.append_border(&gsk::RoundedRect::from_rect(label, 0.0), &[frame; 4], &[rgba(library::LABEL_FRAME); 4]);
        s.save();
        s.translate(&graphene::Point::new(label.x() + pad_x, label.y() + pad_y));
        s.append_layout(&title_layout, &rgba(library::LABEL_INK));
        if let Some((l, _)) = &author_layout {
            s.translate(&graphene::Point::new(0.0, title_h + w * 0.035));
            s.append_layout(l, &rgba(library::LABEL_MUTED));
        }
        s.restore();
    }
}

glib::wrapper! {
    /// The empty state's illustration: a book page over a second one turned slightly (the web's `.empty-book`).
    pub struct Sample(ObjectSubclass<imp::Sample>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Sample {
    fn new(palette: Palette) -> Self {
        let sample: Self = glib::Object::new();
        sample.imp().sheet.set(palette.sheet);
        sample.imp().ink.set(palette.ink);
        sample.imp().muted.set(palette.muted);
        sample
    }

    fn draw(&self, s: &gtk::Snapshot) {
        let imp = self.imp();
        let (w, h) = (imp::Sample::WIDTH as f32, imp::Sample::HEIGHT as f32);
        let page = gsk::RoundedRect::from_rect(graphene::Rect::new(0.0, 0.0, w, h), 6.0);
        s.save();
        s.translate(&graphene::Point::new(4.0, 16.0));
        // The page under it: turned 3.2° about its centre, shifted, faded.
        s.save();
        s.translate(&graphene::Point::new(w / 2.0 + w * 0.04, h / 2.0 + h * 0.01));
        s.rotate(3.2);
        s.translate(&graphene::Point::new(-w / 2.0, -h / 2.0));
        s.push_opacity(0.7);
        s.append_outset_shadow(&page, &black(0.12), 0.0, 18.0, 0.0, 48.0);
        s.push_rounded_clip(&page);
        s.append_color(&rgba(imp.sheet.get()), page.bounds());
        s.pop();
        s.pop();
        s.restore();
        s.append_outset_shadow(&page, &black(0.06), 0.0, 1.0, 0.0, 2.0);
        s.append_outset_shadow(&page, &black(0.12), 0.0, 18.0, 0.0, 48.0);
        s.push_rounded_clip(&page);
        s.append_color(&rgba(imp.sheet.get()), page.bounds());

        let sample = library::sample_page();
        let size = w * 0.0355;
        let (ink, muted) = (rgba(imp.ink.get()), rgba(imp.muted.get()));
        let (left, inner) = (w * 0.09, w * 0.82);
        let layout = |text: &str, family: &str, px: f32| {
            let l = self.create_pango_layout(Some(text));
            l.set_font_description(Some(&font(family, px)));
            l.set_width((inner * pango::SCALE as f32) as i32);
            l.set_wrap(pango::WrapMode::WordChar);
            l
        };
        // Line height 1.6, as the web's page: the font's own line height plus the difference.
        let line = |l: &pango::Layout, px: f32| {
            let natural = l.line(0).map_or(px, |line| line.extents().1.height() as f32 / pango::SCALE as f32);
            l.set_spacing(((px * 1.6 - natural).max(0.0) * pango::SCALE as f32) as i32);
        };
        // The web's padding `9% 9% 7%`: percentages of the width, also above and below.
        let y = Cell::new(w * 0.09);
        let put = |l: &pango::Layout, colour: &gdk::RGBA, after: f32| {
            s.save();
            s.translate(&graphene::Point::new(left, y.get()));
            s.append_layout(l, colour);
            s.restore();
            y.set(y.get() + l.pixel_size().1 as f32 + after);
        };
        let head = layout(sample.head, "Golos Text", size * 0.78);
        head.set_alignment(pango::Alignment::Center);
        put(&head, &muted, size * 2.2);
        let title = layout(sample.title, "Literata Bold", size * 1.15);
        title.set_alignment(pango::Alignment::Center);
        put(&title, &ink, if sample.subtitle.is_empty() { size * 1.4 } else { 0.0 });
        if !sample.subtitle.is_empty() {
            let subtitle = layout(sample.subtitle, "Literata Italic", size);
            subtitle.set_alignment(pango::Alignment::Center);
            put(&subtitle, &ink, size * 1.4);
        }
        let folio = layout(sample.folio, "Golos Text", size * 0.72);
        folio.set_alignment(pango::Alignment::Center);
        // The text stops above the folio, as the web's page hides what does not fit.
        // At a line's edge: the paragraphs' lines are 1.6 of the size apart from the first one on.
        let room = h - w * 0.07 - folio.pixel_size().1 as f32 - size - y.get();
        s.push_clip(&graphene::Rect::new(0.0, 0.0, w, y.get() + (room / (size * 1.6)).floor() * size * 1.6));
        for text in sample.paragraphs {
            let p = layout(&library::soft_hyphens(text, sample.language), "Literata", size);
            p.set_justify(true);
            p.set_indent((size * 1.5 * pango::SCALE as f32) as i32);
            let attrs = pango::AttrList::new();
            attrs.insert(pango::AttrLanguage::new(&pango::Language::from_string(sample.language)));
            p.set_attributes(Some(&attrs));
            line(&p, size);
            put(&p, &ink, 0.0);
        }
        s.pop();
        s.save();
        s.translate(&graphene::Point::new(left, h - w * 0.07 - folio.pixel_size().1 as f32));
        s.append_layout(&folio, &muted);
        s.restore();
        s.pop();
        s.restore();
    }
}

// endregion

/// A book of the library, from the Books folder and the reading state.
#[derive(Clone)]
struct Book {
    id: String,
    path: PathBuf,
    title: String,
    authors: String,
    progress: f32,
    /// False for a book in the folder never opened.
    opened: bool,
    series: Option<Series>,
    /// The reader's shelves it is on (`library-shelves`).
    shelves: Vec<String>,
}

impl Book {
    fn of(e: &Entry) -> Book {
        Book {
            id: e.id.clone(),
            path: e.path.clone(),
            title: if e.title.is_empty() { e.path.file_name().unwrap_or_default().to_string_lossy().into_owned() } else { e.title.clone() },
            authors: e.authors.clone(),
            progress: e.progress,
            opened: e.opened > 0,
            series: e.series.clone(),
            shelves: e.shelves.clone(),
        }
    }

    /// The key the views sort it by (`library::arrange`).
    fn entry(&self) -> Entry {
        Entry {
            title: self.title.clone(),
            authors: self.authors.clone(),
            series: self.series.clone(),
            progress: self.progress,
            opened: u64::from(self.opened),
            shelves: self.shelves.clone(),
            ..Entry::default()
        }
    }
}

/// A selectable book on the window: the card or a shelf item.
struct Item {
    book: usize,
    cover: Cover,
    widget: gtk::Widget,
    /// The "⋯" button, shown under the pointer and on the selected book.
    more: gtk::Button,
}

thread_local! {
    static PROVIDER: gtk::CssProvider = {
        let provider = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        provider
    };
    /// The library window, while it is open: there is one at a time.
    static OPEN: RefCell<Option<glib::WeakRef<gtk::ApplicationWindow>>> = const { RefCell::new(None) };
    /// Its `Library`, for the end page's month line (`open_library`).
    static LIBRARY: RefCell<Option<std::rc::Weak<Library>>> = const { RefCell::new(None) };
}

/// The library's colours in the reader's theme; its layout (the web's `reader.css`).
fn set_theme(p: Palette) {
    let hex = |c: u32| format!("#{c:06x}");
    let (paper, ink, muted, line, sheet, link) = (hex(p.paper), hex(p.ink), hex(p.muted), hex(p.line), hex(p.sheet), hex(p.link));
    PROVIDER.with(|provider| {
        provider.load_from_string(&format!(
            "window.forzac-library {{ background-color: {paper}; color: {ink}; }}
             .forzac-library .muted, .forzac-library button.folder-link, .forzac-library .shelf-author, .forzac-library .shelf-share,
                 .forzac-library .card-caption, .forzac-library .card-authors, .forzac-library .hint, .forzac-library .privacy,
                 .forzac-library .no-results, .forzac-library .question-body, .forzac-library button.remove,
                 .forzac-library .update {{ color: {muted}; }}
             .forzac-library .update link {{ color: {link}; text-decoration-line: none; }}
             .forzac-library button.pill {{ background-color: {ink}; color: {paper}; }}
             .forzac-library button.circle {{ border: 1px solid {line}; }}
             .forzac-library button.circle:hover {{ background-color: alpha({ink}, 0.06); }}
             .forzac-library .rule {{ background-color: {line}; }}
             .forzac-library .privacy {{ border-top: 1px solid {line}; }}
             .forzac-library progressbar trough {{ background-color: {line}; }}
             .forzac-library progressbar progress {{ background-color: {ink}; }}
             .forzac-library entry.search-field {{ color: {ink}; caret-color: {ink}; }}
             .forzac-library entry.search-field text placeholder {{ color: {muted}; }}
             .forzac-library .drop {{ border: 2px dashed {muted}; background-color: alpha({paper}, 0.88); }}
             .forzac-library .question {{ background-color: {sheet}; }}
             .forzac-library button.sheet-button {{ border: 1px solid {line}; }}
             .forzac-library button.sheet-button.filled {{ background-color: {ink}; border-color: {ink}; color: {sheet}; }}
             .forzac-library button.view-tab {{ color: {muted}; }}
             .forzac-library button.view-tab.active {{ color: {ink}; border-bottom: 2px solid {ink}; }}
             .forzac-library .tabs {{ border-bottom: 1px solid {line}; }}
             .forzac-library .about {{ background-color: {sheet}; }}
             .forzac-library .about-details, .forzac-library .about-series {{ color: {muted}; }}
             .forzac-library headerbar {{ background: {paper}; color: {ink}; box-shadow: none; border-bottom: 1px solid {line}; }}
             .forzac-library headerbar button, .forzac-library headerbar menubutton > button {{ color: {ink}; background: none;
                 border: none; box-shadow: none; }}
             .forzac-library headerbar button:hover, .forzac-library headerbar button:checked,
                 .forzac-library headerbar menubutton > button:hover {{ background-color: alpha({ink}, 0.08); }}
             .forzac-library .section-switcher {{ border: 1px solid {line}; }}
             .forzac-library .section-switcher button {{ color: {muted}; }}
             .forzac-library .section-switcher button:checked {{ background-color: {line}; color: {ink}; }}
             .forzac-library .section-pill {{ background-color: {sheet}; }}
             .forzac-library searchbar > revealer > box {{ background: {paper}; border-bottom: 1px solid {line}; }}
             .forzac-library entry.search-field {{ background-color: {sheet}; border: 1px solid {line}; }}
             .forzac-library popover > contents {{ background-color: {sheet}; color: {ink}; }}
             .forzac-library button.swatch.chosen {{ box-shadow: inset 0 0 0 2px {link}; }}
             .forzac-library .about label link, .forzac-library .question-body link {{ color: {link}; }}
             {STATIC_CSS}"
        ));
    });
}

const STATIC_CSS: &str = "
    .forzac-library button.pill { border-radius: 999px; padding: 11px 20px; font-weight: 500; font-size: 15px; }
    .forzac-library button.pill:hover { opacity: 0.88; }
    .forzac-library button.pill.big { padding: 16px 30px; font-size: 17px; }
    .forzac-library button.circle { min-width: 42px; min-height: 42px; border-radius: 22px; }
    .forzac-library .shelf-title { font-weight: 500; font-size: 14px; }
    .forzac-library .shelf-author { font-size: 13px; }
    .forzac-library .shelf-share { font-size: 12px; font-feature-settings: 'tnum'; }
    .forzac-library button.remove { font-size: 18px; padding: 0 2px 4px 8px; }
    .forzac-library .update { font-size: 13px; }
    .forzac-library button.remove.update-close { margin-right: 14px; padding: 0 2px 0 8px; }
    .forzac-library .card-caption { font-size: 12px; font-weight: 500; letter-spacing: 0.7px; }
    .forzac-library .card-title { font-family: 'Literata'; font-weight: 600; font-size: 21px; }
    .forzac-library .card-authors { font-size: 14px; }
    .forzac-library progressbar trough, .forzac-library progressbar progress { min-height: 2px; min-width: 0; margin: 0;
        border-radius: 1px; border: none; box-shadow: none; }
    .forzac-library .rule { min-height: 1px; }
    .forzac-library entry.search-field { font-size: 15px; border-radius: 10px; box-shadow: none; outline: none; padding: 0 12px;
        min-height: 34px; }
    .forzac-library searchbar > revealer > box { box-shadow: none; padding: 8px 12px; }
    .forzac-library headerbar { min-height: 48px; }
    .forzac-library .section-switcher { border-radius: 9px; padding: 2px; }
    .forzac-library .section-switcher button { background: none; border: none; box-shadow: none; border-radius: 7px;
        padding: 3px 14px; min-height: 0; font-weight: 400; }
    .forzac-library .section-switcher button:checked { font-weight: 500; }
    .forzac-library .section-pill { border-radius: 999px; padding: 4px; box-shadow: 0 4px 18px alpha(black, 0.16); }
    .forzac-library .section-pill button { border-radius: 999px; padding: 8px 20px; font-size: 15px; }
    .forzac-library .section-title { font-weight: 600; font-size: 15px; }
    .forzac-library .bottom-sheet { border-radius: 22px 22px 0 0; }
    .forzac-library button.swatch { background: none; border: none; box-shadow: none; padding: 4px; border-radius: 12px; }
    .forzac-library .empty-heading { font-family: 'Literata'; font-weight: 700; font-size: 44px; letter-spacing: -0.4px;
        line-height: 1.08; }
    .forzac-library .empty-lead { font-family: 'Literata'; font-size: 19px; line-height: 1.55; }
    .forzac-library .hint { font-size: 15px; }
    .forzac-library button.folder-link { font-size: 15px; padding: 4px 0; background: none; border: none; box-shadow: none;
        text-decoration-line: underline; }
    .forzac-library .privacy { font-size: 14px; line-height: 1.55; padding-top: 18px; }
    .forzac-library .no-results { font-size: 15px; }
    .forzac-library .drop { border-radius: 20px; margin: 16px; }
    .forzac-library .drop label { font-size: 20px; font-weight: 500; }
    .forzac-library .backdrop { background-color: alpha(black, 0.2); }
    .forzac-library .question { border-radius: 22px 22px 0 0; padding: 24px; }
    .forzac-library .question-title { font-family: 'PT Serif'; font-weight: 700; font-size: 19px; }
    .forzac-library .question-body { font-size: 14px; }
    .forzac-library button.sheet-button { min-height: 48px; border-radius: 24px; font-size: 16px; font-weight: 500; }
    .forzac-library button.view-tab { background: none; border: none; box-shadow: none; border-radius: 0; padding: 8px 0;
        margin-right: 24px; font-size: 15px; font-weight: 500; border-bottom: 2px solid transparent; }
    .forzac-library .group-heading { font-family: 'Literata'; font-weight: 600; font-size: 19px; }
    .forzac-library .about { border-radius: 22px; padding: 28px; }
    .forzac-library .about-title { font-family: 'Literata'; font-weight: 700; font-size: 22px; }
    .forzac-library .about-authors { font-size: 15px; }
    .forzac-library .about-series, .forzac-library .about-details { font-size: 14px; }
    .forzac-library .about-annotation { font-family: 'Literata'; font-size: 15px; line-height: 1.5; }
    .forzac-library .settings-heading { font-weight: 700; font-size: 15px; margin-top: 12px; }
    .forzac-library button.sheet-button.small { min-height: 36px; border-radius: 18px; padding: 0 16px; font-size: 14px; }
";

/// The "Edit…" form: its sheet, the book, its fields and the parsed book (its own details).
type EditForm = (gtk::Widget, usize, [gtk::Entry; 5], forzac_core::Book);

/// The settings sheet (`settings`) and what a sync or a change of the Books folder updates in it.
struct SettingsSheet {
    sheet: gtk::Widget,
    /// The six themes' swatches, the chosen one marked.
    swatches: Vec<gtk::Button>,
    folder: gtk::Label,
    status: gtk::Label,
    devices: gtk::Box,
    sync: gtk::Button,
}

/// For screenshots (`--png`): the section in `FORZAC_SECTION` and the sheet in `FORZAC_SHEET` shown; whether any was.
pub fn screenshot_state(library: &Rc<Library>) -> bool {
    let mut shown = false;
    if std::env::var("FORZAC_SECTION").as_deref() == Ok("1") {
        library.show_section(1);
        shown = true;
    }
    match std::env::var("FORZAC_SHEET").as_deref() {
        Ok("settings") => library.show_settings(),
        Ok("help") => library.toggle_help(),
        Ok("about") => library.show_about_app(),
        _ => return shown,
    }
    true
}

/// The open library window's `Library`, while there is one.
pub fn open_library() -> Option<Rc<Library>> {
    LIBRARY.with(|l| l.borrow().as_ref().and_then(|w| w.upgrade()))
}

/// The library window and what it shows.
pub struct Library {
    pub window: gtk::ApplicationWindow,
    store: Rc<RefCell<Store>>,
    /// Opens a book in its window (copied into the Books folder first).
    open: Rc<dyn Fn(&Path)>,
    /// The Books folder, when it could be made.
    folder: RefCell<Option<PathBuf>>,
    palette: Cell<Palette>,
    books: RefCell<Vec<Book>>,
    thumbs: RefCell<HashMap<String, Thumb>>,
    marbles: RefCell<HashMap<i32, gdk::Texture>>,
    items: RefCell<Vec<Item>>,
    /// The item index of the card's book, if shown.
    card: Cell<bool>,
    /// The groups' shelves, in order (`library::arrange`).
    shelves: RefCell<Vec<Layout>>,
    view: Cell<View>,
    /// The views' tabs under the head.
    tabs: gtk::Box,
    /// "About the book" while it shows, with its book.
    about: RefCell<Option<(gtk::Widget, usize)>>,
    selected: Cell<usize>,
    searching: Cell<bool>,
    scrolled: gtk::ScrolledWindow,
    overlay: gtk::Overlay,
    /// The books' page in the scrolled column: the update line, the tabs, the body.
    page: gtk::Box,
    body: gtk::Box,
    /// "Version N is out — download ×" (`update-check`), on top of "Books".
    update: (gtk::Box, gtk::Label),
    /// The search bar under the header (`library-sections`) and its field.
    search_bar: gtk::SearchBar,
    entry: gtk::Entry,
    /// The header's actions: "+" and the search toggle in "Books", "Save Image…" in "Reading stats".
    add_button: gtk::MenuButton,
    search_toggle: gtk::ToggleButton,
    save_button: gtk::Button,
    /// The section shown (0 "Books", 1 "Reading stats"), the two sections, and the switchers: the header's and the
    /// bottom pill's (narrow windows), with the header's title that stands in for the switcher there.
    section: Cell<usize>,
    sections: gtk::Stack,
    stats: crate::stats::Section,
    stats_shown: Cell<bool>,
    switchers: [(gtk::ToggleButton, gtk::ToggleButton); 2],
    title: (gtk::Stack, gtk::Label),
    pill: gtk::Box,
    narrow: Cell<bool>,
    /// The library's keys card (`help`).
    help: Rc<crate::help::Help>,
    /// "About Forzac" while it shows.
    about_app: RefCell<Option<gtk::Widget>>,
    /// "Adding books… N of M" while a folder is added (`folder-import`), on top of "Books".
    progress: gtk::Label,
    adding: Cell<bool>,
    /// Folders dropped while one is being added.
    waiting: RefCell<Vec<PathBuf>>,
    question: RefCell<Option<(gtk::Widget, usize)>>,
    /// The new shelf's question while it shows: its sheet, the book and the name field.
    naming: RefCell<Option<(gtk::Widget, usize, gtk::Entry)>>,
    /// The "Edit…" form while it shows.
    editing: RefCell<Option<EditForm>>,
    /// The settings while they show.
    settings: RefCell<Option<SettingsSheet>>,
    /// Covers being loaded or made; `settled` runs when none is left after the first frame.
    pending: Cell<usize>,
    first_frame: Cell<Option<Instant>>,
    settled: RefCell<Vec<Box<dyn FnOnce()>>>,
}

impl Library {
    /// The library window for `books` (`library::scan`), or the open one brought forward.
    pub fn show(
        app: &gtk::Application,
        store: &Rc<RefCell<Store>>,
        theme: usize,
        size: (i32, i32),
        open: Rc<dyn Fn(&Path)>,
        books: Vec<Entry>,
        folder: Option<PathBuf>,
    ) -> Option<Rc<Library>> {
        if let Some(window) = OPEN.with(|o| o.borrow().as_ref().and_then(|w| w.upgrade())).filter(|w| w.application().is_some()) {
            window.present();
            return None;
        }
        crate::fonts::add_interface_fonts();
        let palette = THEMES[theme];
        set_theme(palette);
        let books: Vec<Book> = books.iter().map(Book::of).collect();
        let window =
            gtk::ApplicationWindow::builder().application(app).title(tr("Forzac")).default_width(size.0).default_height(size.1).build();
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            window.set_decorated(false);
        }
        window.add_css_class("forzac-panel");
        window.add_css_class("forzac-library");
        OPEN.with(|o| *o.borrow_mut() = Some(window.downgrade()));

        let entry = gtk::Entry::builder().placeholder_text(tr("Title, author or series")).width_request(360).build();
        entry.add_css_class("search-field");
        let search_bar = gtk::SearchBar::builder().child(&entry).show_close_button(false).build();
        search_bar.connect_entry(&entry);
        let header_button = |paths: icons::Paths, tip: &str| {
            let b = gtk::Button::builder().child(&Icon::new(paths, 20)).tooltip_text(tip).focusable(false).build();
            b.set_valign(gtk::Align::Center);
            b
        };
        // "+": opening a book and adding a folder, as File's items on the Mac.
        let add_menu = gio::Menu::new();
        let open_item = gio::MenuItem::new(Some(tr("Open a Book…")), Some("lib.openbook"));
        open_item.set_attribute_value("accel", Some(&"<Control>o".to_variant()));
        add_menu.append_item(&open_item);
        let folder_item = gio::MenuItem::new(Some(tr("Add a Folder…")), Some("lib.addfolder"));
        folder_item.set_attribute_value("accel", Some(&"<Control><Shift>o".to_variant()));
        add_menu.append_item(&folder_item);
        let add_button =
            gtk::MenuButton::builder().child(&Icon::new(icons::PLUS, 20)).tooltip_text(tr("Add a Book")).menu_model(&add_menu).build();
        add_button.set_focusable(false);
        let search_toggle =
            gtk::ToggleButton::builder().child(&Icon::new(icons::SEARCH, 20)).tooltip_text(tr("Search")).focusable(false).build();
        search_toggle.set_valign(gtk::Align::Center);
        search_toggle.bind_property("active", &search_bar, "search-mode-enabled").bidirectional().build();
        let save_button = header_button(icons::DOWNLOAD, tr("Save Image…"));
        save_button.set_visible(false);
        // The main menu (GNOME's "Main Menu"): Settings, Keyboard Shortcuts, About; `F10` opens it.
        let main_menu = gio::Menu::new();
        let item = |label: &str, action: &str, accel: &str| {
            let i = gio::MenuItem::new(Some(label), Some(action));
            i.set_attribute_value("accel", Some(&accel.to_variant()));
            i
        };
        main_menu.append_item(&item(tr("Settings"), "lib.settings", "<Control>comma"));
        main_menu.append_item(&item(tr("Keyboard Shortcuts"), "lib.help", "<Control>question"));
        main_menu.append_item(&item(tr("About Forzac"), "lib.aboutapp", ""));
        let menu_button = gtk::MenuButton::builder()
            .child(&Icon::new(icons::MENU, 20))
            .tooltip_text(tr("Main Menu"))
            .menu_model(&main_menu)
            .primary(true)
            .build();
        menu_button.set_focusable(false);
        // The two switchers: the header's, and the bottom pill's for narrow windows.
        let switcher = |class: &str| {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            b.add_css_class(class);
            b.set_valign(gtk::Align::Center);
            let books = gtk::ToggleButton::builder().label(tr("Books")).focusable(false).active(true).build();
            let stats = gtk::ToggleButton::builder().label(tr("Reading stats")).focusable(false).group(&books).build();
            b.append(&books);
            b.append(&stats);
            (b, books, stats)
        };
        let (header_switcher, hb, hs) = switcher("section-switcher");
        let (pill, pb, ps) = switcher("section-switcher");
        pill.add_css_class("section-pill");
        pill.set_halign(gtk::Align::Center);
        pill.set_valign(gtk::Align::End);
        pill.set_margin_bottom(16);
        pill.set_visible(false);
        let title_label = gtk::Label::new(Some(tr("Books")));
        title_label.add_css_class("section-title");
        let title = gtk::Stack::new();
        title.add_named(&header_switcher, Some("switcher"));
        title.add_named(&title_label, Some("title"));
        title.set_visible_child_name("switcher");
        let header = gtk::HeaderBar::new();
        header.add_css_class("library-header");
        header.pack_start(&add_button);
        header.pack_start(&search_toggle);
        header.pack_start(&save_button);
        header.set_title_widget(Some(&title));
        header.pack_end(&menu_button);

        let progress = gtk::Label::new(None);
        progress.add_css_class("hint");
        progress.set_visible(false);
        let update_label = gtk::Label::new(None);
        update_label.add_css_class("update");
        update_label.set_ellipsize(pango::EllipsizeMode::End);
        update_label.connect_activate_link(|label, uri| {
            let window = label.root().and_downcast::<gtk::Window>();
            gtk::UriLauncher::new(uri).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
            glib::Propagation::Stop
        });
        let dismiss = gtk::Button::builder().label("×").focusable(false).build();
        dismiss.add_css_class("remove");
        dismiss.add_css_class("update-close");
        let update_line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        update_line.append(&update_label);
        update_line.append(&dismiss);
        update_line.set_valign(gtk::Align::Center);
        dismiss.set_valign(gtk::Align::Center);
        update_line.set_visible(false);
        // On top of "Books", one line: the progress of adding books and the update line.
        let notes = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        notes.set_margin_top(12);
        notes.append(&progress);
        notes.append(&update_line);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        body.set_vexpand(true);
        let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        tabs.add_css_class("tabs");
        tabs.set_margin_top(4);
        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        page.append(&notes);
        page.append(&tabs);
        page.append(&body);
        let view = View::from_key(store.borrow().setting(library::VIEW));
        let column = Layout::new(Kind::Column);
        column.add(&page);
        let scrolled = gtk::ScrolledWindow::builder().child(&column).hscrollbar_policy(gtk::PolicyType::Never).build();
        let stats = crate::stats::Section::new(window.upcast_ref(), store, palette, {
            let window = window.downgrade();
            move || {
                if let Some(w) = window.upgrade() {
                    let _ = WidgetExt::activate_action(&w, "lib.books", None);
                }
            }
        });
        let sections = gtk::Stack::builder().vexpand(true).build();
        sections.add_named(&scrolled, Some("books"));
        sections.add_named(&stats.widget, Some("stats"));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        // On Hyprland, which draws no title bars, the header is the window's top row; elsewhere, its title bar.
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            header.set_show_title_buttons(false);
            root.append(&header);
        } else {
            window.set_titlebar(Some(&header));
        }
        root.append(&search_bar);
        root.append(&sections);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&root));
        overlay.add_overlay(&pill);
        let help = crate::help::Help::new(true);
        overlay.add_overlay(&help.root);
        window.set_child(Some(&overlay));

        let library = Rc::new(Library {
            window,
            store: store.clone(),
            open,
            folder: RefCell::new(folder),
            palette: Cell::new(palette),
            books: RefCell::new(books),
            thumbs: RefCell::default(),
            marbles: RefCell::default(),
            items: RefCell::default(),
            card: Cell::new(false),
            shelves: RefCell::default(),
            view: Cell::new(view),
            tabs,
            about: RefCell::default(),
            selected: Cell::new(0),
            searching: Cell::new(false),
            scrolled,
            overlay,
            page,
            body,
            update: (update_line, update_label),
            search_bar,
            entry,
            add_button,
            search_toggle,
            save_button,
            section: Cell::new(0),
            sections,
            stats,
            stats_shown: Cell::new(false),
            switchers: [(hb, hs), (pb, ps)],
            title: (title, title_label),
            pill,
            narrow: Cell::new(false),
            help,
            about_app: RefCell::default(),
            progress,
            adding: Cell::new(false),
            waiting: RefCell::default(),
            question: RefCell::default(),
            naming: RefCell::default(),
            editing: RefCell::default(),
            settings: RefCell::default(),
            pending: Cell::new(0),
            first_frame: Cell::new(None),
            settled: RefCell::default(),
        });
        let weak = Rc::downgrade(&library);
        dismiss.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                let _ = update::dismiss(&mut l.store.borrow_mut());
                l.show_update();
            }
        });
        LIBRARY.with(|l| *l.borrow_mut() = Some(Rc::downgrade(&library)));
        library.load_covers();
        library.rebuild();
        library.connect();
        library.watch_first_frame();
        library.window.present();
        Some(library)
    }

    /// Asks GitHub for the latest release when a check is due, through `curl` in the background; the line follows the
    /// answer. Not for builds from a package manager (`share/forzac/packaged`), which it updates.
    pub fn check_update(self: &Rc<Self>) {
        thread_local!(static CHECKING: Cell<bool> = const { Cell::new(false) });
        if CHECKING.get() || crate::fonts::assets().join("packaged").exists() || !update::due(&self.store.borrow()) {
            return;
        }
        let args = ["curl", "-fsSL", "--max-time", "10", "-H", "Accept: application/vnd.github+json", update::RELEASES];
        let flags = gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE;
        // No curl: no check.
        let Ok(curl) = gio::Subprocess::newv(&args.map(std::ffi::OsStr::new), flags) else { return };
        CHECKING.set(true);
        let weak = Rc::downgrade(self);
        let store = self.store.clone();
        glib::spawn_future_local(async move {
            let answer = curl.communicate_utf8_future(None).await;
            CHECKING.set(false);
            let Ok((Some(answer), _)) = answer else { return };
            if update::record(&mut store.borrow_mut(), &answer)
                && let Some(l) = weak.upgrade()
            {
                l.show_update();
            }
        });
    }

    /// The update line: the offered version, out of the way while searching.
    fn show_update(&self) {
        let (line, label) = &self.update;
        let notice = update::notice(&self.store.borrow()).map(str::to_owned);
        line.set_visible(notice.is_some() && !self.searching.get());
        if let Some(version) = notice {
            let text = glib::markup_escape_text(&fill(tr("Version {version} is out"), &[("version", &version)]));
            let link = glib::markup_escape_text(tr("download"));
            label.set_markup(&format!("{text} — <a href=\"{}\">{link}</a>", update::PAGE));
        }
    }

    /// Runs `f` once the first frame is shown and every cover being loaded has arrived (`--measure`, `--png`).
    pub fn connect_settled(self: &Rc<Self>, f: impl FnOnce() + 'static) {
        self.settled.borrow_mut().push(Box::new(f));
        self.settle();
    }

    /// When the window's first frame was painted.
    pub fn first_frame(&self) -> Option<Instant> {
        self.first_frame.get()
    }

    /// Notes the window's first frame.
    fn watch_first_frame(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.window.connect_map(move |w| {
            let Some(clock) = w.frame_clock() else { return };
            let weak = weak.clone();
            let id: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
            let id2 = id.clone();
            *id.borrow_mut() = Some(clock.connect_after_paint(move |clock| {
                if let Some(id) = id2.borrow_mut().take() {
                    clock.disconnect(id);
                }
                if let Some(l) = weak.upgrade()
                    && l.first_frame.get().is_none()
                {
                    l.first_frame.set(Some(Instant::now()));
                    l.settle(); // may close the window (measurements)
                }
            }));
        });
    }

    fn settle(&self) {
        if self.first_frame.get().is_some() && self.pending.get() == 0 {
            let done: Vec<_> = self.settled.borrow_mut().drain(..).collect();
            for f in done {
                f();
            }
        }
    }

    // region covers

    /// Loads every cached thumbnail on worker threads; books without one get it made, one at a time.
    fn load_covers(self: &Rc<Self>) {
        let mut missing = Vec::new();
        for b in self.books.borrow().iter() {
            let path = thumbnail(&b.id);
            let thumb = match std::fs::metadata(&path) {
                Ok(m) if m.len() == 0 => Thumb::Generated,
                Ok(_) => {
                    self.load_thumbnail(b.id.clone());
                    Thumb::Waiting
                }
                Err(_) => {
                    missing.push((b.id.clone(), b.path.clone()));
                    Thumb::Waiting
                }
            };
            self.thumbs.borrow_mut().insert(b.id.clone(), thumb);
        }
        if missing.is_empty() {
            return;
        }
        self.pending.set(self.pending.get() + missing.len());
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            for (id, path) in missing {
                if weak.upgrade().is_none() {
                    return;
                }
                let made = id.clone();
                let _ = gio::spawn_blocking(move || {
                    let book = forzac_core::open(&path).ok();
                    let bytes = book.as_ref().and_then(|b| b.cover.as_ref().and_then(|c| b.images.get(c)).map(|i| i.bytes.as_slice()));
                    write_thumbnail(&made, bytes);
                })
                .await;
                let Some(library) = weak.upgrade() else { return };
                library.pending.set(library.pending.get() - 1);
                match std::fs::metadata(thumbnail(&id)) {
                    Ok(m) if m.len() > 0 => library.load_thumbnail(id),
                    _ => library.set_thumb(&id, Thumb::Generated),
                }
                library.settle();
            }
        });
    }

    fn load_thumbnail(self: &Rc<Self>, id: String) {
        self.pending.set(self.pending.get() + 1);
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let path = thumbnail(&id);
            let texture = gio::spawn_blocking(move || gdk::Texture::from_filename(path).ok()).await.ok().flatten();
            let Some(library) = weak.upgrade() else { return };
            library.pending.set(library.pending.get() - 1);
            library.set_thumb(&id, texture.map_or(Thumb::Generated, Thumb::Picture));
            library.settle();
        });
    }

    fn set_thumb(self: &Rc<Self>, id: &str, thumb: Thumb) {
        self.thumbs.borrow_mut().insert(id.to_owned(), thumb.clone());
        let books = self.books.borrow();
        for item in self.items.borrow().iter().filter(|i| books[i.book].id == id) {
            item.cover.set_thumb(thumb.clone());
            if matches!(thumb, Thumb::Generated) {
                self.marble_for(&item.cover);
            }
        }
    }

    /// Marbled paper for a generated cover that has it: made on a worker thread, kept by its seed.
    fn marble_for(self: &Rc<Self>, cover: &Cover) {
        let Some(style) = cover.imp().style.get().filter(|s| s.marble) else { return };
        if let Some(t) = self.marbles.borrow().get(&style.seed) {
            cover.set_marble(t.clone());
            return;
        }
        let (weak, cover) = (Rc::downgrade(self), cover.downgrade());
        glib::spawn_future_local(async move {
            let Ok(pixels) = gio::spawn_blocking(move || library::marble(style.seed, MARBLE.0, MARBLE.1)).await else { return };
            let texture: gdk::Texture = gdk::MemoryTexture::new(
                MARBLE.0 as i32,
                MARBLE.1 as i32,
                gdk::MemoryFormat::R8g8b8a8,
                &glib::Bytes::from_owned(pixels),
                MARBLE.0 * 4,
            )
            .upcast();
            if let Some(library) = weak.upgrade() {
                library.marbles.borrow_mut().insert(style.seed, texture.clone());
                // Covers built since with the same seed.
                for item in library.items.borrow().iter() {
                    if item.cover.imp().style.get().is_some_and(|s| s.marble && s.seed == style.seed) {
                        item.cover.set_marble(texture.clone());
                    }
                }
            }
            if let Some(cover) = cover.upgrade() {
                cover.set_marble(texture);
            }
        });
    }

    fn cover(self: &Rc<Self>, book: usize, size: Option<(i32, i32)>) -> Cover {
        let b = &self.books.borrow()[book];
        let thumb = self.thumbs.borrow().get(&b.id).cloned().unwrap_or_default();
        let generated = matches!(thumb, Thumb::Generated);
        let cover = Cover::new(b, thumb, self.palette.get(), size);
        if generated {
            self.marble_for(&cover);
        }
        cover
    }

    // endregion

    // region building

    /// Fills the window for the books and the search: the empty state, or the card and the shelf.
    fn rebuild(self: &Rc<Self>) {
        while let Some(child) = self.body.first_child() {
            self.body.remove(&child);
        }
        self.items.borrow_mut().clear();
        self.shelves.borrow_mut().clear();
        let searching = self.searching.get();
        let empty = self.books.borrow().is_empty();
        self.search_toggle.set_sensitive(!empty);
        self.show_update();
        self.show_tabs(!empty && !searching);
        if empty {
            self.body.append(&self.empty_state());
            return;
        }
        let books = self.books.borrow().clone();
        let series_name = |b: &Book| b.series.as_ref().map_or(String::new(), |s| s.name.clone());
        let view = self.view.get();
        let (card, groups): (Option<usize>, Vec<library::Group>) = if searching {
            let query = self.entry.text();
            let mut ranked: Vec<(u32, usize)> = books
                .iter()
                .enumerate()
                .filter_map(|(i, b)| library::rank(&query, &b.title, &b.authors, &series_name(b)).map(|r| (r, i)))
                .collect();
            ranked.sort_by_key(|&(r, _)| r);
            (None, vec![library::Group { heading: None, books: ranked.into_iter().map(|(_, i)| i).collect() }])
        } else if view == View::Recent {
            let card = library::continue_book(books.iter().map(|b| (b.progress, b.opened)));
            (card, vec![library::Group { heading: None, books: (0..books.len()).filter(|&i| Some(i) != card).collect() }])
        } else {
            let entries: Vec<Entry> = books.iter().map(Book::entry).collect();
            (None, library::arrange(&entries, view))
        };
        let order: Vec<usize> = groups.iter().flat_map(|g| g.books.iter().copied()).collect();
        self.body.append(&spacing(24));
        let mut items = Vec::new();
        if let Some(c) = card {
            let (widget, cover, more) = self.card_widget(c);
            self.body.append(&widget);
            items.push(Item { book: c, cover, widget, more });
            if !order.is_empty() {
                let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                rule.add_css_class("rule");
                rule.set_margin_top(28);
                rule.set_margin_bottom(28);
                self.body.append(&rule);
            }
        }
        self.card.set(card.is_some());
        if searching && order.is_empty() {
            let none = label(tr("No results"), "no-results");
            none.set_halign(gtk::Align::Center);
            none.set_margin_top(48);
            self.body.append(&none);
        }
        for (n, group) in groups.iter().enumerate() {
            if let Some(heading) = &group.heading {
                let heading = label(heading, "group-heading");
                heading.set_margin_top(if n == 0 { 0 } else { 40 });
                heading.set_margin_bottom(16);
                self.body.append(&heading);
            }
            let shelf = Layout::new(Kind::Shelf);
            for &i in &group.books {
                let (widget, cover, more) = self.shelf_item(i, view != View::Series || searching);
                shelf.add(&widget);
                items.push(Item { book: i, cover, widget, more });
            }
            self.body.append(&shelf);
            self.shelves.borrow_mut().push(shelf);
        }
        *self.items.borrow_mut() = items;
        let n = self.items.borrow().len();
        self.select(self.selected.get().min(n.saturating_sub(1)));
    }

    /// The Android app's "Continue reading" card.
    fn card_widget(self: &Rc<Self>, book: usize) -> (gtk::Widget, Cover, gtk::Button) {
        let b = self.books.borrow()[book].clone();
        let cover = self.cover(book, Some((92, 138)));
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        row.append(&cover);
        let text = gtk::Box::new(gtk::Orientation::Vertical, 6);
        text.set_valign(gtk::Align::Center);
        text.set_hexpand(true);
        text.append(&label(&tr("Continue reading").to_uppercase(), "card-caption"));
        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let title = lines(&b.title, "card-title", 2);
        title.set_hexpand(true);
        title_row.append(&title);
        let more = self.more_button(book);
        title_row.append(&more);
        text.append(&title_row);
        if !b.authors.is_empty() {
            let authors = lines(&b.authors, "card-authors", 2);
            text.append(&authors);
        }
        let bar = gtk::ProgressBar::new();
        bar.set_fraction(b.progress.clamp(0.0, 1.0) as f64);
        bar.set_margin_top(4);
        text.append(&bar);
        text.append(&label(&library::share_label(b.progress, b.opened), "shelf-share"));
        row.append(&text);
        self.connect_item(&row, &cover, &more, book);
        (row.upcast(), cover, more)
    }

    /// A book on the shelf: its cover, the title with the "⋯" menu, the authors, the series (`with_series`) and the
    /// share read.
    fn shelf_item(self: &Rc<Self>, book: usize, with_series: bool) -> (gtk::Widget, Cover, gtk::Button) {
        let b = self.books.borrow()[book].clone();
        let cover = self.cover(book, None);
        let item = gtk::Box::new(gtk::Orientation::Vertical, 0);
        item.append(&cover);
        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        title_row.set_margin_top(12);
        let title = lines(&b.title, "shelf-title", 3);
        title.set_hexpand(true);
        title_row.append(&title);
        let more = self.more_button(book);
        more.set_valign(gtk::Align::Start);
        title_row.append(&more);
        item.append(&title_row);
        if !b.authors.is_empty() {
            let authors = lines(&b.authors, "shelf-author", 2);
            authors.set_margin_top(2);
            item.append(&authors);
        }
        if let Some(series) = b.series.as_ref().filter(|_| with_series) {
            let text = match series.number_label() {
                Some(n) => format!("{} · {n}", series.name),
                None => series.name.clone(),
            };
            let series = lines(&text, "shelf-author", 1);
            series.set_margin_top(2);
            item.append(&series);
        }
        let share = label(&library::share_label(b.progress, b.opened), "shelf-share");
        share.set_margin_top(4);
        item.append(&share);
        self.connect_item(&item, &cover, &more, book);
        (item.upcast(), cover, more)
    }

    /// The book's "⋯": its menu (`library-metadata`).
    fn more_button(self: &Rc<Self>, book: usize) -> gtk::Button {
        let more = gtk::Button::builder().label("⋯").tooltip_text(tr("More")).focusable(false).build();
        more.add_css_class("remove");
        more.set_opacity(0.0);
        let weak = Rc::downgrade(self);
        more.connect_clicked(move |b| {
            if let Some(l) = weak.upgrade() {
                l.menu(book, b);
            }
        });
        more
    }

    /// The book's menu at `anchor`: "About the book", "Remove from library".
    fn menu(self: &Rc<Self>, book: usize, anchor: &gtk::Button) {
        let model = gio::Menu::new();
        model.append(Some(tr("About the book")), Some(&format!("lib.about(uint32 {book})")));
        model.append(Some(tr("Edit…")), Some(&format!("lib.edit(uint32 {book})")));
        // "Add to shelf": a checked item per shelf, its action made for this menu.
        let shelves = gio::Menu::new();
        let actions = gio::SimpleActionGroup::new();
        let Some(id) = self.books.borrow().get(book).map(|b| b.id.clone()) else { return };
        let names = self.store.borrow().shelf_names();
        for (i, name) in names.into_iter().enumerate() {
            let on = self.store.borrow().shelves_of(&id).iter().any(|s| s.to_lowercase() == name.to_lowercase());
            let action = gio::SimpleAction::new_stateful(&format!("s{i}"), None, &on.to_variant());
            shelves.append(Some(&name), Some(&format!("shelf.s{i}")));
            let (weak, id) = (Rc::downgrade(self), id.clone());
            action.connect_activate(move |_, _| {
                if let Some(l) = weak.upgrade() {
                    l.set_shelf(&id, &name, !on);
                }
            });
            actions.add_action(&action);
        }
        let section = gio::Menu::new();
        section.append(Some(tr("New shelf…")), Some(&format!("lib.newshelf(uint32 {book})")));
        shelves.append_section(None, &section);
        model.append_submenu(Some(tr("Add to shelf")), &shelves);
        model.append(Some(tr("Send to Kindle")), Some(&format!("lib.kindle(uint32 {book})")));
        model.append(Some(tr("Remove from library")), Some(&format!("lib.remove(uint32 {book})")));
        self.window.insert_action_group("shelf", Some(&actions));
        let popover = gtk::PopoverMenu::from_model(Some(&model));
        popover.set_parent(anchor);
        popover.set_has_arrow(false);
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        popover.popup();
    }

    /// "Change the Books folder…": the folder dialog, then "Move the books…?" (Cancel, Leave them, Move); the change
    /// runs in the background (`library::change_folder`) and the library shows the new folder's books.
    fn choose_books_folder(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title(tr("Change the Books folder…")).modal(true).build();
        let weak = Rc::downgrade(self);
        dialog.select_folder(Some(&self.window), gio::Cancellable::NONE, move |dir| {
            let (Some(l), Some(new)) = (weak.upgrade(), dir.ok().and_then(|d| d.path())) else { return };
            let old = l.folder.borrow().clone();
            if old.as_ref().is_some_and(|o| o.canonicalize().ok() == new.canonicalize().ok()) {
                return;
            }
            let name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let question = gtk::AlertDialog::builder()
                .message(fill(tr("Move the books to “{folder}”?"), &[("folder", &name(&new))]))
                .detail(fill(
                    tr("The books in “{folder}” go to the new folder; where you stopped in each stays."),
                    &[("folder", &old.as_deref().map(name).unwrap_or_default())],
                ))
                .buttons([tr("Cancel"), tr("Leave them"), tr("Move")])
                .cancel_button(0)
                .default_button(2)
                .modal(true)
                .build();
            let weak = Rc::downgrade(&l);
            question.choose(Some(&l.window), gio::Cancellable::NONE, move |answer| {
                if let (Some(l), Ok(answer @ 1..=2)) = (weak.upgrade(), answer) {
                    l.change_folder(new, answer == 2);
                }
            });
        });
    }

    fn change_folder(self: &Rc<Self>, new: PathBuf, move_books: bool) {
        let (state, weak, target) = (self.store.borrow().path().to_owned(), Rc::downgrade(self), new.clone());
        glib::spawn_future_local(async move {
            let done = gio::spawn_blocking(move || {
                let mut s = Store::open(state);
                library::change_folder(&target, &mut s, move_books).map(|moved| (moved, library::scan(&target, &mut s)))
            })
            .await
            .unwrap_or(Err(forzac_core::Error::Unavailable));
            let Some(l) = weak.upgrade() else { return };
            l.store.borrow_mut().reload();
            l.read_sync_status();
            let message = match done {
                Ok((moved, books)) => {
                    *l.folder.borrow_mut() = Some(new);
                    *l.books.borrow_mut() = books.iter().map(Book::of).collect();
                    l.selected.set(0);
                    l.load_covers();
                    l.rebuild();
                    moved.message()
                }
                Err(_) => {
                    let name = new.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    Some(fill(tr("Couldn't open “{name}”: {reason}"), &[("name", &name), ("reason", tr("the file is unavailable"))]))
                }
            };
            if let Some(message) = message {
                gtk::AlertDialog::builder()
                    .message(message)
                    .buttons([tr("Close")])
                    .cancel_button(0)
                    .modal(true)
                    .build()
                    .show(Some(&l.window));
            }
        });
    }

    /// Puts the book `id` on the shelf `name` or takes it off, and shows it.
    fn set_shelf(self: &Rc<Self>, id: &str, name: &str, on: bool) {
        if let Err(e) = self.store.borrow_mut().set_shelf(id, name, on) {
            eprintln!("cannot save the reading state: {e}");
            return;
        }
        let shelves = self.store.borrow().shelves_of(id).to_vec();
        for b in self.books.borrow_mut().iter_mut().filter(|b| b.id == id) {
            b.shelves = shelves.clone();
        }
        if self.view.get() == View::Shelves {
            self.rebuild();
        }
    }

    /// "New shelf": a sheet at the window's bottom with the name field, "Cancel" and "Add".
    fn ask_shelf(self: &Rc<Self>, book: usize) {
        if self.naming.borrow().is_some() || self.question.borrow().is_some() || self.about.borrow().is_some() {
            return;
        }
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("backdrop");
        let sheet = gtk::Box::new(gtk::Orientation::Vertical, 10);
        sheet.add_css_class("question");
        sheet.set_valign(gtk::Align::End);
        sheet.set_halign(gtk::Align::Center);
        sheet.set_vexpand(true);
        sheet.set_size_request(self.window.width().min(560), -1);
        sheet.append(&label(tr("New shelf"), "question-title"));
        let entry = gtk::Entry::builder().placeholder_text(tr("Shelf name")).build();
        sheet.append(&entry);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_homogeneous(true);
        buttons.set_margin_top(8);
        let cancel = gtk::Button::builder().label(tr("Cancel")).focusable(false).hexpand(true).build();
        let add = gtk::Button::builder().label(tr("Add")).focusable(false).hexpand(true).build();
        cancel.add_css_class("sheet-button");
        add.add_css_class("sheet-button");
        add.add_css_class("filled");
        buttons.append(&cancel);
        buttons.append(&add);
        sheet.append(&buttons);
        backdrop.append(&sheet);
        let weak = Rc::downgrade(self);
        cancel.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.name_shelf(false);
            }
        });
        let weak = Rc::downgrade(self);
        add.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.name_shelf(true);
            }
        });
        self.overlay.add_overlay(&backdrop);
        entry.grab_focus();
        *self.naming.borrow_mut() = Some((backdrop.upcast(), book, entry));
    }

    /// Closes the new shelf's question; puts the book on the named shelf when `add`.
    fn name_shelf(self: &Rc<Self>, add: bool) {
        let Some((sheet, book, entry)) = self.naming.borrow_mut().take() else { return };
        self.overlay.remove_overlay(&sheet);
        let id = self.books.borrow().get(book).map(|b| b.id.clone());
        if let Some(id) = id.filter(|_| add) {
            self.set_shelf(&id, &entry.text(), true);
        }
    }

    /// "Edit…": the book's file read in the background, then the form as a sheet at the window's bottom.
    fn edit(self: &Rc<Self>, book: usize) {
        if self.editing.borrow().is_some() || self.question.borrow().is_some() || self.about.borrow().is_some() {
            return;
        }
        let Some(path) = self.books.borrow().get(book).map(|b| b.path.clone()) else { return };
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let parsed = gio::spawn_blocking(move || forzac_core::open(&path).ok()).await.ok().flatten();
            let (Some(l), Some(parsed)) = (weak.upgrade(), parsed) else { return };
            if l.editing.borrow().is_none() && l.question.borrow().is_none() && l.about.borrow().is_none() {
                l.show_edit(book, parsed);
            }
        });
    }

    fn show_edit(self: &Rc<Self>, book: usize, parsed: forzac_core::Book) {
        let form = library::fields(&parsed, &self.store.borrow().edits_of(&parsed.id));
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("backdrop");
        let sheet = gtk::Box::new(gtk::Orientation::Vertical, 10);
        sheet.add_css_class("question");
        sheet.set_valign(gtk::Align::End);
        sheet.set_halign(gtk::Align::Center);
        sheet.set_vexpand(true);
        sheet.set_size_request(self.window.width().min(560), -1);
        sheet.append(&label(tr("Edit the book"), "question-title"));
        let grid = gtk::Grid::builder().row_spacing(8).column_spacing(12).build();
        let values = [&form.title, &form.authors, &form.series, &form.number, &form.genres];
        let names = [tr("Title"), tr("Authors"), tr("Series"), tr("Number"), tr("Genres")];
        let entries: [gtk::Entry; 5] = std::array::from_fn(|i| {
            let name = label(names[i], "question-body");
            name.set_valign(gtk::Align::Center);
            let entry = gtk::Entry::builder().text(values[i].as_str()).hexpand(true).build();
            grid.attach(&name, 0, i as i32, 1, 1);
            grid.attach(&entry, 1, i as i32, 1, 1);
            entry
        });
        sheet.append(&grid);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_homogeneous(true);
        buttons.set_margin_top(8);
        let own = gtk::Button::builder().label(tr("From the book")).focusable(false).hexpand(true).build();
        let cancel = gtk::Button::builder().label(tr("Cancel")).focusable(false).hexpand(true).build();
        let save = gtk::Button::builder().label(tr("Save")).focusable(false).hexpand(true).build();
        for b in [&own, &cancel, &save] {
            b.add_css_class("sheet-button");
            buttons.append(b);
        }
        save.add_css_class("filled");
        sheet.append(&buttons);
        backdrop.append(&sheet);
        for (button, answer) in [(&own, Some(false)), (&cancel, None), (&save, Some(true))] {
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(l) = weak.upgrade() {
                    l.close_edit(answer);
                }
            });
        }
        self.overlay.add_overlay(&backdrop);
        entries[0].grab_focus();
        *self.editing.borrow_mut() = Some((backdrop.upcast(), book, entries, parsed));
    }

    /// Closes the form: `Some(true)` keeps what it changes, `Some(false)` drops the book's edits, `None` changes nothing.
    fn close_edit(self: &Rc<Self>, answer: Option<bool>) {
        let Some((sheet, book, entries, parsed)) = self.editing.borrow_mut().take() else { return };
        self.overlay.remove_overlay(&sheet);
        let Some(save) = answer else { return };
        let edits = if save {
            let text = |i: usize| entries[i].text().to_string();
            let form = library::Fields { title: text(0), authors: text(1), series: text(2), number: text(3), genres: text(4) };
            library::edits_from(&parsed, &form)
        } else {
            forzac_core::state::Edits::default()
        };
        if let Err(e) = self.store.borrow_mut().set_edits(&parsed.id, edits.clone()) {
            eprintln!("cannot save the reading state: {e}");
            return;
        }
        // The library's book as it now reads: the file's details with the edits.
        let shown = library::fields(&parsed, &edits);
        for b in self.books.borrow_mut().iter_mut().filter(|b| b.id == parsed.id) {
            b.title = shown.title.clone();
            b.authors = shown.authors.clone();
            b.series = Series::parse(&shown.series, Some(&shown.number));
        }
        self.rebuild();
        self.select(book.min(self.items.borrow().len().saturating_sub(1)));
    }

    /// "Send to Kindle" (`send-to-kindle`): converted and written in the background, then the message.
    fn send_to_kindle(self: &Rc<Self>, book: usize) {
        let Some(b) = self.books.borrow().get(book).cloned() else { return };
        let edits = self.store.borrow().edits_of(&b.id);
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let (path, title) = (b.path.clone(), b.title.clone());
            let message = gio::spawn_blocking(move || forzac_core::kindle::send_to_kindle(&path, &edits, &title))
                .await
                .unwrap_or_else(|_| tr("Couldn't write to the Kindle.").to_owned());
            if let Some(l) = weak.upgrade() {
                gtk::AlertDialog::builder()
                    .message(message)
                    .buttons([tr("Close")])
                    .cancel_button(0)
                    .modal(true)
                    .build()
                    .show(Some(&l.window));
            }
        });
    }

    /// The views' tabs, the chosen one marked; hidden while searching or without books.
    fn show_tabs(self: &Rc<Self>, visible: bool) {
        while let Some(child) = self.tabs.first_child() {
            self.tabs.remove(&child);
        }
        self.tabs.set_visible(visible);
        for view in View::ALL {
            let tab = gtk::Button::builder().label(view.label()).focusable(false).build();
            tab.add_css_class("view-tab");
            if view == self.view.get() {
                tab.add_css_class("active");
            }
            let weak = Rc::downgrade(self);
            tab.connect_clicked(move |_| {
                if let Some(l) = weak.upgrade() {
                    l.set_view(view);
                }
            });
            self.tabs.append(&tab);
        }
    }

    /// Shows `view` and remembers it.
    fn set_view(self: &Rc<Self>, view: View) {
        self.view.set(view);
        if let Err(e) = self.store.borrow_mut().set_setting(library::VIEW, view.key()) {
            eprintln!("cannot save the reading state: {e}");
        }
        self.selected.set(0);
        self.rebuild();
        self.scrolled.vadjustment().set_value(0.0);
    }

    /// "About the book": the book's file read in the background, then a sheet over the dimmed library.
    fn about(self: &Rc<Self>, book: usize) {
        if self.about.borrow().is_some() || self.question.borrow().is_some() {
            return;
        }
        let Some((path, id)) = self.books.borrow().get(book).map(|b| (b.path.clone(), b.id.clone())) else { return };
        let edits = self.store.borrow().edits_of(&id);
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let info =
                gio::spawn_blocking(move || forzac_core::open(&path).ok().map(|b| library::book_info(&b, &edits))).await.ok().flatten();
            let (Some(l), Some(info)) = (weak.upgrade(), info) else { return };
            if l.about.borrow().is_none() && l.question.borrow().is_none() {
                l.show_about(book, info);
            }
        });
    }

    fn show_about(self: &Rc<Self>, book: usize, info: library::BookInfo) {
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("backdrop");
        let sheet = gtk::Box::new(gtk::Orientation::Vertical, 16);
        sheet.add_css_class("about");
        sheet.set_valign(gtk::Align::Center);
        sheet.set_halign(gtk::Align::Center);
        sheet.set_vexpand(true);
        sheet.set_size_request((self.window.width() - 48).clamp(280, 600), -1);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 20);
        top.append(&self.cover(book, Some((100, 150))));
        let text = gtk::Box::new(gtk::Orientation::Vertical, 6);
        text.set_hexpand(true);
        text.set_valign(gtk::Align::Center);
        for (value, class) in [
            (&info.title, "about-title"),
            (&info.authors, "about-authors"),
            (&info.series, "about-series"),
            (&info.details, "about-details"),
        ] {
            if !value.is_empty() {
                text.append(&label(value, class));
            }
        }
        top.append(&text);
        sheet.append(&top);
        if !info.annotation.is_empty() {
            let annotation = label(&info.annotation, "about-annotation");
            let scroll = gtk::ScrolledWindow::builder().child(&annotation).hscrollbar_policy(gtk::PolicyType::Never).build();
            scroll.set_propagate_natural_height(true);
            scroll.set_max_content_height(260);
            sheet.append(&scroll);
        }
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_homogeneous(true);
        let close = gtk::Button::builder().label(tr("Close")).focusable(false).hexpand(true).build();
        let read = gtk::Button::builder().label(tr("Read")).focusable(false).hexpand(true).build();
        close.add_css_class("sheet-button");
        read.add_css_class("sheet-button");
        read.add_css_class("filled");
        buttons.append(&close);
        buttons.append(&read);
        sheet.append(&buttons);
        backdrop.append(&sheet);
        let weak = Rc::downgrade(self);
        close.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.close_about(false);
            }
        });
        let weak = Rc::downgrade(self);
        read.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.close_about(true);
            }
        });
        let click = gtk::GestureClick::new();
        let (weak, s) = (Rc::downgrade(self), sheet.clone());
        click.connect_released(move |_, _, x, y| {
            let inside =
                s.compute_bounds(&s.parent().unwrap()).is_some_and(|b| b.contains_point(&graphene::Point::new(x as f32, y as f32)));
            if !inside && let Some(l) = weak.upgrade() {
                l.close_about(false);
            }
        });
        backdrop.add_controller(click);
        self.overlay.add_overlay(&backdrop);
        *self.about.borrow_mut() = Some((backdrop.upcast(), book));
    }

    /// The settings (`settings`): a sheet over the dimmed library with the language and syncing.
    fn show_settings(self: &Rc<Self>) {
        if self.settings.borrow().is_some() || self.about.borrow().is_some() || self.question.borrow().is_some() {
            return;
        }
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("backdrop");
        let sheet = gtk::Box::new(gtk::Orientation::Vertical, 8);
        sheet.add_css_class("about");
        // The sheet scrolls when the window is shorter than it.
        let frame = gtk::ScrolledWindow::builder()
            .child(&sheet)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height((self.window.height() - 48).max(200))
            .vexpand(true)
            .build();
        if self.narrow.get() {
            // A narrow window: from the bottom edge across the whole width.
            sheet.add_css_class("bottom-sheet");
            frame.set_valign(gtk::Align::End);
            frame.set_halign(gtk::Align::Fill);
        } else {
            frame.set_valign(gtk::Align::Center);
            frame.set_halign(gtk::Align::Center);
            frame.set_size_request((self.window.width() - 48).clamp(280, 520), -1);
        }
        sheet.append(&label(tr("Settings"), "about-title"));

        // The six reading themes as "Aa" shows them; choosing one is the reading theme itself.
        sheet.append(&label(tr("Theme"), "settings-heading"));
        let current = Settings::load(&self.store.borrow()).theme_index(crate::appearance::dark());
        let row =
            gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None).max_children_per_line(6).min_children_per_line(3).build();
        row.set_column_spacing(8);
        row.set_row_spacing(8);
        let names = [tr("Light"), tr("Sepia"), tr("Dark"), tr("Dawn"), tr("Wave"), tr("Forest")];
        let mut swatches = Vec::new();
        for (i, palette) in THEMES.iter().enumerate() {
            let swatch = gtk::DrawingArea::builder().content_width(56).content_height(44).build();
            let p = *palette;
            swatch.set_draw_func(move |_, cr, w, h| {
                let (w, h) = (w as f64, h as f64);
                let c = |rgb: u32, shift: u32| ((rgb >> shift) & 0xFF) as f64 / 255.0;
                cr.new_sub_path();
                let r = 10.0;
                cr.arc(w - r - 1.0, r + 1.0, r, -std::f64::consts::FRAC_PI_2, 0.0);
                cr.arc(w - r - 1.0, h - r - 1.0, r, 0.0, std::f64::consts::FRAC_PI_2);
                cr.arc(r + 1.0, h - r - 1.0, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
                cr.arc(r + 1.0, r + 1.0, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
                cr.close_path();
                cr.set_source_rgb(c(p.paper, 16), c(p.paper, 8), c(p.paper, 0));
                let _ = cr.fill_preserve();
                cr.set_source_rgb(c(p.line, 16), c(p.line, 8), c(p.line, 0));
                cr.set_line_width(1.0);
                let _ = cr.stroke();
                let layout = pangocairo::functions::create_layout(cr);
                layout.set_font_description(Some(&font("Literata", 17.0)));
                layout.set_text("Aa");
                let (tw, th) = layout.pixel_size();
                cr.set_source_rgb(c(p.ink, 16), c(p.ink, 8), c(p.ink, 0));
                cr.move_to((w - tw as f64) / 2.0, (h - th as f64) / 2.0);
                pangocairo::functions::show_layout(cr, &layout);
            });
            let column = gtk::Box::new(gtk::Orientation::Vertical, 4);
            column.append(&swatch);
            column.append(&gtk::Label::new(Some(names[i])));
            let button = gtk::Button::builder().child(&column).build();
            button.add_css_class("swatch");
            if i == current {
                button.add_css_class("chosen");
            }
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(l) = weak.upgrade() {
                    l.choose_theme(i);
                }
            });
            row.append(&button);
            swatches.push(button);
        }
        sheet.append(&row);

        sheet.append(&label(tr("Language"), "settings-heading"));
        // The languages in their own words, whatever the interface's.
        let language = gtk::DropDown::from_strings(&[tr("As the system"), "English", "Русский"]);
        let chosen = match self.store.borrow().setting(i18n::SETTING) {
            Some("en") => 1,
            Some("ru") => 2,
            _ => 0,
        };
        language.set_selected(chosen);
        language.set_halign(gtk::Align::Start);
        let store = self.store.clone();
        language.connect_selected_notify(move |d| {
            let code = match d.selected() {
                1 => i18n::Language::English.code(),
                2 => i18n::Language::Russian.code(),
                _ => "",
            };
            if let Err(e) = store.borrow_mut().set_setting(i18n::SETTING, code) {
                eprintln!("cannot save the reading state: {e}");
            }
        });
        sheet.append(&language);
        sheet.append(&label(tr("The language changes after a restart."), "question-body"));

        sheet.append(&label(tr("Sync"), "settings-heading"));
        let explanation = label(
            tr(
                "Keep the Books folder in Syncthing, Dropbox or iCloud Drive, and every device opens your books where you left off. On a phone, only Syncthing works.",
            ),
            "question-body",
        );
        // A wrapped label would ask for its whole text's width and widen the sheet past its size.
        explanation.set_max_width_chars(40);
        sheet.append(&explanation);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_margin_top(4);
        let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let weak = Rc::downgrade(self);
        let show_folder = move || {
            if let Some(l) = weak.upgrade() {
                l.show_folder();
            }
        };
        column.set_hexpand(true);
        column.set_valign(gtk::Align::Center);
        column.append(&label(tr("Books folder"), "question-body"));
        let folder = gtk::Label::builder().xalign(0.0).ellipsize(pango::EllipsizeMode::Middle).build();
        column.append(&folder);
        row.append(&column);
        let show = gtk::Button::builder().label(tr("Show")).focusable(false).valign(gtk::Align::Center).build();
        show.add_css_class("sheet-button");
        show.add_css_class("small");
        row.append(&show);
        let change = gtk::Button::builder().label(tr("Change…")).focusable(false).valign(gtk::Align::Center).build();
        change.add_css_class("sheet-button");
        change.add_css_class("small");
        row.append(&change);
        sheet.append(&row);
        let status = label("", "about-authors");
        sheet.append(&status);
        sheet.append(&label(tr("Other devices"), "question-body"));
        let devices = gtk::Box::new(gtk::Orientation::Vertical, 2);
        sheet.append(&devices);

        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_homogeneous(true);
        buttons.set_margin_top(12);
        let close = gtk::Button::builder().label(tr("Close")).focusable(false).hexpand(true).build();
        let sync = gtk::Button::builder().label(tr("Sync Now")).focusable(false).hexpand(true).build();
        close.add_css_class("sheet-button");
        sync.add_css_class("sheet-button");
        sync.add_css_class("filled");
        buttons.append(&close);
        buttons.append(&sync);
        sheet.append(&buttons);
        backdrop.append(&frame);

        show.connect_clicked(move |_| show_folder());
        let weak = Rc::downgrade(self);
        change.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.choose_books_folder();
            }
        });
        let weak = Rc::downgrade(self);
        close.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.close_settings();
            }
        });
        let weak = Rc::downgrade(self);
        sync.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.sync_now();
            }
        });
        let click = gtk::GestureClick::new();
        let (weak, s) = (Rc::downgrade(self), frame.clone());
        click.connect_released(move |_, _, x, y| {
            let inside =
                s.compute_bounds(&s.parent().unwrap()).is_some_and(|b| b.contains_point(&graphene::Point::new(x as f32, y as f32)));
            if !inside && let Some(l) = weak.upgrade() {
                l.close_settings();
            }
        });
        backdrop.add_controller(click);
        self.overlay.add_overlay(&backdrop);
        *self.settings.borrow_mut() = Some(SettingsSheet { sheet: backdrop.upcast(), swatches, folder, status, devices, sync });
        self.read_sync_status();
    }

    /// A theme swatch chosen: the chosen mark moves, the theme applies at once.
    fn choose_theme(self: &Rc<Self>, theme: usize) {
        if let Some(s) = self.settings.borrow().as_ref() {
            for (i, b) in s.swatches.iter().enumerate() {
                if i == theme { b.add_css_class("chosen") } else { b.remove_css_class("chosen") }
            }
        }
        self.apply_theme(theme);
    }

    /// The keys in the settings: the focus moves along the swatches (from the chosen one), `Return` / `Space` choose.
    fn move_swatch(&self, delta: i32) {
        let settings = self.settings.borrow();
        let Some(s) = settings.as_ref() else { return };
        let at = s.swatches.iter().position(|b| b.has_focus()).or_else(|| s.swatches.iter().position(|b| b.has_css_class("chosen")));
        let next = at.map_or(0, |i| (i as i32 + delta).clamp(0, s.swatches.len() as i32 - 1) as usize);
        s.swatches[next].grab_focus();
    }

    fn choose_swatch(self: &Rc<Self>) {
        let focused = self.settings.borrow().as_ref().and_then(|s| s.swatches.iter().position(|b| b.has_focus()));
        if let Some(i) = focused {
            self.choose_theme(i);
        }
    }

    fn toggle_help(&self) {
        if self.help.is_open() { self.help.close() } else { self.help.show() }
    }

    fn close_settings(self: &Rc<Self>) {
        if let Some(settings) = self.settings.borrow_mut().take() {
            self.overlay.remove_overlay(&settings.sheet);
        }
    }

    /// The settings' sync status (`sync::status`), read off the main thread: the other devices' files may be slow.
    fn read_sync_status(self: &Rc<Self>) {
        if self.settings.borrow().is_none() {
            return;
        }
        let (state, weak) = (self.store.borrow().path().to_owned(), Rc::downgrade(self));
        let offset = glib::DateTime::now_local().map_or(0, |d| d.utc_offset().as_seconds());
        glib::spawn_future_local(async move {
            let status = gio::spawn_blocking(move || forzac_core::sync::status(&Store::open(state), offset)).await;
            let (Some(l), Ok(status)) = (weak.upgrade(), status) else { return };
            let settings = l.settings.borrow();
            let Some(s) = settings.as_ref() else { return };
            let path = status.folder.unwrap_or_default();
            let shown = match path.strip_prefix(glib::home_dir()) {
                Ok(rest) => Path::new("~").join(rest),
                Err(_) => path.clone(),
            };
            s.folder.set_text(&shown.to_string_lossy());
            s.folder.set_tooltip_text(Some(&path.to_string_lossy()));
            s.status.set_text(&status.state);
            s.sync.set_sensitive(true);
            while let Some(child) = s.devices.first_child() {
                s.devices.remove(&child);
            }
            for (name, when) in &status.devices {
                s.devices.append(&label(&format!("{name} · {when}"), "about-authors"));
            }
            if status.devices.is_empty() {
                s.devices.append(&label(tr("None yet"), "about-authors"));
            }
        });
    }

    /// "Sync Now": the sync a book window runs when it is left (`sync`), then the status again.
    fn sync_now(self: &Rc<Self>) {
        if let Some(s) = self.settings.borrow().as_ref() {
            s.status.set_text(tr("Syncing…"));
            s.sync.set_sensitive(false);
        }
        let (state, weak) = (self.store.borrow().path().to_owned(), Rc::downgrade(self));
        glib::spawn_future_local(async move {
            let _ = gio::spawn_blocking(move || crate::sync_state(&state)).await;
            let Some(l) = weak.upgrade() else { return };
            l.store.borrow_mut().reload();
            l.read_sync_status();
        });
    }

    /// Closes "About the book"; opens the book when `read`.
    fn close_about(self: &Rc<Self>, read: bool) {
        let Some((sheet, book)) = self.about.borrow_mut().take() else { return };
        self.overlay.remove_overlay(&sheet);
        if read {
            self.open_book(book);
        }
    }

    /// A click opens the book, a right click opens its menu; under the pointer its cover lifts and its "⋯" shows.
    fn connect_item(self: &Rc<Self>, item: &gtk::Box, cover: &Cover, more: &gtk::Button, book: usize) {
        let right = gtk::GestureClick::builder().button(3).build();
        let (weak, m) = (Rc::downgrade(self), more.clone());
        right.connect_pressed(move |g, _, _, _| {
            if let Some(l) = weak.upgrade() {
                g.set_state(gtk::EventSequenceState::Claimed);
                m.set_opacity(1.0);
                l.menu(book, &m);
            }
        });
        item.add_controller(right);
        let click = gtk::GestureClick::new();
        let weak = Rc::downgrade(self);
        click.connect_released(move |g, n, _, _| {
            if n == 1
                && let Some(l) = weak.upgrade()
            {
                g.set_state(gtk::EventSequenceState::Claimed);
                l.open_book(book);
            }
        });
        item.add_controller(click);
        let motion = gtk::EventControllerMotion::new();
        let (c, r) = (cover.clone(), more.clone());
        motion.connect_enter(move |_, _, _| {
            c.set_hover(true);
            r.set_opacity(1.0);
        });
        let (c, r, weak) = (cover.clone(), more.clone(), Rc::downgrade(self));
        motion.connect_leave(move |_| {
            c.set_hover(false);
            // The selected book keeps its "⋯".
            let selected = weak.upgrade().is_some_and(|l| l.items.borrow().get(l.selected.get()).is_some_and(|i| i.more == r));
            r.set_opacity(if selected { 1.0 } else { 0.0 });
        });
        item.add_controller(motion);
        item.set_cursor_from_name(Some("pointer"));
    }

    /// The web's empty library: the invitation, the button, the hint, the Books folder and the formats beside the
    /// sample page.
    fn empty_state(self: &Rc<Self>) -> gtk::Widget {
        let text = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let heading = label(tr("Read books right on your computer"), "empty-heading");
        heading.set_max_width_chars(16);
        text.append(&heading);
        let lead = label(tr("Open a book file — a copy goes into your Books folder, the original stays where it is."), "empty-lead");
        lead.set_max_width_chars(34);
        lead.set_margin_top(24);
        lead.set_margin_bottom(36);
        text.append(&lead);
        let button = pill(tr("Open a book"), true);
        button.set_halign(gtk::Align::Start);
        let weak = Rc::downgrade(self);
        button.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.choose_file();
            }
        });
        text.append(&button);
        let hint = label(tr("or drop a file into this window"), "hint");
        hint.set_margin_top(14);
        text.append(&hint);
        if self.folder.borrow().is_some() {
            let show = gtk::Button::builder().label(tr("Show the Books folder")).focusable(false).halign(gtk::Align::Start).build();
            show.add_css_class("folder-link");
            show.set_margin_top(6);
            let weak = Rc::downgrade(self);
            show.connect_clicked(move |_| {
                if let Some(l) = weak.upgrade() {
                    l.show_folder();
                }
            });
            text.append(&show);
        }
        let formats = label(tr("FB2, FB2.ZIP, FB3, EPUB, TXT and Markdown are supported."), "privacy");
        formats.set_max_width_chars(48);
        formats.set_margin_top(42);
        text.append(&formats);
        let split = Layout::new(Kind::Split);
        split.add(&text);
        split.add(&Sample::new(self.palette.get()));
        split.set_vexpand(true);
        split.set_valign(gtk::Align::Center);
        split.set_margin_top(32);
        split.set_margin_bottom(32);
        split.upcast()
    }

    // endregion

    // region actions

    fn select(&self, index: usize) {
        let items = self.items.borrow();
        for (i, item) in items.iter().enumerate() {
            item.cover.set_selected(i == index);
            item.more.set_opacity(if i == index { 1.0 } else { 0.0 });
        }
        self.selected.set(index);
        let Some(item) = items.get(index) else { return };
        // Into view, with a margin for the ring and the title under the cover.
        let Some(child) = self.scrolled.child() else { return };
        let Some(bounds) = item.widget.compute_bounds(&child) else { return };
        let adj = self.scrolled.vadjustment();
        let (top, bottom) = (bounds.y() as f64 - 24.0, (bounds.y() + bounds.height()) as f64 + 24.0);
        if top < adj.value() {
            adj.set_value(top);
        } else if bottom > adj.value() + adj.page_size() {
            adj.set_value(bottom - adj.page_size());
        }
    }

    /// Moves the selection: `dx` along the books, `dy` rows through the card and the groups' shelves
    /// (`library::step`).
    fn step(&self, dx: i32, dy: i32) {
        let mut groups: Vec<(usize, usize)> = if self.card.get() { vec![(1, 1)] } else { Vec::new() };
        groups.extend(self.shelves.borrow().iter().map(|s| (s.children().len(), s.imp().columns.get().max(1))));
        self.select(library::step(&groups, self.selected.get(), dx, dy));
    }

    fn open_book(&self, book: usize) {
        let path = self.books.borrow().get(book).map(|b| b.path.clone());
        if let Some(path) = path {
            (self.open)(&path);
        }
    }

    fn open_selected(&self) {
        let book = self.items.borrow().get(self.selected.get()).map(|i| i.book);
        if let Some(b) = book {
            self.open_book(b);
        }
    }

    /// Opens the Books folder in the file manager.
    fn show_folder(&self) {
        let Some(folder) = self.folder.borrow().clone() else { return };
        gtk::FileLauncher::new(Some(&gio::File::for_path(&folder))).launch(Some(&self.window), gio::Cancellable::NONE, |r| {
            if let Err(e) = r {
                eprintln!("cannot show the Books folder: {e}");
            }
        });
    }

    fn choose_file(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(tr("Books")));
        for suffix in ["fb2", "zip", "fb3", "epub", "txt", "md", "markdown"] {
            filter.add_suffix(suffix);
        }
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        let dialog = gtk::FileDialog::builder().filters(&filters).default_filter(&filter).modal(true).build();
        let open = self.open.clone();
        dialog.open(Some(&self.window), gio::Cancellable::NONE, move |file| {
            if let Some(path) = file.ok().and_then(|f| f.path()) {
                open(&path);
            }
        });
    }

    /// Asks for a folder to add (`folder-import`).
    fn choose_folder(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title(tr("Add a Folder…")).modal(true).build();
        let weak = Rc::downgrade(self);
        dialog.select_folder(Some(&self.window), gio::Cancellable::NONE, move |dir| {
            if let (Some(l), Some(dir)) = (weak.upgrade(), dir.ok().and_then(|d| d.path())) {
                l.add_folder(dir);
            }
        });
    }

    /// Adds the folder `dir` in the background (`library::import_folder`), the line on top counting the files; then shows the
    /// books again and, when something was not added, says so. A folder added meanwhile waits for this one.
    fn add_folder(self: &Rc<Self>, dir: PathBuf) {
        let Some(folder) = self.folder.borrow().clone() else { return };
        if self.adding.replace(true) {
            self.waiting.borrow_mut().push(dir);
            return;
        }
        self.progress.set_text("");
        self.progress.set_visible(true);
        let counts = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        let (weak, c) = (Rc::downgrade(self), counts.clone());
        let tick = glib::timeout_add_local(Duration::from_millis(100), move || {
            let Some(l) = weak.upgrade() else { return glib::ControlFlow::Break };
            let (done, all) = (c.0.load(Ordering::Relaxed), c.1.load(Ordering::Relaxed));
            if all > 0 {
                l.progress.set_text(&fill(tr("Adding books… {n} of {m}"), &[("n", &done.to_string()), ("m", &all.to_string())]));
            }
            glib::ControlFlow::Continue
        });
        let (state, src, weak) = (self.store.borrow().path().to_owned(), dir.clone(), Rc::downgrade(self));
        glib::spawn_future_local(async move {
            let done = gio::spawn_blocking(move || {
                let mut s = Store::open(state);
                let added = library::import_folder(&src, &folder, &mut s, |done, all| {
                    counts.0.store(done, Ordering::Relaxed);
                    counts.1.store(all, Ordering::Relaxed);
                });
                (added, library::scan(&folder, &mut s))
            })
            .await;
            tick.remove();
            let Some(l) = weak.upgrade() else { return };
            l.store.borrow_mut().reload();
            l.adding.set(false);
            l.progress.set_visible(false);
            if let Ok((added, books)) = done {
                *l.books.borrow_mut() = books.iter().map(Book::of).collect();
                l.selected.set(0);
                l.load_covers();
                let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if let Some(message) = added.message(&name) {
                    gtk::AlertDialog::builder()
                        .message(message)
                        .buttons([tr("Close")])
                        .cancel_button(0)
                        .modal(true)
                        .build()
                        .show(Some(&l.window));
                }
            }
            l.rebuild();
            let next = (!l.waiting.borrow().is_empty()).then(|| l.waiting.borrow_mut().remove(0));
            if let Some(next) = next {
                l.add_folder(next);
            }
        });
    }

    /// Android's question before removing a book, as a sheet at the window's bottom.
    fn ask(self: &Rc<Self>, book: usize) {
        if self.question.borrow().is_some() {
            return;
        }
        let Some(b) = self.books.borrow().get(book).cloned() else { return };
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("backdrop");
        let sheet = gtk::Box::new(gtk::Orientation::Vertical, 10);
        sheet.add_css_class("question");
        sheet.set_valign(gtk::Align::End);
        sheet.set_halign(gtk::Align::Center);
        sheet.set_vexpand(true);
        sheet.set_size_request(self.window.width().min(560), -1);
        let title = label(&fill(tr("Remove “{title}” from the library?"), &[("title", &b.title)]), "question-title");
        title.set_max_width_chars(40);
        sheet.append(&title);
        sheet.append(&label(tr("The book file goes to the Trash."), "question-body"));
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_homogeneous(true);
        buttons.set_margin_top(8);
        let cancel = gtk::Button::builder().label(tr("Cancel")).focusable(false).hexpand(true).build();
        let remove = gtk::Button::builder().label(tr("Remove")).focusable(false).hexpand(true).build();
        cancel.add_css_class("sheet-button");
        remove.add_css_class("sheet-button");
        remove.add_css_class("filled");
        buttons.append(&cancel);
        buttons.append(&remove);
        sheet.append(&buttons);
        backdrop.append(&sheet);
        let weak = Rc::downgrade(self);
        cancel.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.answer(false);
            }
        });
        let weak = Rc::downgrade(self);
        remove.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.answer(true);
            }
        });
        // A click beside the sheet keeps the book.
        let click = gtk::GestureClick::new();
        let (weak, s) = (Rc::downgrade(self), sheet.clone());
        click.connect_released(move |_, _, x, y| {
            let inside =
                s.compute_bounds(&s.parent().unwrap()).is_some_and(|b| b.contains_point(&graphene::Point::new(x as f32, y as f32)));
            if !inside && let Some(l) = weak.upgrade() {
                l.answer(false);
            }
        });
        backdrop.add_controller(click);
        self.overlay.add_overlay(&backdrop);
        *self.question.borrow_mut() = Some((backdrop.upcast(), book));
    }

    /// Closes the question; removes the book when `remove`.
    fn answer(self: &Rc<Self>, remove: bool) {
        let Some((sheet, book)) = self.question.borrow_mut().take() else { return };
        self.overlay.remove_overlay(&sheet);
        if !remove {
            return;
        }
        let b = self.books.borrow()[book].clone();
        // The file goes to the Trash first; the book stays when it cannot. A file outside the folder (one the move
        // could not copy) is left alone.
        if self.folder.borrow().as_ref().is_some_and(|f| b.path.starts_with(f))
            && b.path.exists()
            && let Err(e) = gio::File::for_path(&b.path).trash(gio::Cancellable::NONE)
        {
            eprintln!("cannot move {} to the Trash: {e}", b.path.display());
            let message = fill(tr("Couldn't move “{title}” to the Trash"), &[("title", &b.title)]);
            gtk::AlertDialog::builder()
                .message(message)
                .detail(e.message())
                .buttons([tr("Close")])
                .cancel_button(0)
                .modal(true)
                .build()
                .show(Some(&self.window));
            return;
        }
        let id = b.id;
        if let Err(e) = self.store.borrow_mut().forget(&id) {
            eprintln!("cannot save the reading state: {e}");
            return;
        }
        let _ = std::fs::remove_file(thumbnail(&id));
        self.books.borrow_mut().remove(book);
        self.thumbs.borrow_mut().remove(&id);
        self.rebuild();
    }

    /// The search bar under the header (`/`, `Ctrl+F`, the search button) and the books found.
    fn set_searching(self: &Rc<Self>, on: bool) {
        if on {
            self.show_section(0);
        }
        self.search_bar.set_search_mode(on);
        if self.searching.replace(on) == on {
            return;
        }
        self.entry.set_text("");
        self.selected.set(0);
        self.rebuild();
        if on {
            self.entry.grab_focus();
        } else {
            GtkWindowExt::set_focus(&self.window, None::<&gtk::Widget>);
        }
    }

    // region sections (`library-sections`)

    /// Shows "Books" (0) or "Reading stats" (1); each keeps its state, the stats show the current month the first time.
    pub fn show_section(self: &Rc<Self>, section: usize) {
        if section == 1 && self.searching.get() {
            self.set_searching(false);
        }
        self.section.set(section);
        if section == 1 && !self.stats_shown.replace(true) {
            self.stats.reset();
        }
        for (books, stats) in &self.switchers {
            if books.is_active() != (section == 0) {
                books.set_active(section == 0);
            }
            if stats.is_active() != (section == 1) {
                stats.set_active(section == 1);
            }
        }
        let name = if section == 0 { tr("Books") } else { tr("Reading stats") };
        self.title.1.set_text(name);
        self.window.set_title(Some(name));
        self.sections.set_visible_child_name(if section == 0 { "books" } else { "stats" });
        self.add_button.set_visible(section == 0);
        self.search_toggle.set_visible(section == 0);
        self.save_button.set_visible(section == 1);
        if section == 1 {
            self.stats.focus();
        } else {
            GtkWindowExt::set_focus(&self.window, None::<&gtk::Widget>);
        }
    }

    /// Under 550 logical pixels the switcher leaves the header for the bottom pill, and the header shows the section.
    fn set_narrow(self: &Rc<Self>, narrow: bool) {
        if self.narrow.replace(narrow) == narrow {
            return;
        }
        self.title.0.set_visible_child_name(if narrow { "title" } else { "switcher" });
        self.pill.set_visible(narrow);
        // The last row can rise above the pill.
        let room = if narrow { 72 } else { 0 };
        self.page.set_margin_bottom(room);
        self.stats.set_bottom(room);
    }

    /// The theme chosen in the settings: saved as the reading theme; the library, the stats and every book window
    /// repainted (a redraw, places kept).
    fn apply_theme(self: &Rc<Self>, theme: usize) {
        let mut settings = Settings::load(&self.store.borrow());
        settings.theme = Some(theme);
        if let Err(e) = settings.save(&mut self.store.borrow_mut()) {
            eprintln!("cannot save the reading state: {e}");
        }
        let palette = THEMES[theme];
        self.palette.set(palette);
        set_theme(palette);
        crate::panels::set_theme(theme);
        self.stats.set_palette(palette);
        crate::theme_changed(theme);
        self.rebuild();
    }

    /// "About Forzac": the icon, the version, the licence, the source and the support line, as a sheet.
    fn show_about_app(self: &Rc<Self>) {
        if self.about_app.borrow().is_some() {
            return;
        }
        let backdrop = gtk::Box::new(gtk::Orientation::Vertical, 0);
        backdrop.add_css_class("backdrop");
        let sheet = gtk::Box::new(gtk::Orientation::Vertical, 8);
        sheet.add_css_class("about");
        sheet.set_valign(gtk::Align::Center);
        sheet.set_halign(gtk::Align::Center);
        sheet.set_vexpand(true);
        sheet.set_size_request((self.window.width() - 48).clamp(280, 420), -1);
        let icon = brand_icon();
        icon.set_pixel_size(64);
        icon.set_halign(gtk::Align::Start);
        sheet.append(&icon);
        sheet.append(&label(tr("Forzac"), "about-title"));
        sheet.append(&label(&fill(tr("Version {version}"), &[("version", env!("CARGO_PKG_VERSION"))]), "about-authors"));
        sheet.append(&label(tr("Made by Denis Fedosov-Ledovskikh"), "question-body"));
        let source = gtk::Label::new(None);
        source.set_xalign(0.0);
        source.set_markup(&format!(
            "MPL-2.0 · <a href=\"https://github.com/denisfl/forzac-desktop\">{}</a>",
            glib::markup_escape_text(tr("Source code"))
        ));
        source.add_css_class("question-body");
        sheet.append(&source);
        let help = i18n::help(i18n::Platform::Linux);
        let thanks = i18n::tr(i18n::SUPPORT_LINK);
        let support = gtk::Label::new(None);
        support.set_xalign(0.0);
        support.set_wrap(true);
        support.set_markup(
            &glib::markup_escape_text(help.support).replace(thanks, &format!("<a href=\"{}\">{thanks}</a>", i18n::SUPPORT_URL)),
        );
        support.add_css_class("question-body");
        support.set_max_width_chars(40);
        sheet.append(&support);
        for l in [&source, &support] {
            l.connect_activate_link(|label, uri| {
                let window = label.root().and_downcast::<gtk::Window>();
                gtk::UriLauncher::new(uri).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
                glib::Propagation::Stop
            });
        }
        let close = gtk::Button::builder().label(tr("Close")).focusable(false).build();
        close.add_css_class("sheet-button");
        close.set_margin_top(12);
        sheet.append(&close);
        backdrop.append(&sheet);
        let weak = Rc::downgrade(self);
        close.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.close_about_app();
            }
        });
        self.overlay.add_overlay(&backdrop);
        *self.about_app.borrow_mut() = Some(backdrop.upcast());
    }

    fn close_about_app(&self) {
        if let Some(sheet) = self.about_app.borrow_mut().take() {
            self.overlay.remove_overlay(&sheet);
        }
    }

    // endregion

    // endregion

    /// Keys, the head's buttons, the search field and dropping files.
    fn connect(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.search_bar.connect_search_mode_enabled_notify(move |bar| {
            if let Some(l) = weak.upgrade() {
                l.set_searching(bar.is_search_mode());
            }
        });
        let weak = Rc::downgrade(self);
        self.save_button.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.stats.save();
            }
        });
        for (books, stats) in &self.switchers {
            for (button, section) in [(books, 0), (stats, 1)] {
                let weak = Rc::downgrade(self);
                button.connect_toggled(move |b| {
                    if let Some(l) = weak.upgrade()
                        && b.is_active()
                        && l.section.get() != section
                    {
                        l.show_section(section);
                    }
                });
            }
        }
        // The window's width decides where the switcher is (`set_narrow`), on every layout of its surface.
        let weak = Rc::downgrade(self);
        self.window.connect_realize(move |w| {
            let Some(surface) = w.surface() else { return };
            let weak = weak.clone();
            surface.connect_layout(move |_, width, _| {
                if let Some(l) = weak.upgrade() {
                    l.set_narrow(width < 550);
                }
            });
        });
        let weak = Rc::downgrade(self);
        self.entry.connect_changed(move |_| {
            if let Some(l) = weak.upgrade() {
                l.selected.set(0);
                l.rebuild();
            }
        });

        // The book's menu (`menu`): its items carry the book's index.
        let actions = gio::SimpleActionGroup::new();
        let about = gio::SimpleAction::new("about", Some(glib::VariantTy::UINT32));
        let weak = Rc::downgrade(self);
        about.connect_activate(move |_, v| {
            if let (Some(l), Some(book)) = (weak.upgrade(), v.and_then(|v| v.get::<u32>())) {
                l.about(book as usize);
            }
        });
        let remove = gio::SimpleAction::new("remove", Some(glib::VariantTy::UINT32));
        let weak = Rc::downgrade(self);
        remove.connect_activate(move |_, v| {
            if let (Some(l), Some(book)) = (weak.upgrade(), v.and_then(|v| v.get::<u32>())) {
                l.ask(book as usize);
            }
        });
        let edit = gio::SimpleAction::new("edit", Some(glib::VariantTy::UINT32));
        let weak = Rc::downgrade(self);
        edit.connect_activate(move |_, v| {
            if let (Some(l), Some(book)) = (weak.upgrade(), v.and_then(|v| v.get::<u32>())) {
                l.edit(book as usize);
            }
        });
        actions.add_action(&edit);
        let kindle = gio::SimpleAction::new("kindle", Some(glib::VariantTy::UINT32));
        let weak = Rc::downgrade(self);
        kindle.connect_activate(move |_, v| {
            if let (Some(l), Some(book)) = (weak.upgrade(), v.and_then(|v| v.get::<u32>())) {
                l.send_to_kindle(book as usize);
            }
        });
        actions.add_action(&kindle);
        let newshelf = gio::SimpleAction::new("newshelf", Some(glib::VariantTy::UINT32));
        let weak = Rc::downgrade(self);
        newshelf.connect_activate(move |_, v| {
            if let (Some(l), Some(book)) = (weak.upgrade(), v.and_then(|v| v.get::<u32>())) {
                l.ask_shelf(book as usize);
            }
        });
        for (name, act) in [("openbook", 0), ("addfolder", 1), ("settings", 2), ("help", 3), ("aboutapp", 4), ("books", 5), ("stats", 6)] {
            let action = gio::SimpleAction::new(name, None);
            let weak = Rc::downgrade(self);
            action.connect_activate(move |_, _| {
                let Some(l) = weak.upgrade() else { return };
                match act {
                    0 => l.choose_file(),
                    1 => l.choose_folder(),
                    2 => l.show_settings(),
                    3 => l.toggle_help(),
                    4 => l.show_about_app(),
                    5 => l.show_section(0),
                    _ => l.show_section(1),
                }
            });
            actions.add_action(&action);
        }
        actions.add_action(&about);
        actions.add_action(&remove);
        actions.add_action(&newshelf);
        self.window.insert_action_group("lib", Some(&actions));

        // Keys by keyval; GTK also matches them in the first (Latin) layout group, so a Russian layout works. In the
        // capture phase, before the search field: a key the field needs (a letter, `Delete`, `Home`) goes on to it.
        // The keys card has the keys while it is open.
        let help_keys = gtk::EventControllerKey::new();
        help_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        help_keys.connect_key_pressed(move |_, key, code, state| {
            let handled = weak.upgrade().is_some_and(|l| l.help.key(key, crate::page_view::latin(key, code), state));
            if handled { glib::Propagation::Stop } else { glib::Propagation::Proceed }
        });
        self.window.add_controller(help_keys);
        let controller = gtk::ShortcutController::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        // `section_key`: a key of "Books" that "Reading stats" uses itself (its own `h` `l` `j` `k` `s`…) or ignores.
        let add_key = |trigger: &str, typing: bool, section_key: bool, f: KeyAction| {
            let weak = Rc::downgrade(self);
            let action = gtk::CallbackAction::new(move |_, _| {
                let Some(l) = weak.upgrade() else { return glib::Propagation::Proceed };
                // The field's text widget, not the entry itself, has the focus while searching.
                if typing && (l.searching.get() || l.naming.borrow().is_some() || l.editing.borrow().is_some()) {
                    return glib::Propagation::Proceed;
                }
                if section_key && l.section.get() == 1 && l.settings.borrow().is_none() && l.about_app.borrow().is_none() {
                    return glib::Propagation::Proceed;
                }
                f(&l);
                glib::Propagation::Stop
            });
            controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(trigger), Some(action)));
        };
        let add = |trigger: &str, typing: bool, f: KeyAction| add_key(trigger, typing, true, f);
        let add_global = |trigger: &str, typing: bool, f: KeyAction| add_key(trigger, typing, false, f);
        let asking = |l: &Rc<Library>| {
            l.question.borrow().is_some()
                || l.about.borrow().is_some()
                || l.naming.borrow().is_some()
                || l.editing.borrow().is_some()
                || l.settings.borrow().is_some()
                || l.about_app.borrow().is_some()
        };
        // In the settings, the arrows and `h` `l` move between the theme swatches and `Return` / `Space` chooses one.
        let in_settings = |l: &Rc<Library>, delta: i32| -> bool {
            if l.settings.borrow().is_none() {
                return false;
            }
            l.move_swatch(delta);
            true
        };
        add(
            "h|KP_Left",
            true,
            Box::new(move |l| {
                if in_settings(l, -1) {
                } else if !asking(l) {
                    l.step(-1, 0)
                }
            }),
        );
        add(
            "l|KP_Right",
            true,
            Box::new(move |l| {
                if in_settings(l, 1) {
                } else if !asking(l) {
                    l.step(1, 0)
                }
            }),
        );
        add(
            "Left",
            true,
            Box::new(move |l| {
                if in_settings(l, -1) {
                } else if !asking(l) {
                    l.step(-1, 0)
                }
            }),
        );
        add(
            "Right",
            true,
            Box::new(move |l| {
                if in_settings(l, 1) {
                } else if !asking(l) {
                    l.step(1, 0)
                }
            }),
        );
        add(
            "j",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(0, 1)
                }
            }),
        );
        add(
            "k",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(0, -1)
                }
            }),
        );
        add(
            "Down|KP_Down",
            false,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(0, 1)
                }
            }),
        );
        add(
            "Up|KP_Up",
            false,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(0, -1)
                }
            }),
        );
        add(
            "Home",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.select(0)
                }
            }),
        );
        add(
            "End",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.select(l.items.borrow().len().saturating_sub(1))
                }
            }),
        );
        add(
            "Return|KP_Enter",
            false,
            Box::new(move |l| {
                if l.settings.borrow().is_some() {
                    l.choose_swatch();
                } else if l.editing.borrow().is_some() {
                    l.close_edit(Some(true))
                } else if l.naming.borrow().is_some() {
                    l.name_shelf(true)
                } else if l.about.borrow().is_some() {
                    l.close_about(true)
                } else if asking(l) {
                    l.answer(true)
                } else {
                    l.open_selected()
                }
            }),
        );
        for (trigger, back) in [("Tab|KP_Tab", false), ("<Shift>Tab|ISO_Left_Tab|<Shift>ISO_Left_Tab", true)] {
            add(
                trigger,
                false,
                Box::new(move |l| {
                    if !asking(l) && !l.searching.get() && !l.books.borrow().is_empty() {
                        l.set_view(l.view.get().next(back));
                    }
                }),
            );
        }
        add(
            "i",
            true,
            Box::new(move |l| {
                let book = l.items.borrow().get(l.selected.get()).map(|i| i.book);
                if let Some(b) = book.filter(|_| !asking(l)) {
                    l.about(b);
                }
            }),
        );
        add(
            "Delete|KP_Delete|d",
            true,
            Box::new(move |l| {
                let book = l.items.borrow().get(l.selected.get()).map(|i| i.book);
                if let Some(b) = book.filter(|_| !asking(l)) {
                    l.ask(b);
                }
            }),
        );
        add(
            "slash",
            true,
            Box::new(move |l| {
                if !asking(l) && !l.books.borrow().is_empty() {
                    l.set_searching(true)
                }
            }),
        );
        add(
            "s",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.show_section(1)
                }
            }),
        );
        add_global("<Control>o", false, Box::new(move |l| l.choose_file()));
        add_global("<Control><Shift>o|<Control><Shift>O", false, Box::new(move |l| l.choose_folder()));
        add_global("<Control>1", false, Box::new(move |l| l.show_section(0)));
        add_global("<Control>2", false, Box::new(move |l| l.show_section(1)));
        add_global(
            "<Control>f",
            false,
            Box::new(move |l| {
                if !asking(l) && !l.books.borrow().is_empty() {
                    l.set_searching(true)
                }
            }),
        );
        add_global("<Control>comma", false, Box::new(move |l| l.show_settings()));
        add(
            "space",
            true,
            Box::new(move |l| {
                if l.settings.borrow().is_some() {
                    l.choose_swatch();
                }
            }),
        );
        add_global("<Control>question|question", true, Box::new(move |l| l.toggle_help()));
        add(
            "<Shift>f",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.show_folder()
                }
            }),
        );
        add_global(
            "Escape",
            false,
            Box::new(move |l| {
                if l.about_app.borrow().is_some() {
                    l.close_about_app();
                } else if l.settings.borrow().is_some() {
                    l.close_settings();
                } else if l.editing.borrow().is_some() {
                    l.close_edit(None);
                } else if l.naming.borrow().is_some() {
                    l.name_shelf(false);
                } else if l.about.borrow().is_some() {
                    l.close_about(false);
                } else if asking(l) {
                    l.answer(false);
                } else if l.searching.get() {
                    l.set_searching(false);
                } else if l.section.get() == 1 {
                    l.show_section(0);
                } else {
                    l.window.close();
                }
            }),
        );
        add_global(
            "q",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.window.close()
                }
            }),
        );
        add_global("<Control>w", false, Box::new(move |l| l.window.close()));
        self.window.add_controller(controller);

        // Dropping files: each opens in its window; the web's dashed frame while a file is over the window.
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.add_css_class("drop");
        frame.set_can_target(false);
        frame.set_visible(false);
        let release = gtk::Label::new(Some(tr("Release the file to open it")));
        release.set_vexpand(true);
        frame.append(&release);
        self.overlay.add_overlay(&frame);
        let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let f = frame.clone();
        target.connect_enter(move |_, _, _| {
            f.set_visible(true);
            gdk::DragAction::COPY
        });
        let f = frame.clone();
        target.connect_leave(move |_| f.set_visible(false));
        let weak = Rc::downgrade(self);
        target.connect_drop(move |_, value, _, _| {
            frame.set_visible(false);
            let (Some(l), Ok(files)) = (weak.upgrade(), value.get::<gdk::FileList>()) else { return false };
            for path in files.files().iter().filter_map(|f| f.path()) {
                if path.is_dir() { l.add_folder(path) } else { (l.open)(&path) }
            }
            true
        });
        self.overlay.add_controller(target);

        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |_| {
            if let Some(l) = weak.upgrade() {
                l.settled.borrow_mut().clear();
            }
            glib::Propagation::Proceed
        });
        // The library lives as long as its window.
        let keep = RefCell::new(Some(self.clone()));
        self.window.connect_destroy(move |_| drop(keep.take()));
    }

    /// The window as drawn (`--png`).
    pub fn save_png(&self, path: &Path) {
        let Some(child) = self.window.child() else { return };
        let (w, h) = (child.width(), child.height());
        let snapshot = gtk::Snapshot::new();
        snapshot.append_color(&rgba(self.palette.get().paper), &graphene::Rect::new(0.0, 0.0, w as f32, h as f32));
        self.window.snapshot_child(&child, &snapshot);
        let (Some(node), Some(renderer)) = (snapshot.to_node(), self.window.renderer()) else { return };
        let scale = self.window.scale_factor() as f32;
        let scaled = gsk::TransformNode::new(&node, Some(&gsk::Transform::new().scale(scale, scale)));
        let texture = renderer.render_texture(&scaled, Some(&graphene::Rect::new(0.0, 0.0, w as f32 * scale, h as f32 * scale)));
        match texture.save_to_png(path) {
            Ok(()) => println!("saved {} ({}×{})", path.display(), texture.width(), texture.height()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }

    pub fn book_count(&self) -> usize {
        self.books.borrow().len()
    }
}

/// What a key does in the library.
type KeyAction = Box<dyn Fn(&Rc<Library>)>;

fn label(text: &str, class: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.add_css_class(class);
    l.set_xalign(0.0);
    l.set_wrap(true);
    l.set_wrap_mode(pango::WrapMode::WordChar);
    l
}

/// A label of at most `lines` lines, the rest ellipsized.
fn lines(text: &str, class: &str, lines: i32) -> gtk::Label {
    let l = label(text, class);
    l.set_lines(lines);
    l.set_ellipsize(pango::EllipsizeMode::End);
    l
}

fn spacing(height: i32) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    b.set_size_request(-1, height);
    b
}

/// The web's ink pill.
fn pill(text: &str, big: bool) -> gtk::Button {
    let b = gtk::Button::builder().label(text).focusable(false).build();
    b.add_css_class("pill");
    if big {
        b.add_css_class("big");
    }
    b.set_valign(gtk::Align::Center);
    b
}

/// The app icon, 36 px: from the icon theme, or from the source tree in a development build.
fn brand_icon() -> gtk::Image {
    if let Some(display) = gdk::Display::default() {
        let theme = gtk::IconTheme::for_display(&display);
        let dev = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/icons"));
        if !theme.has_icon(crate::APP_ID) && dev.is_dir() {
            theme.add_search_path(dev);
        }
    }
    let image = gtk::Image::from_icon_name(crate::APP_ID);
    image.set_pixel_size(36);
    image
}
