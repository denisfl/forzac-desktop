//! The settings panel (`s`): a bottom sheet laid out as the Android app's settings (`SettingsSheet.kt`, `aa-panel`): every
//! row runs from edge to edge in equal cells, a caption above each control: the theme cards; the text size | the
//! alignment; the font chips; line spacing | margins; pages.
//! `j`/`k` move through the seven controls in reading order (the current one is outlined), `h`/`l` change its value; a
//! click picks a value directly. Every change shows at once and is saved; `Esc` or `s` closes.

use std::cell::Cell;
use std::rc::Rc;

use forzac_core::fonts::Family;
use forzac_core::i18n::tr;
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
    /// The keys were used since the sheet opened: only then is the control they change outlined.
    keyboard: Cell<bool>,
    cards: Vec<gtk::Button>,
    /// The shown theme's name after the theme's caption.
    theme_name: gtk::Label,
    smaller: gtk::Button,
    larger: gtk::Button,
    aligns: Vec<gtk::Button>,
    chips: Vec<gtk::Button>,
    fonts: gtk::Viewport,
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

/// Equal options in a rounded frame filling its cell, 1 px dividers between them.
fn segmented(options: &[gtk::Button]) -> gtk::Box {
    let frame = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    frame.add_css_class("segmented");
    frame.set_overflow(gtk::Overflow::Hidden);
    frame.set_homogeneous(true);
    for option in options {
        option.add_css_class("segment");
        option.set_hexpand(true);
        frame.append(option);
    }
    frame
}

/// `content` with a 12 px caption 8 px above it; `value` follows the caption in ink ("Theme Light").
fn captioned(caption: &str, content: &impl IsA<gtk::Widget>, value: Option<&gtk::Label>) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let label = gtk::Label::new(Some(caption));
    label.add_css_class("caption");
    label.add_css_class("muted");
    line.append(&label);
    if let Some(value) = value {
        value.add_css_class("caption");
        value.add_css_class("caption-value");
        line.append(value);
    }
    column.append(&line);
    column.append(content);
    column.set_hexpand(true);
    column
}

