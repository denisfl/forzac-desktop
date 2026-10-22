//! The reading stats window (`reading-stats`): a month or a year from the reading log of every device — "Month · Year",
//! the period with arrows, three figures, the reading calendar, the finished books — in the reader's theme, as the
//! Android app's screen; and its 1080 × 1920 picture, saved where the file dialog says (Linux has no share menu). The
//! numbers, the calendar's geometry and the texts come from the core (`stats::view`); this file only draws them, with
//! cairo and Pango. `Esc` closes the window.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use forzac_core::i18n::tr;
use forzac_core::log::{Log, day_number};
use forzac_core::settings::Settings;
use forzac_core::stats::{self, Period, View};
use gtk::prelude::*;
use gtk::{cairo, gdk, gio, glib};

use crate::SharedStore;
use crate::page_view::{Palette, THEMES};

/// The shades of the link colour for the levels 1–4 of a day (level 0 is the line colour).
const LEVEL_ALPHA: [f64; 5] = [0.0, 0.3, 0.5, 0.75, 1.0];
/// The content's widest column and its margin in the window.
const COLUMN: f64 = 560.0;
const MARGIN: f64 = 24.0;

thread_local! {
    /// The open stats window: one at a time.
    static OPEN: RefCell<Option<glib::WeakRef<gtk::Window>>> = const { RefCell::new(None) };
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn offset() -> i64 {
    glib::DateTime::now_local().map_or(0, |d| d.utc_offset().as_seconds())
}

fn set_source(cr: &cairo::Context, rgb: u32, alpha: f64) {
    let c = |shift: u32| ((rgb >> shift) & 0xFF) as f64 / 255.0;
    cr.set_source_rgba(c(16), c(8), c(0), alpha);
}

/// A rounded rectangle path.
fn rounded(cr: &cairo::Context, (x, y, w, h): Bounds, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2);
    cr.close_path();
}

/// Text in a family at a size in pixels, wrapped to `width` (none: one line), at most `lines` lines (0: any).
struct Text {
    layout: pango::Layout,
}

impl Text {
    fn new(cr: &cairo::Context, text: &str, family: &str, size: f64, bold: bool, width: Option<f64>, lines: i32) -> Text {
        let layout = pangocairo::functions::create_layout(cr);
        let mut font = pango::FontDescription::new();
        font.set_family(family);
        font.set_absolute_size(size * pango::SCALE as f64);
        if bold {
            font.set_weight(pango::Weight::Bold);
        }
        layout.set_font_description(Some(&font));
        if let Some(w) = width {
            layout.set_width((w * pango::SCALE as f64) as i32);
            layout.set_wrap(pango::WrapMode::WordChar);
        }
        if lines > 0 {
            layout.set_height(-lines);
            layout.set_ellipsize(pango::EllipsizeMode::End);
        }
        layout.set_text(text);
        Text { layout }
    }

    fn size(&self) -> (f64, f64) {
        let (w, h) = self.layout.pixel_size();
        (w as f64, h as f64)
    }

    /// Draws it with its top left at `(x, y)` in `rgb`; returns its height.
    fn draw(&self, cr: &cairo::Context, x: f64, y: f64, rgb: u32, alpha: f64) -> f64 {
        set_source(cr, rgb, alpha);
        cr.move_to(x, y);
        pangocairo::functions::show_layout(cr, &self.layout);
        self.size().1
    }
}

/// x, y, width, height.
type Bounds = (f64, f64, f64, f64);

const SERIF: &str = "Literata";
const SANS: &str = "Golos Text";

