//! Panels over the page: the contents (`o`), the settings (`s`) and a note. They use the reader's theme and take the
//! focus while open; closing one gives the focus back to the page.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use forzac_core::Locator;
use forzac_core::search::Hit;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, graphene, gsk, prelude::*};

use crate::icons::{self, Icon};
use crate::page_view::{PageView, Palette, THEMES};

thread_local! {
    static THEME: Theme = Theme::new();
}

/// Sets the panels' colours to `theme` (an index in `THEMES`).
pub fn set_theme(theme: usize) {
    THEME.with(|t| t.set(theme));
}

/// The panels' colours follow the reader's theme.
struct Theme(gtk::CssProvider);

impl Theme {
    fn new() -> Self {
        let provider = gtk::CssProvider::new();
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        Theme(provider)
    }

    fn set(&self, theme: usize) {
        let hex = |c: u32| format!("#{c:06x}");
        let Palette { paper, ink, muted, line, sheet, .. } = THEMES[theme];
        let (paper, ink, muted, line, sheet) = (hex(paper), hex(ink), hex(muted), hex(line), hex(sheet));
        // The theme circles keep their own theme's colours. GTK draws a later shadow over an earlier one: the gap in the
        // panel's colour lies over the ring, as on Android (a 4 px gap, a 2 px ring).
        let circles: String = THEMES
            .iter()
            .enumerate()
            .map(|(i, t)| {
                format!(
                    ".forzac-panel .circle.theme-{i} {{ background-color: {}; color: {}; box-shadow: inset 0 0 0 1px {}; }}
                     .forzac-panel .circle.theme-{i}.selected {{ box-shadow: inset 0 0 0 1px {}, 0 0 0 6px {ink}, 0 0 0 4px {sheet}; }}\n",
                    hex(t.paper),
                    hex(t.ink),
                    hex(t.line),
                    hex(t.line),
                )
            })
            .collect();
        self.0.load_from_string(&format!(
            "{STATIC_CSS}
             .forzac-panel {{ color: {ink}; }}
             .forzac-panel .muted {{ color: {muted}; }}
             .forzac-sheet {{ background-color: {sheet}; box-shadow: 0 -1px 0 {line}, 0 -12px 32px alpha(black, 0.08); }}
             .forzac-side {{ background-color: {paper}; box-shadow: -1px 0 0 {line}, -12px 0 32px alpha(black, 0.08); }}
             .forzac-panel .grabber, .forzac-panel .rule, .forzac-panel .divider {{ background-color: {line}; }}
             .forzac-panel .segmented {{ border: 1px solid {line}; }}
             .forzac-panel .chip {{ border: 1px solid {line}; }}
             .forzac-panel button:hover {{ background-color: alpha({ink}, 0.06); }}
             .forzac-panel .segment.selected, .forzac-panel .chip.selected {{ background-color: {ink}; border-color: {ink}; color: {sheet}; }}
             .forzac-panel .keys {{ outline: 2px solid alpha({ink}, 0.55); }}
             .forzac-panel .dot {{ background-color: {ink}; }}
             .forzac-panel .snippet {{ color: alpha({ink}, 0.78); }}
             .forzac-panel .search {{ background-color: {sheet}; border: 1px solid {line}; }}
             .forzac-panel .search text placeholder {{ color: {muted}; }}
             .forzac-panel row:selected {{ background-color: alpha({ink}, 0.07); color: {ink}; }}
             {circles}"
        ));
    }
}

