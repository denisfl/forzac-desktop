//! The settings panel (`s`): a bottom sheet laid out as the Android app's settings (`SettingsSheet.kt`, the design's
//! `Settings.dc.html`): theme circles; the text size and the alignment; font chips; line spacing and margins; pages.
//! `j`/`k` move through the seven controls in reading order (the current one is outlined), `h`/`l` change its value; a
//! click picks a value directly. Every change shows at once and is saved; `Esc` or `s` closes.

use std::cell::Cell;
use std::rc::Rc;

use forzac_core::fonts::Family;
use forzac_core::settings::{self, Align, Margins, Setting, Settings, Spacing, Turning};
use gtk::prelude::*;

use crate::icons::{self, Icon};
use crate::page_view::PageView;
use crate::panels::{SHEET_WIDTH, grabber, shortcut};

/// The controls in reading order, as the settings they change.
const CONTROLS: [Setting; 7] =
    [Setting::Theme, Setting::Size, Setting::Align, Setting::Font, Setting::Spacing, Setting::Margins, Setting::Turning];

pub struct SettingsSheet {
    panel: gtk::Box,
    /// The widget outlined for each of `CONTROLS`.
    controls: Vec<gtk::Widget>,
    current: Cell<usize>,
    circles: Vec<gtk::Button>,
    size: gtk::Label,
    smaller: gtk::Button,
    larger: gtk::Button,
    aligns: Vec<gtk::Button>,
    chips: Vec<gtk::Button>,
    spacings: Vec<gtk::Button>,
    margins: Vec<gtk::Button>,
    pages: Vec<gtk::Button>,
}

/// A button that does not take the keyboard focus: clicks change values, the keys stay with the sheet.
fn button(classes: &[&str], child: &impl IsA<gtk::Widget>) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(child));
    b.set_focusable(false);
    b.set_focus_on_click(false);
    for class in classes {
        b.add_css_class(class);
    }
    b
}

/// A row of options in a rounded frame; each option fills its share (`homogeneous`) or `width` pixels.
fn segmented(options: &[gtk::Button], width: Option<i32>) -> gtk::Box {
    let frame = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    frame.add_css_class("segmented");
    frame.set_overflow(gtk::Overflow::Hidden);
    frame.set_homogeneous(width.is_none());
    for option in options {
        option.add_css_class("segment");
        match width {
            Some(w) => option.set_width_request(w),
            None => option.set_hexpand(true),
        }
        frame.append(option);
    }
    frame
}

fn divider() -> gtk::Box {
    let d = gtk::Box::new(gtk::Orientation::Vertical, 0);
    d.add_css_class("divider");
    d
}

/// `content` with a small caption above it.
fn captioned(caption: &str, content: &impl IsA<gtk::Widget>) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let label = gtk::Label::new(Some(caption));
    label.add_css_class("caption");
    label.add_css_class("muted");
    label.set_xalign(0.0);
    column.append(&label);
    column.append(content);
    column.set_hexpand(true);
    column
}

fn select(buttons: &[gtk::Button], selected: usize) {
    for (i, b) in buttons.iter().enumerate() {
        if i == selected {
            b.add_css_class("selected");
        } else {
            b.remove_css_class("selected");
        }
    }
}

impl SettingsSheet {
    /// `save` remembers the settings and recolours the panels after a change.
    pub fn new(overlay: &gtk::Overlay, view: &PageView, save: Rc<dyn Fn()>) -> Rc<Self> {
        crate::fonts::add_interface_fonts();
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 18);
        panel.add_css_class("forzac-panel");
        panel.add_css_class("forzac-sheet");
        panel.set_valign(gtk::Align::End);
        panel.set_halign(gtk::Align::Center);
        panel.set_focusable(true);
        panel.append(&grabber());

