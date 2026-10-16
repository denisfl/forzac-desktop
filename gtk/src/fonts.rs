//! Pango fonts for the core's font ids, through a fontconfig configuration that holds only the files the core shaped
//! with: the bundled faces and the fallback fonts it registered.
//!
//! The core shaped the text with these exact files; a system font of the same family (PT Serif is common) could have
//! different glyph ids, so the system font set is kept out of this font map. It is consulted only to find a fallback
//! file for a character the book face lacks.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::PathBuf;

use forzac_core::Face;
use forzac_core::fonts::{self, FontId, FontSource};
use gtk::glib::translate::ToGlibPtr;
use gtk::pango;
use gtk::prelude::*;

#[repr(C)]
struct FcFontSet {
    nfont: c_int,
    sfont: c_int,
    fonts: *mut *mut c_void,
}

const FC_MATCH_PATTERN: c_int = 0;
const FC_RESULT_MATCH: c_int = 0;

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigCreate() -> *mut c_void;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const c_char) -> c_int;
    fn FcInitLoadConfigAndFonts() -> *mut c_void;
    fn FcPatternCreate() -> *mut c_void;
    fn FcPatternDestroy(p: *mut c_void);
    fn FcPatternAddString(p: *mut c_void, object: *const c_char, s: *const c_char) -> c_int;
    fn FcPatternAddCharSet(p: *mut c_void, object: *const c_char, c: *const c_void) -> c_int;
    fn FcPatternGetCharSet(p: *const c_void, object: *const c_char, n: c_int, c: *mut *mut c_void) -> c_int;
    fn FcPatternGetString(p: *const c_void, object: *const c_char, n: c_int, s: *mut *const c_char) -> c_int;
    fn FcPatternGetInteger(p: *const c_void, object: *const c_char, n: c_int, i: *mut c_int) -> c_int;
    fn FcCharSetCreate() -> *mut c_void;
    fn FcCharSetDestroy(c: *mut c_void);
    fn FcCharSetAddChar(c: *mut c_void, ucs4: u32) -> c_int;
    fn FcCharSetHasChar(c: *const c_void, ucs4: u32) -> c_int;
    fn FcConfigSubstitute(config: *mut c_void, p: *mut c_void, kind: c_int) -> c_int;
    fn FcDefaultSubstitute(p: *mut c_void);
    fn FcFontSort(config: *mut c_void, p: *mut c_void, trim: c_int, csp: *mut *mut c_void, result: *mut c_int) -> *mut FcFontSet;
    fn FcFontSetDestroy(s: *mut FcFontSet);
}

#[link(name = "pangoft2-1.0")]
unsafe extern "C" {
    fn pango_fc_font_map_set_config(fontmap: *mut c_void, config: *mut c_void);
    fn pango_fc_font_map_config_changed(fontmap: *mut c_void);
}

/// Where the font files are: `FORZAC_ASSETS`, then next to the binary (`../share/forzac`), then the source tree.
pub fn assets() -> PathBuf {
    if let Some(dir) = std::env::var_os("FORZAC_ASSETS") {
        return dir.into();
    }
    let installed = std::env::current_exe().ok().and_then(|e| Some(e.parent()?.parent()?.join("share/forzac")));
    match installed {
        Some(dir) if dir.join("fonts").is_dir() => dir,
        _ => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")),
    }
}

/// Makes the interface font (Golos Text) and the book families usable by name in GTK's own widgets: their files join
/// the process's default fontconfig configuration. Done once, when the first panel is built: reading the files takes
/// about 10 ms on a Raspberry Pi 5, which the start does not pay.
pub fn add_interface_fonts() {
    static ADDED: std::sync::Once = std::sync::Once::new();
    ADDED.call_once(|| {
        let fonts = assets().join("fonts");
        for family in ["golos-text", "pt-serif", "literata", "source-serif", "pt-sans"] {
            let Ok(files) = std::fs::read_dir(fonts.join(family)) else { continue };
            for path in files.flatten().map(|f| f.path()).filter(|p| p.extension().is_some_and(|e| e == "ttf")) {
                if let Ok(path) = CString::new(path.as_os_str().as_encoded_bytes()) {
                    // A null configuration is the current (default) one.
                    unsafe { FcConfigAppFontAddFile(std::ptr::null_mut(), path.as_ptr()) };
                }
            }
        }
        // GTK's widgets use the default font map, which cached the configuration before.
        unsafe { pango_fc_font_map_config_changed(Fonts::raw(&pangocairo::FontMap::default())) };
    });
}

/// The system font configuration, used only to look up fallback files from the layout threads.
struct SystemFonts(*mut c_void);
// SAFETY: fontconfig serialises access to a configuration internally (since 2.13); lookups do not modify it.
unsafe impl Send for SystemFonts {}
unsafe impl Sync for SystemFonts {}

/// Sets the core's fallback provider: the font fontconfig prefers for serif text with the character, so the user's
/// configuration decides (emoji go to the colour emoji font).
pub fn install_fallback_provider() {
    let system = SystemFonts(unsafe { FcInitLoadConfigAndFonts() });
    fonts::set_fallback_provider(Box::new(move |ch| unsafe { find(&system, ch) }));
}