/// The panels' layout, the same in every theme (sizes from the Android design, in logical pixels).
const STATIC_CSS: &str = "
    .forzac-panel { font-family: 'Golos Text'; font-size: 14px; }
    .forzac-panel button { background: none; border: none; box-shadow: none; outline: none; min-height: 0; min-width: 0;
        padding: 0; border-radius: 0; color: inherit; font-weight: normal; }
    .forzac-panel button:disabled { opacity: 0.35; }
    .forzac-sheet { border-radius: 22px 22px 0 0; padding: 10px 20px 24px 20px; }
    .forzac-sheet.note { padding: 10px 24px 36px 24px; }
    .forzac-panel .grabber { min-width: 36px; min-height: 4px; border-radius: 2px; }
    .forzac-panel .rule { min-height: 1px; }
    .forzac-panel .divider { min-width: 1px; }
    .forzac-panel .theme-name { font-size: 11px; }
    .forzac-panel .circle { min-width: 44px; min-height: 44px; border-radius: 22px; font-family: 'PT Serif'; font-size: 16px; }
    .forzac-panel .segmented { border-radius: 12px; min-height: 44px; }
    .forzac-panel .segmented button { min-height: 44px; }
    .forzac-panel .size-smaller { font-family: 'PT Serif'; font-size: 14px; }
    .forzac-panel .size-larger { font-family: 'PT Serif'; font-size: 22px; }
    .forzac-panel .size { font-size: 14px; font-feature-settings: 'tnum'; }
    .forzac-panel .chip { min-height: 40px; border-radius: 20px; padding: 0 16px; font-size: 15px; }
    .forzac-panel .font-pt-serif { font-family: 'PT Serif'; }
    .forzac-panel .font-literata { font-family: 'Literata'; }
    .forzac-panel .font-source-serif { font-family: 'Source Serif 4'; }
    .forzac-panel .font-pt-sans { font-family: 'PT Sans'; }
    .forzac-panel .caption { font-size: 12px; }
    .forzac-panel .keys { outline-offset: 3px; border-radius: 12px; }
    .forzac-panel .note-label { font-size: 12px; font-weight: 500; letter-spacing: 0.72px; }
    .forzac-panel .panel-title { font-size: 17px; font-weight: 600; }
    .forzac-panel .book-title { font-family: 'PT Serif'; font-size: 18px; font-weight: 600; }
    .forzac-panel .book-meta, .forzac-panel .position { font-size: 13px; font-feature-settings: 'tnum'; }
    .forzac-panel .entry { font-family: 'PT Serif'; font-size: 16px; }
    .forzac-panel .snippet { font-family: 'PT Serif'; font-size: 16px; line-height: 1.44; }
    .forzac-panel .entry.current { font-weight: 700; }
    .forzac-panel .dot { min-width: 6px; min-height: 6px; border-radius: 3px; }
    .forzac-panel .search { min-height: 44px; border-radius: 22px; padding: 0 14px; font-size: 15px; }
    .forzac-panel .plain-cover { background-color: #2f3e46; color: #f3efe6; border-radius: 3px; padding: 5px;
        font-family: 'PT Serif'; font-size: 8px; font-weight: 700; }
    .forzac-panel listview, .forzac-panel row, .forzac-panel scrolledwindow { background: none; color: inherit; padding: 0; }
    .forzac-panel row:focus-visible { outline: none; }
";

/// Lower case and "ё" as "е", for filtering the contents.
fn fold(s: &str) -> String {
    s.to_lowercase().replace('ё', "е")
}

pub(crate) fn shortcut(controller: &gtk::ShortcutController, trigger: &str, f: impl Fn() + 'static) {
    let action = gtk::CallbackAction::new(move |_, _| {
        f();
        glib::Propagation::Stop
    });
    controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(trigger), Some(action)));
}

/// The widest bottom sheet (settings, notes), as on Android tablets.
pub(crate) const SHEET_WIDTH: i32 = 560;

/// The 36×4 handle at the top of a bottom sheet.
pub(crate) fn grabber() -> gtk::Box {
    let grabber = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    grabber.add_css_class("grabber");
    grabber.set_halign(gtk::Align::Center);
    grabber
}

/// The contents panel of a reader window, laid out as the Android contents (`TocScreen.kt`, the design's
/// `Toc.dc.html`): a title with a close button, a filter field, the book, then the entries with their position in the
/// book. Built once, hidden (a list view: only the rows on screen exist); `open` shows it with the current entry
/// selected. `Enter` jumps (remembered for `Ctrl+O`), `/` filters, `Esc` or `o` closes.
pub struct Contents {
    panel: gtk::Box,
    list: gtk::ListView,
    search: gtk::Text,
    selection: gtk::SingleSelection,
    model: gtk::StringList,
    places: Rc<Vec<Locator>>,
    /// The entry of the current place: bold and marked.
    current: Rc<Cell<usize>>,
    /// "author · N% read".
    meta: gtk::Label,
}

