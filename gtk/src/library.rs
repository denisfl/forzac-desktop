//! The library (`library`): the reading state's books as the web reader's shelf of covers, with the Android app's
//! "Continue reading" card, generated covers, search and a question before a book is removed. Books open in place;
//! cover thumbnails are cached in `$XDG_CACHE_HOME/forzac/covers`, one file per book id (empty: the book has none).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use forzac_core::i18n::{fill, tr};
use forzac_core::library::{self, CoverStyle};
use forzac_core::state::Store;
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

/// A book of the library, from the reading state.
#[derive(Clone)]
struct Book {
    id: String,
    path: PathBuf,
    title: String,
    authors: String,
    progress: f32,
}

/// A selectable book on the window: the card or a shelf item.
struct Item {
    book: usize,
    cover: Cover,
    widget: gtk::Widget,
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
}

/// The library's colours in the reader's theme; its layout (the web's `reader.css`).
fn set_theme(p: Palette) {
    let hex = |c: u32| format!("#{c:06x}");
    let (paper, ink, muted, line, sheet) = (hex(p.paper), hex(p.ink), hex(p.muted), hex(p.line), hex(p.sheet));
    PROVIDER.with(|provider| {
        provider.load_from_string(&format!(
            "window.forzac-library {{ background-color: {paper}; color: {ink}; }}
             .forzac-library .muted, .forzac-library .note, .forzac-library .shelf-author, .forzac-library .shelf-share,
                 .forzac-library .card-caption, .forzac-library .card-authors, .forzac-library .hint, .forzac-library .privacy,
                 .forzac-library .no-results, .forzac-library .question-body, .forzac-library button.remove {{ color: {muted}; }}
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
             {STATIC_CSS}"
        ));
    });
}

const STATIC_CSS: &str = "
    .forzac-library .brand-name { font-family: 'Literata'; font-weight: 700; font-size: 22px; }
    .forzac-library button.pill { border-radius: 999px; padding: 11px 20px; font-weight: 500; font-size: 15px; }
    .forzac-library button.pill:hover { opacity: 0.88; }
    .forzac-library button.pill.big { padding: 16px 30px; font-size: 17px; }
    .forzac-library button.circle { min-width: 42px; min-height: 42px; border-radius: 22px; }
    .forzac-library .note { font-size: 14px; }
    .forzac-library .shelf-title { font-weight: 500; font-size: 14px; }
    .forzac-library .shelf-author { font-size: 13px; }
    .forzac-library .shelf-share { font-size: 12px; font-feature-settings: 'tnum'; }
    .forzac-library button.remove { font-size: 18px; padding: 0 2px 4px 8px; }
    .forzac-library .card-caption { font-size: 12px; font-weight: 500; letter-spacing: 0.7px; }
    .forzac-library .card-title { font-family: 'Literata'; font-weight: 600; font-size: 21px; }
    .forzac-library .card-authors { font-size: 14px; }
    .forzac-library progressbar trough, .forzac-library progressbar progress { min-height: 2px; border-radius: 1px; border: none;
        box-shadow: none; }
    .forzac-library .rule { min-height: 1px; }
    .forzac-library entry.search-field { font-family: 'PT Serif'; font-size: 26px; background: none; border: none; box-shadow: none;
        outline: none; padding: 0; min-height: 44px; }
    .forzac-library .empty-heading { font-family: 'Literata'; font-weight: 700; font-size: 44px; letter-spacing: -0.4px;
        line-height: 1.08; }
    .forzac-library .empty-lead { font-family: 'Literata'; font-size: 19px; line-height: 1.55; }
    .forzac-library .hint { font-size: 15px; }
    .forzac-library .privacy { font-size: 14px; line-height: 1.55; padding-top: 18px; }
    .forzac-library .no-results { font-size: 15px; }
    .forzac-library .drop { border-radius: 20px; margin: 16px; }
    .forzac-library .drop label { font-size: 20px; font-weight: 500; }
    .forzac-library .backdrop { background-color: alpha(black, 0.2); }
    .forzac-library .question { border-radius: 22px 22px 0 0; padding: 24px; }
    .forzac-library .question-title { font-family: 'PT Serif'; font-weight: 700; font-size: 19px; }
    .forzac-library .question-body { font-size: 14px; }
    .forzac-library button.sheet-button { min-height: 48px; border-radius: 24px; font-size: 16px; font-weight: 500; }
