//! Pango fonts for the bundled faces, through a fontconfig configuration that holds only those files.
//!
//! The core shaped the text with these exact files; a system font of the same family (PT Serif is common) could have
//! different glyph ids, so system fonts are kept out of this font map.

use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::path::PathBuf;

use forzac_core::Face;
use gtk::glib::translate::ToGlibPtr;
use gtk::pango;
use gtk::prelude::*;

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigCreate() -> *mut c_void;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const u8) -> i32;
}

#[link(name = "pangoft2-1.0")]
unsafe extern "C" {
    fn pango_fc_font_map_set_config(fontmap: *mut c_void, config: *mut c_void);
}

/// Where the font files are: `FORZAC_ASSETS`, then next to the binary (`../share/forzac`), then the source tree.
fn assets() -> PathBuf {
    if let Some(dir) = std::env::var_os("FORZAC_ASSETS") {
        return dir.into();
    }
    let installed = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.parent()?.join("share/forzac")));
    match installed {
        Some(dir) if dir.join("fonts").is_dir() => dir,
        _ => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")),
    }
}

pub struct Fonts {
    map: pango::FontMap,
    context: pango::Context,
    loaded: HashMap<(Face, u32), pango::Font>,
}

impl Fonts {
    pub fn new() -> Self {
        let map = pangocairo::FontMap::new();
        unsafe {
            let config = FcConfigCreate();
            for face in Face::ALL {
                let path = CString::new(assets().join("fonts/pt-serif").join(face.file_name()).to_string_lossy().as_bytes()).unwrap();
                FcConfigAppFontAddFile(config, path.as_ptr().cast());
            }
            let ptr: *mut pango::ffi::PangoFontMap = map.to_glib_none().0;
            pango_fc_font_map_set_config(ptr.cast(), config);
        }
        let context = map.create_context();
        Fonts { map, context, loaded: HashMap::new() }
    }

    /// The face at `size` logical pixels.
    pub fn get(&mut self, face: Face, size: f32) -> &pango::Font {
        self.loaded.entry((face, size.to_bits())).or_insert_with(|| {
            let mut desc = pango::FontDescription::new();
            desc.set_family(Face::FAMILY);
            desc.set_style(if face.is_italic() { pango::Style::Italic } else { pango::Style::Normal });
            desc.set_weight(if face.is_bold() { pango::Weight::Bold } else { pango::Weight::Normal });
            desc.set_absolute_size(size as f64 * pango::SCALE as f64);
            self.map.load_font(&self.context, &desc).expect("bundled font")
        })
    }
}