unsafe fn find(system: &SystemFonts, ch: char) -> Option<(PathBuf, u32)> {
    unsafe {
        let pattern = FcPatternCreate();
        FcPatternAddString(pattern, c"family".as_ptr(), c"serif".as_ptr());
        let charset = FcCharSetCreate();
        FcCharSetAddChar(charset, ch as u32);
        FcPatternAddCharSet(pattern, c"charset".as_ptr(), charset);
        FcCharSetDestroy(charset);
        FcConfigSubstitute(system.0, pattern, FC_MATCH_PATTERN);
        FcDefaultSubstitute(pattern);
        let mut result = 0;
        let set = FcFontSort(system.0, pattern, 1, std::ptr::null_mut(), &mut result);
        FcPatternDestroy(pattern);
        if set.is_null() {
            return None;
        }
        let fonts = std::slice::from_raw_parts((*set).fonts, (*set).nfont.max(0) as usize);
        let found = fonts.iter().find_map(|&font| {
            let mut cs = std::ptr::null_mut();
            if FcPatternGetCharSet(font, c"charset".as_ptr(), 0, &mut cs) != FC_RESULT_MATCH || FcCharSetHasChar(cs, ch as u32) == 0 {
                return None;
            }
            let mut file = std::ptr::null();
            if FcPatternGetString(font, c"file".as_ptr(), 0, &mut file) != FC_RESULT_MATCH {
                return None;
            }
            let mut index = 0;
            FcPatternGetInteger(font, c"index".as_ptr(), 0, &mut index);
            Some((PathBuf::from(CStr::from_ptr(file).to_str().ok()?), index.max(0) as u32))
        });
        FcFontSetDestroy(set);
        found
    }
}

pub struct Fonts {
    map: pango::FontMap,
    context: pango::Context,
    config: *mut c_void,
    /// Fallback fonts whose files are in `config`.
    added: Vec<FontId>,
    loaded: HashMap<(FontId, u32), Option<pango::Font>>,
}

impl Fonts {
    pub fn new() -> Self {
        let map = pangocairo::FontMap::new();
        let config = unsafe { FcConfigCreate() };
        for face in Face::ALL {
            let path = CString::new(assets().join("fonts/pt-serif").join(face.file_name()).to_string_lossy().as_bytes()).unwrap();
            unsafe { FcConfigAppFontAddFile(config, path.as_ptr()) };
        }
        unsafe { pango_fc_font_map_set_config(Self::raw(&map), config) };
        let context = map.create_context();
        Fonts { map, context, config, added: Vec::new(), loaded: HashMap::new() }
    }

    fn raw(map: &pango::FontMap) -> *mut c_void {
        let ptr: *mut pango::ffi::PangoFontMap = map.to_glib_none().0;
        ptr.cast()
    }

    /// Font `id` at `size` logical pixels; `None` for a fallback font that cannot be loaded.
    pub fn get(&mut self, id: FontId, size: f32) -> Option<&pango::Font> {
        let key = (id, size.to_bits());
        if !self.loaded.contains_key(&key) {
            let mut desc = pango::FontDescription::new();
            match id.face() {
                Some(face) => {
                    desc.set_family(Face::FAMILY);
                    desc.set_style(if face.is_italic() { pango::Style::Italic } else { pango::Style::Normal });
                    desc.set_weight(if face.is_bold() { pango::Weight::Bold } else { pango::Weight::Normal });
                }
                None => {
                    // A fallback font, or a face of another book family: by the file's family name, and for a
                    // family face its style and location on the variation axes (the core shaped that instance).
                    self.add_file(id);
                    desc.set_family(&fonts::font_family(id).unwrap_or_default());
                    if let Some(face) = fonts::font_style(id) {
                        desc.set_style(if face.is_italic() { pango::Style::Italic } else { pango::Style::Normal });
                        desc.set_weight(if face.is_bold() { pango::Weight::Bold } else { pango::Weight::Normal });
                    }
                    if let Some(FontSource::File { variations, .. }) = fonts::font_source(id)
                        && !variations.is_empty()
                    {
                        let axes: Vec<String> = variations.iter().map(|(tag, value)| format!("{tag}={value}")).collect();
                        desc.set_variations(Some(&axes.join(",")));
                    }
                }
            }
            desc.set_absolute_size(size as f64 * pango::SCALE as f64);
            let font = self.map.load_font(&self.context, &desc);
            self.loaded.insert(key, font);
        }
        self.loaded[&key].as_ref()
    }

    /// Puts the file of a fallback font into this font map: the glyph ids the core shaped are this file's.
    fn add_file(&mut self, id: FontId) {
        if self.added.contains(&id) {
            return;
        }
        self.added.push(id);
        let Some(FontSource::File { path, .. }) = fonts::font_source(id) else { return };
        let Ok(path) = CString::new(path.as_os_str().as_encoded_bytes()) else { return };
        unsafe {
            FcConfigAppFontAddFile(self.config, path.as_ptr());
            pango_fc_font_map_config_changed(Self::raw(&self.map));
        }
    }
}