/// The calendar of `v` with its top left at `(x, y)`.
fn draw_calendar(cr: &cairo::Context, v: &View, p: Palette, (x, y): (f64, f64), numbers: f64, labels: f64, stroke: f64) {
    for l in &v.labels {
        let t = Text::new(cr, &l.text, SANS, labels, false, None, 0);
        let lx = if l.centered { l.rect.x as f64 + (l.rect.w as f64 - t.size().0) / 2.0 } else { l.rect.x as f64 };
        t.draw(cr, x + lx, y + l.rect.y as f64, p.muted, 1.0);
    }
    let radius = v.radius as f64;
    for c in &v.cells {
        let r = (x + c.rect.x as f64, y + c.rect.y as f64, c.rect.w as f64, c.rect.h as f64);
        if !c.future {
            if c.level == 0 {
                set_source(cr, p.line, 1.0);
            } else {
                set_source(cr, p.link, LEVEL_ALPHA[c.level as usize]);
            }
            rounded(cr, r, radius);
            let _ = cr.fill();
        }
        if c.today {
            set_source(cr, p.ink, 1.0);
            cr.set_line_width(stroke);
            rounded(cr, (r.0 + stroke / 2.0, r.1 + stroke / 2.0, r.2 - stroke, r.3 - stroke), radius);
            let _ = cr.stroke();
        }
        if let Some(n) = c.number {
            let t = Text::new(cr, &n.to_string(), SANS, numbers, false, None, 0);
            let (w, h) = t.size();
            let (ink, alpha) = if c.future {
                (p.muted, 0.6)
            } else if c.level >= 3 {
                (p.paper, 1.0)
            } else {
                (p.ink, 1.0)
            };
            t.draw(cr, r.0 + (r.2 - w) / 2.0, r.1 + (r.3 - h) / 2.0, ink, alpha);
        }
    }
}

/// The period's picture, 1080 × 1920, as the Android app's `StatsImage`.
fn picture(log: &Log, period: Period, p: Palette) -> Option<gdk::MemoryTexture> {
    let (w, h, edge) = (1080.0, 1920.0, 88.0);
    let width = w - 2.0 * edge;
    let v = stats::view(log, period, now(), offset(), width as f32, 2.5, 4);
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, w as i32, h as i32).ok()?;
    {
        let cr = cairo::Context::new(&surface).ok()?;
        set_source(&cr, p.paper, 1.0);
        let _ = cr.paint();
        // The period large: the month and its year under it, or the year.
        let (big, small) = period.big_and_small();
        let mut top = edge;
        if small.is_empty() {
            top += Text::new(&cr, &big, SERIF, 220.0, false, None, 0).draw(&cr, edge, top, p.ink, 1.0);
        } else {
            top += Text::new(&cr, &big, SERIF, 150.0, false, None, 0).draw(&cr, edge, top, p.ink, 1.0);
            top += 4.0 + Text::new(&cr, &small, SERIF, 56.0, false, None, 0).draw(&cr, edge, top + 4.0, p.muted, 1.0);
        }
        // "Forzac" small at the bottom.
        let brand = Text::new(&cr, tr("Forzac"), SERIF, 44.0, false, None, 0);
        let (bw, bh) = brand.size();
        let bottom = h - edge - bh;
        brand.draw(&cr, (w - bw) / 2.0, bottom, p.muted, 1.0);
        // The figures: one size for the three, the largest that fits.
        let weights = [1.4, 1.0, 1.0];
        let widths: Vec<f64> = weights.iter().map(|k| width * k / 3.4).collect();
        let mut size = 96.0;
        while size > 40.0
            && v.figures.iter().zip(&widths).any(|((value, _), cw)| Text::new(&cr, value, SERIF, size, false, None, 0).size().0 > cw - 40.0)
        {
            size -= 4.0;
        }
        let value_height = Text::new(&cr, "0", SERIF, size, false, None, 0).size().1;
        let captions: Vec<Text> =
            v.figures.iter().zip(&widths).map(|((_, c), cw)| Text::new(&cr, c, SANS, 38.0, false, Some(cw - 40.0), 0)).collect();
        let figures_height = value_height + 8.0 + captions.iter().map(|t| t.size().1).fold(0.0, f64::max);
        let gap = 96.0;
        let mut y = top + ((bottom - top) - (v.height as f64 + gap + figures_height)) / 2.0;
        draw_calendar(&cr, &v, p, (edge, y), 32.0, 28.0, 4.0);
        y += v.height as f64 + gap;
        let mut x = edge;
        for (((value, _), caption), cw) in v.figures.iter().zip(&captions).zip(&widths) {
            Text::new(&cr, value, SERIF, size, false, None, 0).draw(&cr, x, y, p.ink, 1.0);
            caption.draw(&cr, x, y + value_height + 8.0, p.muted, 1.0);
            x += cw;
        }
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.take_data().ok()?;
    let bytes = glib::Bytes::from(&data[..]);
    Some(gdk::MemoryTexture::new(w as i32, h as i32, gdk::MemoryFormat::B8g8r8a8Premultiplied, &bytes, stride))
}

