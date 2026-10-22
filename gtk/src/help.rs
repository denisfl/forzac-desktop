//! The help (`help`): a card over the dimmed page with every key by section, in the reader's theme. `?`, `Esc`, the
//! close button and a click beside the card hide it; while it is open it takes every key.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use forzac_core::i18n::{self, Platform};
use gtk::{gdk, gio, glib, prelude::*};

use crate::icons::{self, Icon};

const CLOSE: Duration = Duration::from_millis(150);
const SITE: &str = "forzac.app";
/// A column's width, its key area's and the meaning's (`design.md`).
const COLUMN: i32 = 416;
const KEYS: i32 = 168;
const MEANING: i32 = 236;
/// The window width from which the sections stand in two columns: two columns, the gap, the card's padding and
/// margins.
const TWO_COLUMNS: i32 = 2 * COLUMN + 40 + 2 * 28 + 2 * 24;
/// `j`/`k` scroll this far.
const LINE: f64 = 40.0;

pub struct Help {
    /// The dimming over the whole window; the card in its middle.
    pub root: gtk::Box,
    card: gtk::Box,
    scrolled: gtk::ScrolledWindow,
    sections: Vec<gtk::Box>,
    columns: [gtk::Box; 2],
    /// The first section of the second column.
    split: usize,
    open: Rc<Cell<bool>>,
    hide: Rc<RefCell<Option<glib::SourceId>>>,
}

impl Help {
    pub fn new() -> Rc<Help> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("forzac-panel");
        root.add_css_class("forzac-help");
        root.set_visible(false);

        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("help-card");
        card.set_halign(gtk::Align::Center);
        card.set_valign(gtk::Align::Center);
        card.set_vexpand(true);
        card.set_margin_top(24);
        card.set_margin_bottom(24);
        card.set_margin_start(24);
        card.set_margin_end(24);
        root.append(&card);

        let help = i18n::help(Platform::Linux);
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        head.add_css_class("help-head");
        let title = gtk::Label::new(Some(help.title));
        title.add_css_class("help-title");
        title.set_hexpand(true);
        title.set_xalign(0.0);
        let close = gtk::Button::new();
        close.set_child(Some(&Icon::new(icons::X, 20)));
        close.add_css_class("help-close");
        close.set_focusable(false);
        close.set_valign(gtk::Align::Center);
        head.append(&title);
        head.append(&close);
        card.append(&head);

        // The sections in two columns or one, arranged when the help opens (`arrange`).
        let columns = gtk::Box::new(gtk::Orientation::Horizontal, 40);
        columns.set_valign(gtk::Align::Start);
        let [left, right] = [(); 2].map(|_| {
            let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
            column.set_size_request(COLUMN, -1);
            columns.append(&column);
            column
        });
        let split = split(&help.sections.iter().map(|s| s.rows.len()).collect::<Vec<_>>());
        let sections: Vec<gtk::Box> = help.sections.iter().map(section_box).collect();
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_hscrollbar_policy(gtk::PolicyType::Never);
        scrolled.set_propagate_natural_height(true);
        scrolled.set_propagate_natural_width(true);
        scrolled.set_child(Some(&columns));
        scrolled.set_vexpand(true);
        card.append(&scrolled);

        let rule = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        rule.add_css_class("rule");
        rule.add_css_class("help-rule");
        rule.set_size_request(-1, 1);
        card.append(&rule);
        let support = gtk::Label::new(None);
        let escaped = glib::markup_escape_text(help.support);
        support.set_markup(&escaped.replace(SITE, &format!("<a href=\"https://{SITE}\">{SITE}</a>")));
        support.add_css_class("help-support");
        support.set_xalign(0.0);
        support.set_wrap(true);
        support.connect_activate_link(|label, uri| {
            let window = label.root().and_downcast::<gtk::Window>();
            gtk::UriLauncher::new(uri).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
            glib::Propagation::Stop
        });
        card.append(&support);