impl Contents {
    pub fn new(overlay: &gtk::Overlay, view: &PageView) -> Rc<Self> {
        crate::fonts::add_interface_fonts();
        let entries = view.contents();
        let book = view.book();
        let places: Rc<Vec<Locator>> = Rc::new(entries.iter().map(|e| e.2).collect());
        let positions = forzac_core::state::progress_of(&book, &places);
        let titles: Vec<String> = entries.iter().map(|e| fold(&e.0)).collect();
        let current = Rc::new(Cell::new(usize::MAX));

        // Items are entry indices; the factory looks the entry up.
        let indices: Vec<String> = (0..entries.len()).map(|i| i.to_string()).collect();
        let model = gtk::StringList::new(&indices.iter().map(String::as_str).collect::<Vec<_>>());
        let query = Rc::new(RefCell::new(String::new()));
        let filter = {
            let query = query.clone();
            gtk::CustomFilter::new(move |item| {
                let q = query.borrow();
                q.is_empty() || index_of(item).and_then(|i| titles.get(i)).is_some_and(|t| t.contains(q.as_str()))
            })
        };
        let filtered = gtk::FilterListModel::new(Some(model.clone()), Some(filter.clone()));
        let selection = gtk::SingleSelection::new(Some(filtered));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            row.set_height_request(32);
            row.set_margin_top(8);
            row.set_margin_bottom(8);
            row.set_margin_end(20);
            let t = gtk::Label::new(None);
            t.add_css_class("entry");
            t.set_xalign(0.0);
            t.set_hexpand(true);
            t.set_wrap(true);
            t.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            t.set_lines(2);
            t.set_ellipsize(gtk::pango::EllipsizeMode::End);
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("dot");
            dot.set_valign(gtk::Align::Center);
            dot.set_margin_start(12);
            let p = gtk::Label::new(None);
            p.add_css_class("position");
            p.add_css_class("muted");
            p.set_width_request(44);
            p.set_xalign(1.0);
            row.append(&t);
            row.append(&dot);
            row.append(&p);
            item.downcast_ref::<gtk::ListItem>().unwrap().set_child(Some(&row));
        });
        let c = current.clone();
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let (Some(i), Some(row)) = (item.item().as_ref().and_then(index_of), item.child().and_downcast::<gtk::Box>()) else { return };
            let (title, level, _) = &entries[i];
            row.set_margin_start(20 + 16 * (*level as i32 - 1).clamp(0, 4));
            let t = row.first_child().and_downcast::<gtk::Label>().unwrap();
            t.set_text(title);
            let dot = t.next_sibling().unwrap();
            let is_current = i == c.get();
            if is_current {
                t.add_css_class("current")
            } else {
                t.remove_css_class("current")
            }
            dot.set_visible(is_current);
            let p = dot.next_sibling().and_downcast::<gtk::Label>().unwrap();
            p.set_text(&format!("{}%", (positions[i] * 100.0).round() as u32));
        });
        let list = gtk::ListView::new(Some(selection.clone()), Some(factory));

        let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panel.add_css_class("forzac-panel");
        panel.add_css_class("forzac-side");
        panel.set_halign(gtk::Align::End);
        panel.set_visible(false);

        // Title and close.
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        header.set_margin_start(20);
        header.set_margin_end(8);
        header.set_margin_top(12);
        let title = gtk::Label::new(Some("Contents"));
        title.add_css_class("panel-title");
        title.set_xalign(0.0);
        title.set_hexpand(true);
        header.append(&title);
        let close = gtk::Button::new();
        close.set_child(Some(&Icon::new(icons::X, 22)));
        close.set_size_request(44, 44);
        close.set_focusable(false);
        header.append(&close);
        panel.append(&header);

        // The filter field.
        let field = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        field.add_css_class("search");
        field.set_margin_start(20);
        field.set_margin_end(20);
        field.set_margin_top(8);
        let icon = Icon::new(icons::SEARCH, 18);
        icon.add_css_class("muted");
        field.append(&icon);
        let search = gtk::Text::new();
        search.set_placeholder_text(Some("Filter contents"));
        search.set_hexpand(true);
        field.append(&search);
        panel.append(&field);

        // The book.
        let card = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        card.set_margin_start(20);
        card.set_margin_end(20);
        card.set_margin_top(12);
        card.set_margin_bottom(22);
        card.append(&cover(&book));
        let about = gtk::Box::new(gtk::Orientation::Vertical, 3);
        about.set_valign(gtk::Align::Center);
        let name = gtk::Label::new(Some(&book.title));
        name.add_css_class("book-title");
        name.set_xalign(0.0);
        name.set_wrap(true);
        name.set_lines(2);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        about.append(&name);
        let meta = gtk::Label::new(None);
        meta.add_css_class("book-meta");
        meta.add_css_class("muted");
        meta.set_xalign(0.0);
        about.append(&meta);
        card.append(&about);
        panel.append(&card);
        let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rule.add_css_class("rule");
        panel.append(&rule);

        let scrolled = gtk::ScrolledWindow::builder().child(&list).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
        scrolled.set_margin_top(6);
        panel.append(&scrolled);
        overlay.add_overlay(&panel);
        // Shown once, invisibly: styles, the interface font and the rows are ready before the reader presses `o`.
        panel.set_width_request(panel_width(overlay.width()));
        panel.set_opacity(0.0);
        panel.set_can_target(false);
        panel.set_visible(true);
        let p = panel.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
            if p.opacity() == 0.0 {
                p.set_visible(false);
            }
            p.set_opacity(1.0);
            p.set_can_target(true);
        });

        let contents = Rc::new(Contents {
            panel,
            list: list.clone(),
            search: search.clone(),
            selection: selection.clone(),
            model,
            places,
            current,
            meta,
        });
        let weak = Rc::downgrade(&contents);
        let view = view.downgrade();
        let with = move |f: &dyn Fn(&Contents, &PageView)| {
            if let (Some(c), Some(v)) = (weak.upgrade(), view.upgrade()) {
                f(&c, &v);
            }
        };

        let w = with.clone();
        close.connect_clicked(move |_| w(&|c, v| c.close(v)));
        let w = with.clone();
        search.connect_changed(move |entry| {
            *query.borrow_mut() = fold(&entry.text());
            filter.changed(gtk::FilterChange::Different);
            w(&|c, _| c.select(0));
        });
        let w = with.clone();
        list.connect_activate(move |_, _| w(&|c, v| c.jump(v)));
        let w = with.clone();
        search.connect_activate(move |_| w(&|c, v| c.jump(v)));

        // Keys on the list: j/k move, `/` filters, Esc and `o` close.
        let keys = gtk::ShortcutController::new();
        let w = with.clone();
        shortcut(&keys, "j", move || w(&|c, _| c.step(1)));
        let w = with.clone();
        shortcut(&keys, "k", move || w(&|c, _| c.step(-1)));
        let s = search.clone();
        shortcut(&keys, "slash", move || {
            s.grab_focus();
        });
        let w = with.clone();
        shortcut(&keys, "Escape|o|q", move || w(&|c, v| c.close(v)));
        list.add_controller(keys);

        // Esc in the filter clears it, then goes back to the list; ↓ goes to the list.
        let entry_keys = gtk::ShortcutController::new();
        let w = with.clone();
        shortcut(&entry_keys, "Escape", move || {
            w(&|c, _| {
                if c.search.text().is_empty() {
                    c.select(c.selection.selected());
                } else {
                    c.search.set_text("");
                }
            })
        });
        shortcut(&entry_keys, "Down", move || with(&|c, _| c.select(c.selection.selected())));
        search.add_controller(entry_keys);
        contents
    }

    /// Shows the panel over `view`'s window with the entry of the current place selected.
    pub fn open(&self, view: &PageView) {
        let here = view.locator();
        let book = view.book();
        let read = (forzac_core::state::progress(&book, here) * 100.0).round() as u32;
        let meta: Vec<String> = book.authors.first().cloned().into_iter().chain([format!("{read}% read")]).collect();
        self.meta.set_text(&meta.join(" · "));
        self.search.set_text("");
        let current = self.places.iter().rposition(|p| *p <= here).unwrap_or(0);
        if current != self.current.replace(current) {
            // Rows bound for the old current entry are bound again.
            let n = self.model.n_items();
            self.model.items_changed(0, n, n);
        }
        self.panel.set_width_request(panel_width(view.parent().map_or(0, |p| p.width())));
        self.panel.set_opacity(1.0);
        self.panel.set_can_target(true);
        self.panel.set_visible(true);
        self.select(current as u32);
        self.list.grab_focus();
    }

    fn close(&self, view: &PageView) {
        self.panel.set_visible(false);
        view.grab_focus();
    }

    fn select(&self, position: u32) {
        if position < self.selection.n_items() {
            self.list.scroll_to(position, gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT, None);
        }
    }

    fn step(&self, delta: i64) {
        let n = self.selection.n_items() as i64;
        if n > 0 {
            self.select((self.selection.selected() as i64 + delta).clamp(0, n - 1) as u32);
        }
    }

    fn jump(&self, view: &PageView) {
        let Some(i) = self.selection.selected_item().as_ref().and_then(index_of) else { return };
        self.close(view);
        view.go_to(self.places[i]);
    }
}