        // Themes: a circle in each theme's colours, the name below.
        let themes = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        themes.set_homogeneous(true);
        let circles: Vec<gtk::Button> = settings::THEMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let cell = gtk::Box::new(gtk::Orientation::Vertical, 8);
                let aa = gtk::Label::new(Some("Aa"));
                let circle = button(&["circle", &format!("theme-{i}")], &aa);
                circle.set_halign(gtk::Align::Center);
                cell.append(&circle);
                let label = gtk::Label::new(Some(name));
                label.add_css_class("theme-name");
                label.add_css_class("muted");
                cell.append(&label);
                themes.append(&cell);
                circle
            })
            .collect();
        panel.append(&themes);

        // Text size and alignment.
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let smaller = button(&["size-smaller"], &gtk::Label::new(Some("A")));
        let larger = button(&["size-larger"], &gtk::Label::new(Some("A")));
        let size = gtk::Label::new(None);
        size.add_css_class("size");
        size.add_css_class("muted");
        size.set_width_request(44);
        let size_frame = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        size_frame.add_css_class("segmented");
        size_frame.set_overflow(gtk::Overflow::Hidden);
        size_frame.set_hexpand(true);
        smaller.set_hexpand(true);
        larger.set_hexpand(true);
        size_frame.append(&smaller);
        size_frame.append(&divider());
        size_frame.append(&size);
        size_frame.append(&divider());
        size_frame.append(&larger);
        row.append(&size_frame);
        let aligns: Vec<gtk::Button> = [icons::ALIGN_JUSTIFY, icons::ALIGN_LEFT].iter().map(|p| button(&[], &Icon::new(p, 20))).collect();
        let align_frame = segmented(&aligns, Some(48));
        row.append(&align_frame);
        panel.append(&row);

        // Fonts, each name in its own font.
        let fonts = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let chips: Vec<gtk::Button> = Family::ALL
            .iter()
            .map(|f| {
                let chip = button(&["chip", &format!("font-{}", f.key())], &gtk::Label::new(Some(f.name())));
                fonts.append(&chip);
                chip
            })
            .collect();
        panel.append(&fonts);

        // Line spacing and margins.
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        row.set_homogeneous(true);
        let spacings: Vec<gtk::Button> = icons::SPACING.iter().map(|p| button(&[], &Icon::new(p, 20))).collect();
        let spacing_frame = segmented(&spacings, None);
        row.append(&captioned("Line spacing", &spacing_frame));
        let margins: Vec<gtk::Button> = icons::MARGINS.iter().map(|p| button(&[], &Icon::new(p, 20))).collect();
        let margin_frame = segmented(&margins, None);
        row.append(&captioned("Margins", &margin_frame));
        panel.append(&row);

        // Pages.
        let pages: Vec<gtk::Button> = ["Slide", "Scroll"].iter().map(|t| button(&[], &gtk::Label::new(Some(t)))).collect();
        let pages_frame = segmented(&pages, None);
        panel.append(&captioned("Pages", &pages_frame));

        panel.set_visible(false);
        overlay.add_overlay(&panel);

        let controls = vec![
            themes.upcast(),
            size_frame.upcast(),
            align_frame.upcast(),
            fonts.upcast(),
            spacing_frame.upcast(),
            margin_frame.upcast(),
            pages_frame.upcast(),
        ];
        let sheet = Rc::new(SettingsSheet {
            panel,
            controls,
            current: Cell::new(0),
            circles,
            size,
            smaller,
            larger,
            aligns,
            chips,
            spacings,
            margins,
            pages,
        });

        let weak = Rc::downgrade(&sheet);
        let view = view.downgrade();
        let with = Rc::new(move |f: &dyn Fn(&SettingsSheet, &PageView)| {
            if let (Some(s), Some(v)) = (weak.upgrade(), view.upgrade()) {
                f(&s, &v);
            }
        });
        // Applies a change, saves it and shows the new values.
        let apply = {
            let with = with.clone();
            Rc::new(move |change: &dyn Fn(&PageView)| {
                with(&|s, v| {
                    change(v);
                    save();
                    s.show(v);
                })
            })
        };
        let pick = |buttons: &[gtk::Button], set: fn(&mut Settings, usize)| {
            for (i, b) in buttons.iter().enumerate() {
                let apply = apply.clone();
                b.connect_clicked(move |_| {
                    apply(&|v| {
                        let mut s = v.settings();
                        set(&mut s, i);
                        v.set_settings(s);
                    })
                });
            }
        };
        pick(&sheet.circles, |s, i| s.theme = Some(i));
        pick(&sheet.aligns, |s, i| s.align = [Align::Justified, Align::Left][i]);
        pick(&sheet.chips, |s, i| s.family = Family::ALL[i]);
        pick(&sheet.spacings, |s, i| s.spacing = [Spacing::Compact, Spacing::Normal, Spacing::Loose][i]);
        pick(&sheet.margins, |s, i| s.margins = [Margins::Narrow, Margins::Medium, Margins::Wide][i]);
        pick(&sheet.pages, |s, i| s.turning = [Turning::Slide, Turning::Scroll][i]);
        for (b, delta) in [(&sheet.smaller, -1), (&sheet.larger, 1)] {
            let apply = apply.clone();
            b.connect_clicked(move |_| apply(&|v| v.step(Setting::Size, delta)));
        }

        let keys = gtk::ShortcutController::new();
        let w = with.clone();
        shortcut(&keys, "j|Down", move || w(&|s, _| s.focus(s.current.get() as i32 + 1)));
        let w = with.clone();
        shortcut(&keys, "k|Up", move || w(&|s, _| s.focus(s.current.get() as i32 - 1)));
        for (trigger, delta) in [("l|Right", 1), ("h|Left", -1)] {
            let (w, apply) = (with.clone(), apply.clone());
            shortcut(&keys, trigger, move || {
                w(&|s, _| {
                    let setting = CONTROLS[s.current.get()];
                    apply(&|v| v.step(setting, delta));
                })
            });
        }
        shortcut(&keys, "Escape|s|q", move || {
            with(&|s, v| {
                s.panel.set_visible(false);
                v.grab_focus();
            })
        });
        sheet.panel.add_controller(keys);
        sheet
    }

    pub fn open(&self, view: &PageView) {
        let width = view.parent().map_or(SHEET_WIDTH, |p| p.width()).min(SHEET_WIDTH);
        self.panel.set_width_request(width);
        self.show(view);
        self.focus(self.current.get() as i32);
        self.panel.set_visible(true);
        self.panel.grab_focus();
    }

    /// Outlines control `i` (clamped) as the one the keys change.
    fn focus(&self, i: i32) {
        let i = i.clamp(0, CONTROLS.len() as i32 - 1) as usize;
        self.controls[self.current.get()].remove_css_class("keys");
        self.controls[i].add_css_class("keys");
        self.current.set(i);
    }

    /// Marks the current values.
    fn show(&self, view: &PageView) {
        let s = view.settings();
        select(&self.circles, view.theme());
        self.size.set_text(&s.text_size().to_string());
        self.smaller.set_sensitive(s.size > 0);
        self.larger.set_sensitive(s.size < settings::TEXT_SIZES.len() - 1);
        select(&self.aligns, s.align as usize);
        select(&self.chips, Family::ALL.iter().position(|f| *f == s.family).unwrap_or(0));
        select(&self.spacings, s.spacing as usize);
        select(&self.margins, s.margins as usize);
        select(&self.pages, s.turning as usize);
    }
}