/// What the window shows and where its parts are, for clicks.
struct Page {
    store: SharedStore,
    palette: Palette,
    period: Cell<Period>,
    view: RefCell<Option<View>>,
    /// The clicked day and its minutes.
    day: Cell<Option<(i64, u64)>>,
    /// Click targets by kind, laid out in `draw`: 0 share, 1 month, 2 year, 3 back, 4 forward; and the calendar's origin.
    targets: RefCell<Vec<(Bounds, u8)>>,
    calendar: Cell<(f64, f64)>,
    area: gtk::DrawingArea,
    window: gtk::Window,
}

impl Page {
    fn log(&self) -> Log {
        forzac_core::log::load(&self.store.borrow()).unwrap_or_default()
    }

    /// The period again at the area's width.
    fn reload(&self) {
        let width = (self.area.width() as f64 - 2.0 * MARGIN).clamp(200.0, COLUMN);
        let view = stats::view(&self.log(), self.period.get(), now(), offset(), width as f32, 1.0, 3);
        self.period.set(view.period);
        *self.view.borrow_mut() = Some(view);
        self.area.set_content_height(self.view.borrow().as_ref().map_or(0.0, |v| height(v, width)) as i32);
        self.area.queue_draw();
    }

    fn show(&self, period: Period) {
        self.period.set(period);
        self.day.set(None);
        self.reload();
    }

    fn step(&self, delta: i64) {
        let can = self.view.borrow().as_ref().is_some_and(|v| if delta < 0 { v.can_previous } else { v.can_next });
        if can {
            self.show(self.period.get().step(delta));
        }
    }

    fn month(&self) {
        if let Period::Year(year) = self.period.get() {
            let today = Period::month_of(day_number(now(), offset()));
            let Period::Month { year: this, month } = today else { return };
            self.show(Period::Month { year, month: if year == this { month } else { 12 } });
        }
    }

    fn year(&self) {
        if let Period::Month { year, .. } = self.period.get() {
            self.show(Period::Year(year));
        }
    }

    /// "Save Image…": the picture where the file dialog says, `~/Pictures/forzac-2026-10.png` by default.
    fn save(self: &Rc<Self>) {
        let period = self.period.get();
        let dialog = gtk::FileDialog::builder().title(tr("Save Image…")).modal(true).initial_name(period.file_name()).build();
        if let Some(dir) = glib::user_special_dir(glib::UserDirectory::Pictures) {
            dialog.set_initial_folder(Some(&gio::File::for_path(dir)));
        }
        let page = self.clone();
        dialog.save(Some(&self.window), gio::Cancellable::NONE, move |file| {
            let Some(path) = file.ok().and_then(|f| f.path()) else { return };
            match picture(&page.log(), period, page.palette).map(|t| t.save_to_png(&path)) {
                Some(Ok(())) => {}
                Some(Err(e)) => eprintln!("{}: {e}", path.display()),
                None => eprintln!("cannot draw the picture"),
            }
        });
    }

    fn draw(&self, cr: &cairo::Context, width: f64) {
        let view = self.view.borrow();
        let Some(v) = view.as_ref() else { return };
        let (targets, calendar) = draw_page(cr, v, self.palette, self.day.get(), width);
        *self.targets.borrow_mut() = targets;
        self.calendar.set(calendar);
    }

    fn click(self: &Rc<Self>, x: f64, y: f64) {
        let hit =
            self.targets.borrow().iter().find(|((tx, ty, tw, th), _)| x >= *tx && x <= tx + tw && y >= *ty && y <= ty + th).map(|t| t.1);
        match hit {
            Some(0) => return self.save(),
            Some(1) => return self.month(),
            Some(2) => return self.year(),
            Some(3) => return self.step(-1),
            Some(4) => return self.step(1),
            _ => {}
        }
        let (cx, cy) = self.calendar.get();
        let day = self.view.borrow().as_ref().and_then(|v| {
            let (x, y) = ((x - cx) as f32, (y - cy) as f32);
            v.cells
                .iter()
                .find(|c| !c.future && x >= c.rect.x && x <= c.rect.x + c.rect.w && y >= c.rect.y && y <= c.rect.y + c.rect.h)
                .map(|c| (c.day, c.minutes))
        });
        if day.is_some() {
            self.day.set(day);
            self.area.queue_draw();
        }
    }
}