/// The book's cover, 44×66: its own picture, else a plain one with the title.
fn cover(book: &forzac_core::Book) -> gtk::Widget {
    const W: f32 = 44.0;
    const H: f32 = 66.0;
    let picture = book
        .cover
        .as_ref()
        .and_then(|id| book.images.get(id))
        .and_then(|i| gdk::Texture::from_bytes(&glib::Bytes::from(&i.bytes[..])).ok());
    let widget: gtk::Widget = match picture {
        Some(texture) => {
            // Filling the cover's box, cropped, with rounded corners.
            let (w, h) = (texture.width() as f32, texture.height() as f32);
            let scale = (W / w).max(H / h);
            let rect = graphene::Rect::new((W - w * scale) / 2.0, (H - h * scale) / 2.0, w * scale, h * scale);
            let image = gsk::TextureScaleNode::new(&texture, &rect, gsk::ScalingFilter::Trilinear);
            let clip = gsk::RoundedRect::from_rect(graphene::Rect::new(0.0, 0.0, W, H), 3.0);
            NodeView::new(gsk::RoundedClipNode::new(&image, &clip).upcast(), W, H).upcast()
        }
        None => {
            let plain = gtk::Box::new(gtk::Orientation::Vertical, 0);
            plain.add_css_class("plain-cover");
            let title = gtk::Label::new(Some(&book.title));
            title.set_wrap(true);
            title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            title.set_xalign(0.0);
            title.set_vexpand(true);
            title.set_valign(gtk::Align::End);
            title.set_max_width_chars(1);
            plain.append(&title);
            plain.upcast()
        }
    };
    widget.set_size_request(W as i32, H as i32);
    widget.set_vexpand(false); // the title inside expands; the card must not
    widget.set_overflow(gtk::Overflow::Hidden);
    widget.set_valign(gtk::Align::Center);
    widget
}