        let help =
            Rc::new(Help { root, card, scrolled, sections, columns: [left, right], split, open: Rc::default(), hide: Rc::default() });
        let h = Rc::downgrade(&help);
        close.connect_clicked(move |_| {
            if let Some(h) = h.upgrade() {
                h.close();
            }
        });
        // A click on the dimming beside the card hides the help; the page under it gets nothing.
        let click = gtk::GestureClick::new();
        let h = Rc::downgrade(&help);
        click.connect_pressed(move |g, _, x, y| {
            g.set_state(gtk::EventSequenceState::Claimed);
            let Some(h) = h.upgrade() else { return };
            let inside = h
                .root
                .compute_point(&h.card, &gtk::graphene::Point::new(x as f32, y as f32))
                .is_some_and(|p| h.card.contains(f64::from(p.x()), f64::from(p.y())));
            if !inside {
                h.close();
            }
        });
        help.root.add_controller(click);
        help
    }

    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    pub fn show(&self) {
        if self.open.replace(true) {
            return;
        }
        if let Some(id) = self.hide.borrow_mut().take() {
            id.remove();
        }
        self.scrolled.vadjustment().set_value(0.0);
        self.root.set_visible(true);
        // The columns by the window's width, known once it is laid out (not yet when a book opens with the help); the
        // transition needs a frame with the closed style first.
        let open = self.open.clone();
        let (sections, columns, split) = (self.sections.clone(), self.columns.clone(), self.split);
        self.root.add_tick_callback(move |root, _| {
            let width = root.parent().map_or(0, |p| p.width());
            if width == 0 && open.get() {
                return glib::ControlFlow::Continue;
            }
            arrange(&sections, &columns, split, width >= TWO_COLUMNS);
            if open.get() {
                root.add_css_class("open");
            }
            glib::ControlFlow::Break
        });
    }

    pub fn close(&self) {
        if !self.open.replace(false) {
            return;
        }
        self.root.remove_css_class("open");
        let (root, hide) = (self.root.downgrade(), self.hide.clone());
        let id = glib::timeout_add_local_once(CLOSE, move || {
            hide.borrow_mut().take(); // fired: the id is no longer valid
            if let Some(root) = root.upgrade() {
                root.set_visible(false);
            }
        });
        if let Some(old) = self.hide.replace(Some(id)) {
            old.remove();
        }
    }

    /// The keys while the help is open: `?` and `Esc` hide it, `j` `k` `↓` `↑` `Space` `PgDn` `PgUp` scroll it, the
    /// rest do nothing. False when the help is not open.
    pub fn key(&self, key: gdk::Key, latin: gdk::Key, state: gdk::ModifierType) -> bool {
        if !self.is_open() {
            return false;
        }
        let page = (self.scrolled.height() as f64 - LINE).max(LINE);
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        let step = match key {
            gdk::Key::question | gdk::Key::Escape => {
                self.close();
                return true;
            }
            gdk::Key::Down => LINE,
            gdk::Key::Up => -LINE,
            gdk::Key::Page_Down => page,
            gdk::Key::Page_Up => -page,
            gdk::Key::space if shift => -page,
            gdk::Key::space => page,
            _ => match latin {
                gdk::Key::j => LINE,
                gdk::Key::k => -LINE,
                _ => return true,
            },
        };
        let a = self.scrolled.vadjustment();
        a.set_value((a.value() + step).clamp(a.lower(), a.upper() - a.page_size()));
        true
    }
}

/// The sections in two columns, split at `split`, or all in the first.
fn arrange(sections: &[gtk::Box], columns: &[gtk::Box; 2], split: usize, two: bool) {
    let [left, right] = columns;
    for (i, section) in sections.iter().enumerate() {
        let column = if two && i >= split { right } else { left };
        if section.parent().as_ref() != Some(column.upcast_ref()) {
            if let Some(old) = section.parent().and_downcast::<gtk::Box>() {
                old.remove(section);
            }
            column.append(section);
        }
    }
    right.set_visible(two);
}

/// Where the second column starts: the split between sections that makes the taller column shortest.
fn split(rows: &[usize]) -> usize {
    let total: usize = rows.iter().sum();
    (1..rows.len())
        .min_by_key(|&i| {
            let first: usize = rows[..i].iter().sum();
            first.max(total - first)
        })
        .unwrap_or(rows.len())
}

fn section_box(section: &i18n::Section) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = gtk::Label::new(Some(&section.title.to_uppercase()));
    title.add_css_class("help-section");
    title.set_xalign(0.0);
    title.set_margin_bottom(2);
    b.append(&title);
    for row in &section.rows {
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let meaning = gtk::Label::new(Some(row.meaning));
        meaning.add_css_class("help-meaning");
        meaning.set_xalign(0.0);
        meaning.set_valign(gtk::Align::Start);
        meaning.set_wrap(true);
        meaning.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        // The width comes from the size request; one character keeps the natural width from asking for more.
        meaning.set_max_width_chars(1);
        meaning.set_hexpand(true);
        if row.keys.is_empty() {
            meaning.set_size_request(COLUMN, -1);
        } else {
            line.append(&caps(row.keys));
            meaning.set_size_request(MEANING, -1);
        }
        line.append(&meaning);
        b.append(&line);
    }
    b
}

/// The key caps in lines no wider than the key area. They are measured once they are in the window and styled, then
/// broken into lines.
fn caps(keys: &str) -> gtk::Box {
    let lines = gtk::Box::new(gtk::Orientation::Vertical, 4);
    lines.set_size_request(KEYS, -1);
    lines.set_valign(gtk::Align::Start);
    let first = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    for key in keys.split(' ') {
        let cap = gtk::Label::new(Some(key));
        cap.add_css_class("keycap");
        first.append(&cap);
    }
    lines.append(&first);
    lines.connect_realize(move |lines| {
        let caps: Vec<gtk::Widget> = std::iter::successors(first.first_child(), |c| c.next_sibling()).collect();
        let mut line = first.clone();
        let mut width = 0;
        for cap in caps {
            let w = cap.measure(gtk::Orientation::Horizontal, -1).1;
            if width > 0 && width + 4 + w > KEYS {
                line = gtk::Box::new(gtk::Orientation::Horizontal, 4);
                lines.append(&line);
                width = 0;
            }
            width += if width > 0 { 4 } else { 0 } + w;
            if cap.parent().as_ref() != Some(line.upcast_ref()) {
                first.remove(&cap);
                line.append(&cap);
            }
        }
    });
    lines
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn the_columns_are_as_even_as_the_sections_allow() {
        assert_eq!(split(&[5, 4, 5, 8, 6, 3, 3]), 3);
        assert_eq!(split(&[5, 4, 5, 7, 6, 3, 3]), 3);
        assert_eq!(split(&[1]), 1);
    }
}