/// Draws the window's content for `v` `width` wide, with the clicked `day`'s line; returns the click targets and the
/// calendar's origin.
fn draw_page(cr: &cairo::Context, v: &View, p: Palette, day: Option<(i64, u64)>, width: f64) -> (Vec<(Bounds, u8)>, (f64, f64)) {
    set_source(cr, p.paper, 1.0);
    let _ = cr.paint();
    let mut targets = Vec::new();
    let w = (width - 2.0 * MARGIN).min(COLUMN);
    let x0 = (width - w) / 2.0;

    // The title and the save button.
    let mut y = 20.0;
    Text::new(cr, tr("Reading stats"), SERIF, 26.0, true, Some(w - 48.0), 1).draw(cr, x0, y, p.ink, 1.0);
    let share = (x0 + w - 36.0, y, 36.0, 36.0);
    icon(cr, crate::icons::SHARE, (share.0 + 6.0, share.1 + 6.0, 24.0), p.ink);
    targets.push((share, 0));
    y += 36.0 + 16.0;

    // "Month · Year".
    let month = matches!(v.period, Period::Month { .. });
    let halves = [(x0, y, w / 2.0, 44.0), (x0 + w / 2.0, y, w / 2.0, 44.0)];
    cr.save().ok();
    rounded(cr, (x0, y, w, 44.0), 12.0);
    cr.clip();
    let selected = halves[if month { 0 } else { 1 }];
    set_source(cr, p.ink, 1.0);
    cr.rectangle(selected.0, selected.1, selected.2, selected.3);
    let _ = cr.fill();
    cr.restore().ok();
    set_source(cr, p.line, 1.0);
    cr.set_line_width(1.0);
    rounded(cr, (x0 + 0.5, y + 0.5, w - 1.0, 43.0), 12.0);
    let _ = cr.stroke();
    for (i, (name, r)) in [tr("Month"), tr("Year")].into_iter().zip(halves).enumerate() {
        let on = (i == 0) == month;
        let t = Text::new(cr, name, SANS, 15.0, false, None, 0);
        let (tw, th) = t.size();
        t.draw(cr, r.0 + (r.2 - tw) / 2.0, r.1 + (r.3 - th) / 2.0, if on { p.sheet } else { p.ink }, 1.0);
        targets.push((r, 1 + i as u8));
    }
    y += 44.0 + 12.0;

    // The period between its arrows.
    let (back, forward) = ((x0, y, 44.0, 44.0), (x0 + w - 44.0, y, 44.0, 44.0));
    icon(cr, crate::icons::CHEVRON_LEFT, (back.0 + 10.0, back.1 + 10.0, 24.0), if v.can_previous { p.ink } else { p.line });
    icon(cr, crate::icons::CHEVRON_RIGHT, (forward.0 + 10.0, forward.1 + 10.0, 24.0), if v.can_next { p.ink } else { p.line });
    targets.push((back, 3));
    targets.push((forward, 4));
    let title = Text::new(cr, &v.title, SERIF, 21.0, false, None, 0);
    let (tw, th) = title.size();
    title.draw(cr, x0 + (w - tw) / 2.0, y + (44.0 - th) / 2.0, p.ink, 1.0);
    y += 44.0 + 20.0;

    // The three figures.
    let mut x = x0;
    let value_height = Text::new(cr, "0", SERIF, 22.0, false, None, 0).size().1;
    for ((value, caption), weight) in v.figures.iter().zip([1.5, 1.0, 1.0]) {
        let cw = w * weight / 3.5;
        Text::new(cr, value, SERIF, 22.0, false, None, 0).draw(cr, x, y, p.ink, 1.0);
        Text::new(cr, caption, SANS, 13.0, false, Some(cw - 8.0), 0).draw(cr, x, y + value_height + 2.0, p.muted, 1.0);
        x += cw;
    }
    y += figures_height(cr, &v.figures, w) + 24.0;

    // The calendar and the clicked day.
    let calendar = (x0, y);
    draw_calendar(cr, v, p, (x0, y), 13.0, 11.0, 1.5);
    y += v.height as f64;
    if let Some((day, minutes)) = day {
        let line = Text::new(cr, &stats::day_line(day, minutes), SANS, 14.0, false, Some(w), 1);
        line.draw(cr, x0, y + (40.0 - line.size().1) / 2.0, p.muted, 1.0);
    }
    y += 40.0 + 8.0;

    // The finished books, or what will be there.
    y += Text::new(cr, tr("Finished"), SERIF, 19.0, true, None, 0).draw(cr, x0, y, p.ink, 1.0) + 6.0;
    if v.books.is_empty() {
        Text::new(cr, tr("What you finish will appear here"), SANS, 15.0, false, Some(w), 0).draw(cr, x0, y + 8.0, p.muted, 1.0);
    }
    for (title, caption) in &v.books {
        let th = Text::new(cr, title, SERIF, 17.0, false, Some(w), 2).draw(cr, x0, y + 8.0, p.ink, 1.0);
        let ch = Text::new(cr, caption, SANS, 13.0, false, Some(w), 1).draw(cr, x0, y + 8.0 + th, p.muted, 1.0);
        y += 8.0 + th + ch + 8.0;
    }
    (targets, calendar)
}