/// The contents panel's width in a window `width` wide.
fn panel_width(width: i32) -> i32 {
    ((width as f32 * 0.45) as i32).clamp(280, 460)
}

/// The entry index an item of the contents model stands for.
fn index_of(item: &glib::Object) -> Option<usize> {
    item.downcast_ref::<gtk::StringObject>()?.string().parse().ok()
}

/// The search panel, laid out like the contents: the field, the count, then the matches (a snippet with the match in
/// bold, the section and the position). The book is searched as the reader types, on a worker thread. `Enter` or a
/// click goes to a match (remembered for `Ctrl+O`), `↓` moves into the list, `j`/`k` move in it, `Esc` clears the field
/// or closes the panel.
pub struct SearchPanel {
    panel: gtk::Box,
    field: gtk::Text,
    list: gtk::ListView,
    selection: gtk::SingleSelection,
    model: gtk::StringList,
    count: gtk::Label,
    hits: RefCell<Rc<Vec<Hit>>>,
    /// The query of the search running or shown, so a slower older search does not overwrite a newer one.
    query: RefCell<String>,
}

impl SearchPanel {
    pub fn new(overlay: &gtk::Overlay, view: &PageView) -> Rc<Self> {
        crate::fonts::add_interface_fonts();
        let model = gtk::StringList::new(&[]);
        let selection = gtk::SingleSelection::new(Some(model.clone()));
        let hits: RefCell<Rc<Vec<Hit>>> = RefCell::default();

        let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panel.add_css_class("forzac-panel");
        panel.add_css_class("forzac-side");
        panel.set_halign(gtk::Align::End);
        panel.set_visible(false);

        // The field, with the search icon and a clear button.
        let field_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        field_row.add_css_class("search");
        field_row.set_margin_start(20);
        field_row.set_margin_end(20);
        field_row.set_margin_top(16);
        let icon = Icon::new(icons::SEARCH, 18);
        icon.add_css_class("muted");
        field_row.append(&icon);
        let field = gtk::Text::new();
        field.set_placeholder_text(Some("Search in book"));
        field.set_hexpand(true);
        field_row.append(&field);
        let clear = gtk::Button::new();
        clear.set_child(Some(&Icon::new(icons::X, 18)));
        clear.set_focusable(false);
        field_row.append(&clear);
        panel.append(&field_row);
        let count = gtk::Label::new(None);
        count.add_css_class("book-meta");
        count.add_css_class("muted");
        count.set_xalign(0.0);
        count.set_margin_start(20);
        count.set_margin_top(14);
        count.set_margin_bottom(6);
        panel.append(&count);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let row = gtk::Box::new(gtk::Orientation::Vertical, 4);
            row.set_margin_start(20);
            row.set_margin_end(20);
            row.set_margin_top(12);
            row.set_margin_bottom(12);
            let snippet = gtk::Label::new(None);
            snippet.add_css_class("snippet");
            snippet.set_xalign(0.0);
            snippet.set_wrap(true);
            snippet.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            let place = gtk::Label::new(None);
            place.add_css_class("position");
            place.add_css_class("muted");
            place.set_xalign(0.0);
            place.set_ellipsize(gtk::pango::EllipsizeMode::End);
            row.append(&snippet);
            row.append(&place);
            item.downcast_ref::<gtk::ListItem>().unwrap().set_child(Some(&row));
        });
        let list = gtk::ListView::new(Some(selection.clone()), Some(factory.clone()));
        let scrolled = gtk::ScrolledWindow::builder().child(&list).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
        panel.append(&scrolled);
        overlay.add_overlay(&panel);

        let this = Rc::new(SearchPanel {
            panel,
            field: field.clone(),
            list: list.clone(),
            selection,
            model,
            count,
            hits,
            query: RefCell::default(),
        });
        let weak = Rc::downgrade(&this);
        let view_weak = view.downgrade();
        let with = move |f: &dyn Fn(&Rc<SearchPanel>, &PageView)| {
            if let (Some(p), Some(v)) = (weak.upgrade(), view_weak.upgrade()) {
                f(&p, &v);
            }
        };

        let w = with.clone();
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let (Some(i), Some(row)) = (item.item().as_ref().and_then(index_of), item.child().and_downcast::<gtk::Box>()) else { return };
            w(&|p, v| {
                let hits = p.hits.borrow();
                let Some(hit) = hits.get(i) else { return };
                let ink = format!("#{:06x}", THEMES[v.theme()].ink);
                let (a, m, b) = (&hit.snippet[..hit.matched.start], &hit.snippet[hit.matched.clone()], &hit.snippet[hit.matched.end..]);
                let markup = format!(
                    "{}<span foreground=\"{ink}\" weight=\"bold\">{}</span>{}",
                    glib::markup_escape_text(a),
                    glib::markup_escape_text(m),
                    glib::markup_escape_text(b)
                );
                let snippet = row.first_child().and_downcast::<gtk::Label>().unwrap();
                snippet.set_markup(&markup);
                let place = snippet.next_sibling().and_downcast::<gtk::Label>().unwrap();
                let percent = format!("{}%", (hit.position * 100.0).round() as u32);
                place.set_text(&hit.section.as_ref().map_or(percent.clone(), |s| format!("{s} · {percent}")));
            })
        });

        // Searched as the reader types, a moment after the last key, on a worker thread.
        let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
        let w = with.clone();
        field.connect_changed(move |field| {
            if let Some(id) = pending.borrow_mut().take() {
                id.remove();
            }
            let query = field.text().to_string();
            let (w, p) = (w.clone(), pending.clone());
            *pending.borrow_mut() = Some(glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                p.borrow_mut().take();
                w(&|panel, view| panel.search(view, query.clone()));
            }));
        });
        let w = with.clone();
        clear.connect_clicked(move |_| w(&|p, _| p.field.set_text("")));
        let w = with.clone();
        list.connect_activate(move |_, _| w(&|p, v| p.pick(v)));
        let w = with.clone();
        field.connect_activate(move |_| w(&|p, v| p.pick(v)));

        let keys = gtk::ShortcutController::new();
        let w = with.clone();
        shortcut(&keys, "j", move || w(&|p, _| p.step(1)));
        let w = with.clone();
        shortcut(&keys, "k", move || w(&|p, _| p.step(-1)));
        let w = with.clone();
        shortcut(&keys, "slash", move || w(&|p, _| p.focus_field()));
        let w = with.clone();
        shortcut(&keys, "Escape|q", move || w(&|p, v| p.close(v)));
        list.add_controller(keys);

        let field_keys = gtk::ShortcutController::new();
        let w = with.clone();
        shortcut(&field_keys, "Escape", move || {
            w(&|p, v| {
                if p.field.text().is_empty() { p.close(v) } else { p.field.set_text("") }
            })
        });
        shortcut(&field_keys, "Down", move || with(&|p, _| p.select(p.selection.selected())));
        field.add_controller(field_keys);
        this
    }

    /// Shows the panel with the field focused, the last query selected.
    pub fn open(&self, view: &PageView) {
        self.panel.set_width_request(panel_width(view.parent().map_or(0, |p| p.width())));
        self.panel.set_visible(true);
        self.focus_field();
    }

    fn focus_field(&self) {
        self.field.grab_focus();
        self.field.select_region(0, -1);
    }

    fn close(&self, view: &PageView) {
        self.panel.set_visible(false);
        view.grab_focus();
    }

    fn search(self: &Rc<Self>, view: &PageView, query: String) {
        *self.query.borrow_mut() = query.clone();
        let (book, weak, q) = (view.book(), Rc::downgrade(self), query.clone());
        glib::spawn_future_local(async move {
            let Ok(hits) = gio::spawn_blocking(move || forzac_core::search::search(&book, &q)).await else { return };
            if let Some(panel) = weak.upgrade() {
                panel.show(query, hits);
            }
        });
    }

    fn show(&self, query: String, hits: Vec<Hit>) {
        if *self.query.borrow() != query {
            return; // a newer query is running
        }
        let count = match hits.len() {
            _ if query.trim().chars().count() < 2 => String::new(),
            0 => "No matches".to_owned(),
            1 => "1 match".to_owned(),
            n => format!("{n} matches"),
        };
        self.count.set_text(&count);
        let n = hits.len();
        *self.hits.borrow_mut() = Rc::new(hits);
        let indices: Vec<String> = (0..n).map(|i| i.to_string()).collect();
        self.model.splice(0, self.model.n_items(), &indices.iter().map(String::as_str).collect::<Vec<_>>());
    }

    fn select(&self, position: u32) {
        if position < self.selection.n_items() {
            self.list.scroll_to(position, gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT, None);
        }
    }

    fn step(&self, delta: i64) {
        let n = self.selection.n_items() as i64;
        if n > 0 {
            self.select((self.selection.selected() as i64 + delta).clamp(0, n - 1) as u32);
        }
    }

    fn pick(&self, view: &PageView) {
        let hits = self.hits.borrow().clone();
        if hits.is_empty() {
            return;
        }
        let i = (self.selection.selected() as usize).min(hits.len() - 1);
        self.close(view);
        view.go_to_match(hits, i);
    }
}

