//! [Lucide](https://lucide.dev) icons (ISC) drawn from their SVG paths: a 24×24 grid, stroke 1.75, round caps and joins,
//! in the widget's CSS colour. The spacing and margin presets are drawn in Lucide's style, as in the Android app.

use std::cell::Cell;

use gtk::subclass::prelude::*;
use gtk::{glib, gsk, prelude::*};

pub type Paths = &'static [&'static str];

pub const CHEVRON_LEFT: Paths = &["m15 18-6-6 6-6"];
pub const LIST: Paths = &["M3 12h.01", "M3 18h.01", "M3 6h.01", "M8 12h13", "M8 18h13", "M8 6h13"];
pub const X: Paths = &["M18 6 6 18", "m6 6 12 12"];
pub const FOLDER: Paths =
    &["M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"];
pub const SEARCH: Paths = &["M19 11a8 8 0 1 1-16 0a8 8 0 1 1 16 0", "m21 21-4.3-4.3"];
pub const ALIGN_JUSTIFY: Paths = &["M3 6h18", "M3 12h18", "M3 18h18"];
pub const ALIGN_LEFT: Paths = &["M3 6h18", "M3 12h12", "M3 18h15"];
pub const SPACING: [Paths; 3] =
    [&["M4 8h16", "M4 12h16", "M4 16h16"], &["M4 6h16", "M4 12h16", "M4 18h16"], &["M4 4h16", "M4 12h16", "M4 20h16"]];
pub const MARGINS: [Paths; 3] = [
    &["M3 3h18v18H3z", "M6 8h12", "M6 12h12", "M6 16h12"],
    &["M3 3h18v18H3z", "M8 8h8", "M8 12h8", "M8 16h8"],
    &["M3 3h18v18H3z", "M10 8h4", "M10 12h4", "M10 16h4"],
];

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Icon {
        pub paths: Cell<Paths>,
        pub size: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Icon {
        const NAME: &'static str = "ForzacIcon";
        type Type = super::Icon;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Icon {}

    impl WidgetImpl for Icon {
        fn measure(&self, _: gtk::Orientation, _: i32) -> (i32, i32, i32, i32) {
            (self.size.get(), self.size.get(), -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let scale = self.size.get() as f32 / 24.0;
            let stroke = gsk::Stroke::new(1.75);
            stroke.set_line_cap(gsk::LineCap::Round);
            stroke.set_line_join(gsk::LineJoin::Round);
            snapshot.save();
            snapshot.scale(scale, scale);
            for d in self.paths.get() {
                if let Ok(path) = gsk::Path::parse(d) {
                    snapshot.append_stroke(&path, &stroke, &widget.color());
                }
            }
            snapshot.restore();
        }
    }
}

glib::wrapper! {
    pub struct Icon(ObjectSubclass<imp::Icon>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Icon {
    /// An icon `size` logical pixels square.
    pub fn new(paths: Paths, size: i32) -> Self {
        let icon: Self = glib::Object::new();
        icon.imp().paths.set(paths);
        icon.imp().size.set(size);
        icon.set_halign(gtk::Align::Center);
        icon.set_valign(gtk::Align::Center);
        icon
    }
}