/// The height of the window's content for `v` at `width`.
fn height(v: &View, width: f64) -> f64 {
    let w = (width - 2.0 * MARGIN).min(COLUMN);
    let (calendar, books, figures) = (v.height as f64, &v.books, &v.figures);
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).expect("a surface");
    let cr = cairo::Context::new(&surface).expect("a context");
    let mut y = 20.0 + 36.0 + 16.0 + 44.0 + 12.0 + 44.0 + 20.0 + figures_height(&cr, figures, w) + 24.0 + calendar + 40.0 + 8.0;
    y += Text::new(&cr, tr("Finished"), SERIF, 19.0, true, None, 0).size().1 + 6.0;
    if books.is_empty() {
        y += 16.0 + Text::new(&cr, tr("What you finish will appear here"), SANS, 15.0, false, Some(w), 0).size().1;
    }
    for (title, caption) in books {
        y += 8.0
            + Text::new(&cr, title, SERIF, 17.0, false, Some(w), 2).size().1
            + Text::new(&cr, caption, SANS, 13.0, false, Some(w), 1).size().1
            + 8.0;
    }
    y + 24.0
}

fn figures_height(cr: &cairo::Context, figures: &[(String, String); 3], w: f64) -> f64 {
    let caption = figures
        .iter()
        .zip([1.5, 1.0, 1.0])
        .map(|((_, c), k)| Text::new(cr, c, SANS, 13.0, false, Some(w * k / 3.5 - 8.0), 0).size().1)
        .fold(0.0, f64::max);
    Text::new(cr, "0", SERIF, 22.0, false, None, 0).size().1 + 2.0 + caption
}

/// A Lucide icon's strokes at `(x, y)`, `size` square, in `rgb`.
fn icon(cr: &cairo::Context, paths: crate::icons::Paths, (x, y, size): (f64, f64, f64), rgb: u32) {
    cr.save().ok();
    cr.translate(x, y);
    cr.scale(size / 24.0, size / 24.0);
    set_source(cr, rgb, 1.0);
    cr.set_line_width(1.75);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    crate::icons::trace(cr, paths);
    let _ = cr.stroke();
    cr.restore().ok();
}

/// `--stats DIR` (measurements): the window's current month and year at 520 pixels wide and their pictures, as PNGs.
pub fn save_pngs(store: &SharedStore, dir: &std::path::Path) {
    crate::fonts::add_interface_fonts();
    let palette = THEMES[Settings::load(&store.borrow()).theme_index(crate::appearance::dark())];
    let log = forzac_core::log::load(&store.borrow()).unwrap_or_default();
    let month = Period::month_of(day_number(now(), offset()));
    let Period::Month { year, .. } = month else { return };
    for (name, period) in [("month", month), ("year", Period::Year(year))] {
        let width = 520.0;
        let v = stats::view(&log, period, now(), offset(), (width - 2.0 * MARGIN) as f32, 1.0, 3);
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height(&v, width) as i32).expect("a surface");
        draw_page(
            &cairo::Context::new(&surface).expect("a context"),
            &v,
            palette,
            v.cells.iter().find(|c| c.today).map(|c| (c.day, c.minutes)),
            width,
        );
        drop(v);
        surface.flush();
        let (w, h, stride) = (surface.width(), surface.height(), surface.stride() as usize);
        let bytes = glib::Bytes::from(&surface.take_data().expect("the data")[..]);
        let window = gdk::MemoryTexture::new(w, h, gdk::MemoryFormat::B8g8r8a8Premultiplied, &bytes, stride);
        for (path, texture) in [
            (dir.join(format!("stats-{name}.png")), Some(window)),
            (dir.join(format!("picture-{name}.png")), picture(&log, period, palette)),
        ] {
            match texture.map(|t| t.save_to_png(&path)) {
                Some(Ok(())) => println!("saved {}", path.display()),
                _ => eprintln!("{}: not saved", path.display()),
            }
        }
    }
}