/// Note `id` in a bottom sheet over the page, as the Android note (`NoteSheet.kt`, the design's `Footnote.dc.html`): a
/// grabber, "NOTE" with the note's title, then the note typeset by the core. `Esc`, `Enter` or `q` closes it.
pub fn note(overlay: &gtk::Overlay, view: &PageView, id: &str) {
    crate::fonts::add_interface_fonts();
    let width = overlay.width().min(SHEET_WIDTH);
    let Some((node, height)) = view.note(id, (width - 48) as f32) else { return };
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    panel.add_css_class("forzac-panel");
    panel.add_css_class("forzac-sheet");
    panel.add_css_class("note");
    panel.set_valign(gtk::Align::End);
    panel.set_halign(gtk::Align::Center);
    panel.set_width_request(width);
    panel.append(&grabber());
    let title = view.book().notes.get(id).and_then(|n| n.title.clone()).filter(|t| !t.trim().is_empty());
    let label = gtk::Label::new(Some(&["Note", title.as_deref().unwrap_or("")].join(" ").trim().to_uppercase()));
    label.add_css_class("note-label");
    label.add_css_class("muted");
    label.set_xalign(0.0);
    panel.append(&label);
    let note = NodeView::new(node, (width - 48) as f32, height);
    note.set_focusable(true);
    let scrolled = gtk::ScrolledWindow::builder()
        .child(&note)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        // 60 % of the window for the whole sheet, as on Android.
        .max_content_height((overlay.height() as f32 * 0.6) as i32 - 80)
        .build();
    panel.append(&scrolled);
    overlay.add_overlay(&panel);
    let keys = gtk::ShortcutController::new();
    let (o, p, v) = (overlay.clone(), panel.clone(), view.clone());
    shortcut(&keys, "Escape|Return|KP_Enter|q|space", move || {
        o.remove_overlay(&p);
        v.grab_focus();
    });
    note.add_controller(keys);
    note.grab_focus();
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct NodeView {
        pub node: RefCell<Option<gsk::RenderNode>>,
        pub size: std::cell::Cell<(f32, f32)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NodeView {
        const NAME: &'static str = "ForzacNodeView";
        type Type = super::NodeView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for NodeView {}

    impl WidgetImpl for NodeView {
        fn measure(&self, orientation: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            let (w, h) = self.size.get();
            let size = if orientation == gtk::Orientation::Horizontal { w } else { h }.ceil() as i32;
            (size, size, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            // Centred when the panel is wider than the node.
            let offset = ((self.obj().width() as f32 - self.size.get().0) / 2.0).max(0.0);
            if let Some(node) = self.node.borrow().as_ref() {
                snapshot.translate(&gtk::graphene::Point::new(offset, 0.0));
                snapshot.append_node(node);
            }
        }
    }
}

glib::wrapper! {
    /// A render node of a fixed size: a note typeset by the core, a book's cover.
    pub struct NodeView(ObjectSubclass<imp::NodeView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NodeView {
    fn new(node: gsk::RenderNode, width: f32, height: f32) -> Self {
        let view: Self = glib::Object::new();
        *view.imp().node.borrow_mut() = Some(node);
        view.imp().size.set((width, height));
        view
    }
}
