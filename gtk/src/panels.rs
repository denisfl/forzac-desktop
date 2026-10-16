//! Panels over the page: the contents (`o`) and a note. They use the reader's theme and take the focus while open;
//! closing one gives the focus back to the page.

use std::cell::RefCell;
use std::rc::Rc;

use forzac_core::Locator;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, gsk, prelude::*};

use crate::page_view::{PageView, THEMES};

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
        let (_, bg, ink, link) = THEMES[theme];
        let hex = |c: u32| format!("#{c:06x}");
        self.0.load_from_string(&format!(
            ".forzac-panel {{ background-color: {bg}; color: {ink}; }}
             .forzac-panel.side {{ border-left: 1px solid alpha({ink}, 0.15); box-shadow: -8px 0 24px alpha(black, 0.12); }}
             .forzac-panel.bottom {{ border-top: 1px solid alpha({ink}, 0.15); box-shadow: 0 -8px 24px alpha(black, 0.12); }}
             .forzac-panel listview, .forzac-panel row, .forzac-panel scrolledwindow {{ background: transparent; color: {ink}; }}
             .forzac-panel row:selected {{ background-color: alpha({link}, 0.22); color: {ink}; }}
             .forzac-panel .dim {{ color: alpha({ink}, 0.6); }}
             .forzac-panel entry {{ background-color: alpha({ink}, 0.06); color: {ink}; margin: 10px 12px 4px 12px; }}",
            bg = hex(bg),
            ink = hex(ink),
            link = hex(link),
        ));
    }
}

/// Lower case and "ё" as "е", for filtering the contents.
fn fold(s: &str) -> String {
    s.to_lowercase().replace('ё', "е")
}

fn shortcut(controller: &gtk::ShortcutController, trigger: &str, f: impl Fn() + 'static) {
    let action = gtk::CallbackAction::new(move |_, _| {
        f();
        glib::Propagation::Stop
    });
    controller.add_shortcut(gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(trigger), Some(action)));
}

/// The contents panel of a reader window: titled sections with their position in the book. Built once, hidden (a
/// list view: only the rows on screen exist); `open` shows it with the current entry selected. `Enter` jumps (remembered
/// for `Ctrl+O`), `/` filters, `Esc` or `o` closes.
pub struct Contents {
    panel: gtk::Box,
    list: gtk::ListView,
    search: gtk::SearchEntry,
    selection: gtk::SingleSelection,
    places: Rc<Vec<Locator>>,
}