/// Two equal halves 16 px apart.
fn halves(first: &gtk::Box, second: &gtk::Box) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.set_homogeneous(true);
    row.append(first);
    row.append(second);
    row
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
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 20);
        panel.add_css_class("forzac-panel");
        panel.add_css_class("forzac-sheet");
        panel.set_valign(gtk::Align::End);
        panel.set_halign(gtk::Align::Center);
        panel.set_focusable(true);
        panel.append(&grabber());

        // Themes: six equal cards, each its theme's paper with "Aa" in its ink; the shown one's name in the caption.
        let themes = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        themes.set_homogeneous(true);
        let names = [tr("Light"), tr("Sepia"), tr("Dark"), tr("Dawn"), tr("Wave"), tr("Forest")];
        let cards: Vec<gtk::Button> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let card = button(&["card", &format!("theme-{i}")], &gtk::Label::new(Some("Aa")));
                card.set_tooltip_text(Some(name));
                themes.append(&card);
                card
            })
            .collect();
        let theme_name = gtk::Label::new(None);
        panel.append(&captioned(tr("Theme"), &themes, Some(&theme_name)));

        // Text size and alignment.
        let smaller = button(&["size-smaller"], &gtk::Label::new(Some("A")));
        let larger = button(&["size-larger"], &gtk::Label::new(Some("A")));
        let size_frame = segmented(&[smaller.clone(), larger.clone()]);
        let aligns: Vec<gtk::Button> =
            [icons::ALIGN_JUSTIFY, icons::ALIGN_LEFT].iter().map(|p| button(&["option"], &Icon::new(p, 20))).collect();
        let align_frame = segmented(&aligns);
        panel.append(&halves(&captioned(tr("Size"), &size_frame, None), &captioned(tr("Alignment"), &align_frame, None)));

        // Fonts, each name in its own font. When they all fit, they stretch evenly to the row's right edge; when not,
        // they keep their widths and the row scrolls sideways.
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let chips: Vec<gtk::Button> = Family::ALL
            .iter()
            .map(|f| {
                let chip = button(&["chip", &format!("font-{}", f.key())], &gtk::Label::new(Some(f.name())));
                chip.set_hexpand(true);
                row.append(&chip);
                chip
            })
            .collect();
        let fonts = gtk::Viewport::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        fonts.set_child(Some(&row));
        let scrolled =
            gtk::ScrolledWindow::builder().child(&fonts).vscrollbar_policy(gtk::PolicyType::Never).propagate_natural_height(true).build();
        panel.append(&captioned(tr("Font"), &scrolled, None));

        // Line spacing and margins.
        let spacings: Vec<gtk::Button> = icons::SPACING.iter().map(|p| button(&["option"], &Icon::new(p, 20))).collect();
        let spacing_frame = segmented(&spacings);
        let margins: Vec<gtk::Button> = icons::MARGINS.iter().map(|p| button(&["option"], &Icon::new(p, 20))).collect();
        let margin_frame = segmented(&margins);
        panel.append(&halves(&captioned(tr("Line spacing"), &spacing_frame, None), &captioned(tr("Margins"), &margin_frame, None)));

        // Pages.
        let pages: Vec<gtk::Button> = [tr("Slide"), tr("Scroll")].iter().map(|t| button(&["option"], &gtk::Label::new(Some(t)))).collect();
        let pages_frame = segmented(&pages);
        panel.append(&captioned(tr("Pages"), &pages_frame, None));

        panel.set_visible(false);
        overlay.add_overlay(&panel);

        let controls = vec![
            themes.upcast(),
            size_frame.upcast(),
            align_frame.upcast(),
            scrolled.upcast(),
            spacing_frame.upcast(),
            margin_frame.upcast(),
            pages_frame.upcast(),
        ];
        let sheet = Rc::new(SettingsSheet {
            panel,
            controls,
            current: Cell::new(0),
            keyboard: Cell::new(false),
            cards,
            theme_name,
            smaller,
            larger,
            aligns,
            chips,
            fonts,
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
        pick(&sheet.cards, |s, i| s.theme = Some(i));
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
        shortcut(&keys, "j|Down", move || w(&|s, _| s.keys_used(s.current.get() as i32 + 1)));
        let w = with.clone();
        shortcut(&keys, "k|Up", move || w(&|s, _| s.keys_used(s.current.get() as i32 - 1)));
        for (trigger, delta) in [("l|Right", 1), ("h|Left", -1)] {
            let (w, apply) = (with.clone(), apply.clone());
            shortcut(&keys, trigger, move || {
                w(&|s, _| {
                    s.keys_used(s.current.get() as i32);
                    let setting = CONTROLS[s.current.get()];
                    apply(&|v| v.step(setting, delta));
                })
            });
        }
        shortcut(&keys, "Escape|s|q", move || with(&|s, v| s.close(v)));
        sheet.panel.add_controller(keys);
        sheet
    }

    pub fn open(&self, view: &PageView) {
        let width = view.parent().map_or(SHEET_WIDTH, |p| p.width()).min(SHEET_WIDTH);
        self.panel.set_width_request(width);
        self.show(view);
        self.keyboard.set(false);
        self.controls[self.current.get()].remove_css_class("keys");
        self.panel.set_visible(true);
        self.panel.grab_focus();
    }

    pub fn is_open(&self) -> bool {
        self.panel.is_visible()
    }

    pub fn close(&self, view: &PageView) {
        self.panel.set_visible(false);
        view.grab_focus();
    }

    /// Makes control `i` (clamped) the one the keys change and outlines it: the outline shows once the keys are used,
    /// not for the mouse.
    fn keys_used(&self, i: i32) {
        self.keyboard.set(true);
        let i = i.clamp(0, CONTROLS.len() as i32 - 1) as usize;
        self.controls[self.current.get()].remove_css_class("keys");
        self.controls[i].add_css_class("keys");
        self.current.set(i);
    }

    /// Marks the current values.
    fn show(&self, view: &PageView) {
        let s = view.settings();
        select(&self.cards, view.theme());
        self.theme_name.set_text(tr(settings::THEMES[view.theme()]));
        self.smaller.set_sensitive(s.size > 0);
        self.larger.set_sensitive(s.size < settings::TEXT_SIZES.len() - 1);
        select(&self.aligns, s.align as usize);
        let font = Family::ALL.iter().position(|f| *f == s.family).unwrap_or(0);
        select(&self.chips, font);
        // The keys may pick a chip beyond the edge.
        self.fonts.scroll_to(&self.chips[font], None);
        select(&self.spacings, s.spacing as usize);
        select(&self.margins, s.margins as usize);
        select(&self.pages, s.turning as usize);
    }
}