/// Opens the stats window on the current month over `parent`, or brings the open one forward on it.
pub fn open(parent: &impl IsA<gtk::Window>, store: &SharedStore) {
    let month = Period::month_of(day_number(now(), offset()));
    if let Some(window) = OPEN.with(|o| o.borrow().as_ref().and_then(|w| w.upgrade())) {
        window.present();
        return;
    }
    crate::fonts::add_interface_fonts();
    let palette = THEMES[Settings::load(&store.borrow()).theme_index(crate::appearance::dark())];
    let area = gtk::DrawingArea::builder().hexpand(true).build();
    let scrolled = gtk::ScrolledWindow::builder().child(&area).hscrollbar_policy(gtk::PolicyType::Never).build();
    let window = gtk::Window::builder()
        .title(tr("Reading stats"))
        .transient_for(parent)
        .default_width(520)
        .default_height(820)
        .child(&scrolled)
        .build();
    let page = Rc::new(Page {
        store: store.clone(),
        palette,
        period: Cell::new(month),
        view: RefCell::default(),
        day: Cell::new(None),
        targets: RefCell::default(),
        calendar: Cell::new((0.0, 0.0)),
        area: area.clone(),
        window: window.clone(),
    });
    let p = page.clone();
    area.set_draw_func(move |_, cr, width, _| p.draw(cr, width as f64));
    let p = Rc::downgrade(&page);
    area.connect_resize(move |_, _, _| {
        if let Some(p) = p.upgrade() {
            p.reload();
        }
    });
    let click = gtk::GestureClick::new();
    let p = Rc::downgrade(&page);
    click.connect_released(move |_, _, x, y| {
        if let Some(p) = p.upgrade() {
            p.click(x, y);
        }
    });
    area.add_controller(click);
    let keys = gtk::EventControllerKey::new();
    let (p, s) = (Rc::downgrade(&page), scrolled.clone());
    keys.connect_key_pressed(move |_, key, code, state| {
        let Some(p) = p.upgrade() else { return glib::Propagation::Proceed };
        if state.intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK) {
            return glib::Propagation::Proceed;
        }
        let adj = s.vadjustment();
        let scroll = |dy: f64| adj.set_value((adj.value() + dy).clamp(adj.lower(), adj.upper() - adj.page_size()));
        let page_step = adj.page_size() - 40.0;
        match crate::page_view::latin(key, code) {
            gdk::Key::Left | gdk::Key::h => p.step(-1),
            gdk::Key::Right | gdk::Key::l => p.step(1),
            gdk::Key::Down | gdk::Key::j => scroll(40.0),
            gdk::Key::Up | gdk::Key::k => scroll(-40.0),
            gdk::Key::space if state.contains(gdk::ModifierType::SHIFT_MASK) => scroll(-page_step),
            gdk::Key::space | gdk::Key::Page_Down => scroll(page_step),
            gdk::Key::Page_Up => scroll(-page_step),
            gdk::Key::Home => scroll(-adj.upper()),
            gdk::Key::End => scroll(adj.upper()),
            gdk::Key::m => p.month(),
            gdk::Key::y => p.year(),
            gdk::Key::s => p.save(),
            gdk::Key::Escape => p.window.close(),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    window.add_controller(keys);
    set_background(&window, palette);
    OPEN.with(|o| *o.borrow_mut() = Some(window.downgrade()));
    window.present();
    page.reload();
}

/// The window's background in the theme's paper, so nothing flashes around the drawing.
fn set_background(window: &gtk::Window, p: Palette) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&format!(".forzac-stats {{ background: #{:06x}; }}", p.paper));
    window.add_css_class("forzac-stats");
    gtk::style_context_add_provider_for_display(&WidgetExt::display(window), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}