";

/// The library window and what it shows.
pub struct Library {
    pub window: gtk::ApplicationWindow,
    store: Rc<RefCell<Store>>,
    /// Opens a book in its window; false when it cannot be read.
    open: Rc<dyn Fn(&Path) -> bool>,
    palette: Palette,
    books: RefCell<Vec<Book>>,
    thumbs: RefCell<HashMap<String, Thumb>>,
    marbles: RefCell<HashMap<i32, gdk::Texture>>,
    items: RefCell<Vec<Item>>,
    /// The item index of the card's book, if shown.
    card: Cell<bool>,
    shelf: RefCell<Option<Layout>>,
    selected: Cell<usize>,
    searching: Cell<bool>,
    scrolled: gtk::ScrolledWindow,
    overlay: gtk::Overlay,
    body: gtk::Box,
    brand: gtk::Box,
    entry: gtk::Entry,
    buttons: (gtk::Button, gtk::Button, gtk::Button),
    question: RefCell<Option<(gtk::Widget, usize)>>,
    /// Covers being loaded or made; `settled` runs when none is left after the first frame.
    pending: Cell<usize>,
    first_frame: Cell<Option<Instant>>,
    settled: RefCell<Vec<Box<dyn FnOnce()>>>,
}

impl Library {
    /// The library window, or the open one brought forward.
    pub fn show(
        app: &gtk::Application,
        store: &Rc<RefCell<Store>>,
        theme: usize,
        size: (i32, i32),
        open: Rc<dyn Fn(&Path) -> bool>,
    ) -> Option<Rc<Library>> {
        if let Some(window) = OPEN.with(|o| o.borrow().as_ref().and_then(|w| w.upgrade())).filter(|w| w.application().is_some()) {
            window.present();
            return None;
        }
        crate::fonts::add_interface_fonts();
        let palette = THEMES[theme];
        set_theme(palette);
        let books: Vec<Book> = store
            .borrow()
            .books()
            .iter()
            .map(|e| Book {
                id: e.id.clone(),
                path: e.path.clone(),
                title: if e.title.is_empty() {
                    e.path.file_name().unwrap_or_default().to_string_lossy().into_owned()
                } else {
                    e.title.clone()
                },
                authors: e.authors.clone(),
                progress: e.progress,
            })
            .collect();
        let window =
            gtk::ApplicationWindow::builder().application(app).title(tr("Forzac")).default_width(size.0).default_height(size.1).build();
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() {
            window.set_decorated(false);
        }
        window.add_css_class("forzac-panel");
        window.add_css_class("forzac-library");
        OPEN.with(|o| *o.borrow_mut() = Some(window.downgrade()));

        let entry = gtk::Entry::builder().placeholder_text(tr("Title or author")).hexpand(true).visible(false).build();
        entry.add_css_class("search-field");
        let circle = |paths: icons::Paths, tip: &str| {
            let b = gtk::Button::builder().child(&Icon::new(paths, 20)).tooltip_text(tip).focusable(false).build();
            b.add_css_class("circle");
            b.set_valign(gtk::Align::Center);
            b
        };
        let search = circle(icons::SEARCH, tr("Search"));
        let close = circle(icons::X, tr("Close"));
        close.set_visible(false);
        let open_button = pill(tr("Open a book"), false);
        let brand = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        brand.append(&brand_icon());
        let name = gtk::Label::new(Some(tr("Forzac")));
        name.add_css_class("brand-name");
        brand.append(&name);
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        head.set_size_request(-1, 48);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        for w in [
            brand.upcast_ref::<gtk::Widget>(),
            entry.upcast_ref(),
            spacer.upcast_ref(),
            search.upcast_ref(),
            close.upcast_ref(),
            open_button.upcast_ref(),
        ] {
            head.append(w);
        }
        let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
        body.set_vexpand(true);
        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        page.append(&head);
        page.append(&body);
        let column = Layout::new(Kind::Column);
        column.add(&page);
        let scrolled = gtk::ScrolledWindow::builder().child(&column).hscrollbar_policy(gtk::PolicyType::Never).build();
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scrolled));
        window.set_child(Some(&overlay));

        let library = Rc::new(Library {
            window,
            store: store.clone(),
            open,
            palette,
            books: RefCell::new(books),
            thumbs: RefCell::default(),
            marbles: RefCell::default(),
            items: RefCell::default(),
            card: Cell::new(false),
            shelf: RefCell::default(),
            selected: Cell::new(0),
            searching: Cell::new(false),
            scrolled,
            overlay,
            body,
            brand,
            entry,
            buttons: (search, close, open_button),
            question: RefCell::default(),
            pending: Cell::new(0),
            first_frame: Cell::new(None),
            settled: RefCell::default(),
        });
        library.load_covers();
        library.rebuild();
        library.connect();
        library.watch_first_frame();
        library.window.present();
        Some(library)
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
        let cover = Cover::new(b, thumb, self.palette, size);
        if generated {
            self.marble_for(&cover);
        }
        cover
    }

    // endregion

    // region building

    /// Fills the window for the books and the search: the empty state, or the note, the card and the shelf.
    fn rebuild(self: &Rc<Self>) {
        while let Some(child) = self.body.first_child() {
            self.body.remove(&child);
        }
        self.items.borrow_mut().clear();
        *self.shelf.borrow_mut() = None;
        let searching = self.searching.get();
        let empty = self.books.borrow().is_empty();
        let (search, close, open) = &self.buttons;
        search.set_visible(!empty && !searching);
        close.set_visible(searching);
        open.set_visible(!empty && !searching);
        self.brand.set_visible(!searching);
        if empty {
            self.body.append(&self.empty_state());
            return;
        }
        let books = self.books.borrow().clone();
        let (card, order): (Option<usize>, Vec<usize>) = if searching {
            let query = self.entry.text();
            let mut ranked: Vec<(u32, usize)> =
                books.iter().enumerate().filter_map(|(i, b)| library::rank(&query, &b.title, &b.authors).map(|r| (r, i))).collect();
            ranked.sort_by_key(|&(r, _)| r);
            (None, ranked.into_iter().map(|(_, i)| i).collect())
        } else {
            let card = library::continue_book(books.iter().map(|b| b.progress));
            (card, (0..books.len()).filter(|&i| Some(i) != card).collect())
        };
        if !searching {
            let note = label(tr(NOTE), "note");
            note.set_margin_top(12);
            note.set_margin_bottom(36);
            note.set_max_width_chars(80);
            note.set_halign(gtk::Align::Start);
            self.body.append(&note);
        } else {
            self.body.append(&spacing(24));
        }
        let mut items = Vec::new();
        if let Some(c) = card {
            let (widget, cover) = self.card_widget(c);
            self.body.append(&widget);
            items.push(Item { book: c, cover, widget });
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
        let shelf = Layout::new(Kind::Shelf);
        for &i in &order {
            let (widget, cover) = self.shelf_item(i);
            shelf.add(&widget);
            items.push(Item { book: i, cover, widget });
        }
        self.body.append(&shelf);
        *self.shelf.borrow_mut() = Some(shelf);
        *self.items.borrow_mut() = items;
        let n = self.items.borrow().len();
        self.select(self.selected.get().min(n.saturating_sub(1)));
    }

    /// The Android app's "Continue reading" card.
    fn card_widget(self: &Rc<Self>, book: usize) -> (gtk::Widget, Cover) {
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
        let remove = self.remove_button(book);
        title_row.append(&remove);
        text.append(&title_row);
        if !b.authors.is_empty() {
            let authors = lines(&b.authors, "card-authors", 2);
            text.append(&authors);
        }
        let bar = gtk::ProgressBar::new();
        bar.set_fraction(b.progress.clamp(0.0, 1.0) as f64);
        bar.set_margin_top(4);
        text.append(&bar);
        text.append(&label(&library::share_label(b.progress), "shelf-share"));
        row.append(&text);
        self.connect_item(&row, &cover, &remove, book);
        (row.upcast(), cover)
    }

    /// A book on the shelf: its cover, the title with the remove "×", the authors and the share read.
    fn shelf_item(self: &Rc<Self>, book: usize) -> (gtk::Widget, Cover) {
        let b = self.books.borrow()[book].clone();
        let cover = self.cover(book, None);
        let item = gtk::Box::new(gtk::Orientation::Vertical, 0);
        item.append(&cover);
        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        title_row.set_margin_top(12);
        let title = lines(&b.title, "shelf-title", 3);
        title.set_hexpand(true);
        title_row.append(&title);
        let remove = self.remove_button(book);
        remove.set_valign(gtk::Align::Start);
        title_row.append(&remove);
        item.append(&title_row);
        if !b.authors.is_empty() {
            let authors = lines(&b.authors, "shelf-author", 2);
            authors.set_margin_top(2);
            item.append(&authors);
        }
        let share = label(&library::share_label(b.progress), "shelf-share");
        share.set_margin_top(4);
        item.append(&share);
        self.connect_item(&item, &cover, &remove, book);
        (item.upcast(), cover)
    }

    fn remove_button(self: &Rc<Self>, book: usize) -> gtk::Button {
        let remove = gtk::Button::builder().label("×").tooltip_text(tr("Remove from library")).focusable(false).build();
        remove.add_css_class("remove");
        remove.set_opacity(0.0);
        let weak = Rc::downgrade(self);
        remove.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.ask(book);
            }
        });
        remove
    }

    /// A click opens the book; under the pointer its cover lifts and its "×" shows.
    fn connect_item(self: &Rc<Self>, item: &gtk::Box, cover: &Cover, remove: &gtk::Button, book: usize) {
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
        let (c, r) = (cover.clone(), remove.clone());
        motion.connect_enter(move |_, _, _| {
            c.set_hover(true);
            r.set_opacity(1.0);
        });
        let (c, r) = (cover.clone(), remove.clone());
        motion.connect_leave(move |_| {
            c.set_hover(false);
            r.set_opacity(0.0);
        });
        item.add_controller(motion);
        item.set_cursor_from_name(Some("pointer"));
    }

    /// The web's empty library: the invitation, the button, the hint and the note beside the sample page.
    fn empty_state(self: &Rc<Self>) -> gtk::Widget {
        let text = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let heading = label(tr("Read books right on your computer"), "empty-heading");
        heading.set_max_width_chars(16);
        text.append(&heading);
        let lead = label(tr("Open a book file — it stays where it is, the library only remembers it."), "empty-lead");
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
        let privacy = label(tr(NOTE), "privacy");
        privacy.set_max_width_chars(48);
        privacy.set_margin_top(48);
        text.append(&privacy);
        let split = Layout::new(Kind::Split);
        split.add(&text);
        split.add(&Sample::new(self.palette));
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

    /// Moves the selection: `dx` along the books, `dy` rows of the shelf (the card is a row of its own).
    fn step(&self, dx: i32, dy: i32) {
        let n = self.items.borrow().len();
        if n == 0 {
            return;
        }
        let i = self.selected.get();
        let card = usize::from(self.card.get());
        let columns = self.shelf.borrow().as_ref().map_or(1, |s| s.imp().columns.get().max(1));
        let target = if dx != 0 {
            (i as i32 + dx).clamp(0, n as i32 - 1) as usize
        } else if dy > 0 {
            if card == 1 && i == 0 {
                1.min(n - 1)
            } else if (i - card) / columns < (n - 1 - card) / columns {
                (i + columns).min(n - 1)
            } else {
                i
            }
        } else if i < card + columns {
            if card == 1 && i > 0 { 0 } else { i }
        } else {
            i - columns
        };
        self.select(target);
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

    fn choose_file(self: &Rc<Self>) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(tr("Books")));
        for suffix in ["fb2", "zip", "fb3", "epub"] {
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
        sheet.append(&label(tr("The book file stays where it is."), "question-body"));
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
        let id = self.books.borrow()[book].id.clone();
        if let Err(e) = self.store.borrow_mut().forget(&id) {
            eprintln!("cannot save the reading state: {e}");
            return;
        }
        let _ = std::fs::remove_file(thumbnail(&id));
        self.books.borrow_mut().remove(book);
        self.thumbs.borrow_mut().remove(&id);
        self.rebuild();
    }

    fn set_searching(self: &Rc<Self>, on: bool) {
        if self.searching.replace(on) == on {
            return;
        }
        self.entry.set_text("");
        self.entry.set_visible(on);
        self.selected.set(0);
        self.rebuild();
        if on {
            self.entry.grab_focus();
        } else {
            GtkWindowExt::set_focus(&self.window, None::<&gtk::Widget>);
        }
    }

    // endregion

    /// Keys, the head's buttons, the search field and dropping files.
    fn connect(self: &Rc<Self>) {
        let (search, close, open) = &self.buttons;
        let weak = Rc::downgrade(self);
        search.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.set_searching(true);
            }
        });
        let weak = Rc::downgrade(self);
        close.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.set_searching(false);
            }
        });
        let weak = Rc::downgrade(self);
        open.connect_clicked(move |_| {
            if let Some(l) = weak.upgrade() {
                l.choose_file();
            }
        });
        let weak = Rc::downgrade(self);
        self.entry.connect_changed(move |_| {
            if let Some(l) = weak.upgrade() {
                l.selected.set(0);
                l.rebuild();
            }
        });

        // Keys by keyval; GTK also matches them in the first (Latin) layout group, so a Russian layout works. In the
        // capture phase, before the search field: a key the field needs (a letter, `Delete`, `Home`) goes on to it.
        let controller = gtk::ShortcutController::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let add = |trigger: &str, typing: bool, f: KeyAction| {
            let weak = Rc::downgrade(self);
            let action = gtk::CallbackAction::new(move |_, _| {
                let Some(l) = weak.upgrade() else { return glib::Propagation::Proceed };
                if typing && l.entry.has_focus() {
                    return glib::Propagation::Proceed;
                }
                f(&l);
                glib::Propagation::Stop
            });
            controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(trigger), Some(action)));
        };
        let asking = |l: &Rc<Library>| l.question.borrow().is_some();
        add(
            "h|KP_Left",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(-1, 0)
                }
            }),
        );
        add(
            "l|KP_Right",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(1, 0)
                }
            }),
        );
        add(
            "Left",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.step(-1, 0)
                }
            }),
        );
        add(
            "Right",
            true,
            Box::new(move |l| {
                if !asking(l) {
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
        add("Return|KP_Enter", false, Box::new(move |l| if asking(l) { l.answer(true) } else { l.open_selected() }));
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
        add("<Control>o", false, Box::new(move |l| l.choose_file()));
        add(
            "Escape",
            false,
            Box::new(move |l| {
                if asking(l) {
                    l.answer(false);
                } else if l.searching.get() {
                    l.set_searching(false);
                } else {
                    l.window.close();
                }
            }),
        );
        add(
            "q",
            true,
            Box::new(move |l| {
                if !asking(l) {
                    l.window.close()
                }
            }),
        );
        add("<Control>w", false, Box::new(move |l| l.window.close()));
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
                (l.open)(&path);
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
        snapshot.append_color(&rgba(self.palette.paper), &graphene::Rect::new(0.0, 0.0, w as f32, h as f32));
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

/// The web's note line.
const NOTE: &str =
    "We don’t store or send any data to a server. Everything runs on your computer. FB2, FB2.ZIP, FB3 and EPUB are supported.";

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