impl Contents {
    pub fn new(overlay: &gtk::Overlay, view: &PageView) -> Rc<Self> {
        let entries = view.contents();
        let book = view.book();
        let places: Rc<Vec<Locator>> = Rc::new(entries.iter().map(|e| e.2).collect());
        let positions = forzac_core::state::progress_of(&book, &places);
        let titles: Vec<String> = entries.iter().map(|e| fold(&e.0)).collect();

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
        let filtered = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
        let selection = gtk::SingleSelection::new(Some(filtered));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.set_margin_top(7);
            row.set_margin_bottom(7);
            row.set_margin_end(16);
            let t = gtk::Label::new(None);
            t.set_xalign(0.0);
            t.set_hexpand(true);
            t.set_wrap(true);
            t.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            let p = gtk::Label::new(None);
            p.add_css_class("dim");
            p.set_valign(gtk::Align::Start);
            row.append(&t);
            row.append(&p);
            item.downcast_ref::<gtk::ListItem>().unwrap().set_child(Some(&row));
        });
        factory.connect_bind(move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let (Some(i), Some(row)) = (item.item().as_ref().and_then(index_of), item.child().and_downcast::<gtk::Box>()) else { return };
            let (title, level, _) = &entries[i];
            row.set_margin_start(16 + 18 * (*level as i32 - 1).clamp(0, 4));
            let t = row.first_child().and_downcast::<gtk::Label>().unwrap();
            t.set_text(title);
            let p = t.next_sibling().and_downcast::<gtk::Label>().unwrap();
            p.set_text(&format!("{}%", (positions[i] * 100.0).round() as u32));
        });
        let list = gtk::ListView::new(Some(selection.clone()), Some(factory));

        let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
        panel.add_css_class("forzac-panel");
        panel.add_css_class("side");
        panel.set_halign(gtk::Align::End);
        panel.set_visible(false);
        let search = gtk::SearchEntry::new();
        search.set_visible(false);
        panel.append(&search);
        let scrolled = gtk::ScrolledWindow::builder().child(&list).hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
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

        let contents = Rc::new(Contents { panel, list: list.clone(), search: search.clone(), selection: selection.clone(), places });
        let weak = Rc::downgrade(&contents);
        let view = view.downgrade();
        let with = move |f: &dyn Fn(&Contents, &PageView)| {
            if let (Some(c), Some(v)) = (weak.upgrade(), view.upgrade()) {
                f(&c, &v);
            }
        };

        let w = with.clone();
        search.connect_search_changed(move |entry| {
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
            s.set_visible(true);
            s.grab_focus();
        });
        let w = with.clone();
        shortcut(&keys, "Escape|o|q", move || w(&|c, v| c.close(v)));
        list.add_controller(keys);

        // Esc in the filter clears it, then hides it; ↓ goes to the list.
        let entry_keys = gtk::ShortcutController::new();
        let w = with.clone();
        shortcut(&entry_keys, "Escape", move || {
            w(&|c, _| {
                if c.search.text().is_empty() {
                    c.search.set_visible(false);
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
        self.search.set_text("");
        self.search.set_visible(false);
        self.panel.set_width_request(panel_width(view.parent().map_or(0, |p| p.width())));
        self.panel.set_opacity(1.0);
        self.panel.set_can_target(true);
        self.panel.set_visible(true);
        let current = self.places.iter().rposition(|p| *p <= here).unwrap_or(0) as u32;
        self.select(current);
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

/// The contents panel's width in a window `width` wide.
fn panel_width(width: i32) -> i32 {
    ((width as f32 * 0.45) as i32).clamp(280, 460)
}

/// The entry index an item of the contents model stands for.
fn index_of(item: &glib::Object) -> Option<usize> {
    item.downcast_ref::<gtk::StringObject>()?.string().parse().ok()
}

/// Note `id` in a panel over the lower part of the page, typeset by the core; `Esc`, `Enter` or `q` closes it.
pub fn note(overlay: &gtk::Overlay, view: &PageView, id: &str) {
    let width = (overlay.width() as f32).min(760.0);
    let Some((node, height)) = view.note(id, width) else { return };
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("forzac-panel");
    panel.add_css_class("bottom");
    panel.set_valign(gtk::Align::End);
    let note = NoteView::new(node, width, height);
    let scrolled = gtk::ScrolledWindow::builder()
        .child(&note)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(overlay.height() / 2)
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
    pub struct NoteView {
        pub node: RefCell<Option<gsk::RenderNode>>,
        pub size: std::cell::Cell<(f32, f32)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NoteView {
        const NAME: &'static str = "ForzacNoteView";
        type Type = super::NoteView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for NoteView {}

    impl WidgetImpl for NoteView {
        fn measure(&self, orientation: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            let (w, h) = self.size.get();
            let size = if orientation == gtk::Orientation::Horizontal { w } else { h }.ceil() as i32;
            (size, size, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            // Centred when the panel is wider than the note's column.
            let offset = ((self.obj().width() as f32 - self.size.get().0) / 2.0).max(0.0);
            if let Some(node) = self.node.borrow().as_ref() {
                snapshot.translate(&gtk::graphene::Point::new(offset, 0.0));
                snapshot.append_node(node);
            }
        }
    }
}

glib::wrapper! {
    /// A note typeset by the core, drawn like a page.
    pub struct NoteView(ObjectSubclass<imp::NoteView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl NoteView {
    fn new(node: gsk::RenderNode, width: f32, height: f32) -> Self {
        let view: Self = glib::Object::new();
        *view.imp().node.borrow_mut() = Some(node);
        view.imp().size.set((width, height));
        view.set_focusable(true);
        view
    }
}
