//! The book's images beyond what the shells decode themselves (`standard-ebooks`): SVG is recognised by its content,
//! sized from its root element and rasterised here with resvg, for every shell; and the line-art test, by which a dark
//! drawing on transparency is drawn in the theme's ink.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// An image's size: raster images in pixels, SVG in the units of its root (`px`).
pub fn size(bytes: &[u8]) -> Option<(f32, f32)> {
    if is_svg(bytes) {
        return svg_size(bytes);
    }
    let size = imagesize::blob_size(bytes).ok()?;
    (size.width > 0 && size.height > 0).then_some((size.width as f32, size.height as f32))
}

/// Whether the document's root element is `svg` (after a byte order mark, the XML declaration, comments and a
/// DOCTYPE). An XHTML page with SVG inside is not an image.
pub fn is_svg(bytes: &[u8]) -> bool {
    let mut rest = &bytes[..bytes.len().min(HEAD)];
    rest = rest.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(rest);
    loop {
        rest = rest.trim_ascii_start();
        let skip = |open: &[u8], close: &[u8]| {
            rest.strip_prefix(open).map(|r| r.windows(close.len()).position(|w| w == close).map_or(&[][..], |at| &r[at + close.len()..]))
        };
        if let Some(r) = skip(b"<?", b"?>").or_else(|| skip(b"<!--", b"-->")).or_else(|| skip(b"<!DOCTYPE", b">")) {
            rest = r;
            continue;
        }
        return rest.strip_prefix(b"<svg").is_some_and(|r| r.first().is_some_and(|c| c.is_ascii_whitespace() || *c == b'>'));
    }
}

/// The root's `width` and `height` in absolute units, else its `viewBox` (one side given: the other by the
/// `viewBox`'s proportions). Only the opening tag is read.
fn svg_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(HEAD)]);
    let at = head.find("<svg")?;
    let tag = &head[at..at + head[at..].find('>')?];
    let attr = |name: &str| {
        let mut from = 0;
        while let Some(i) = tag[from..].find(name) {
            let start = from + i;
            from = start + name.len();
            if !tag[..start].ends_with(|c: char| c.is_ascii_whitespace()) {
                continue;
            }
            let rest = tag[from..].trim_start().strip_prefix('=')?.trim_start();
            let quote = rest.chars().next().filter(|q| *q == '"' || *q == '\'')?;
            return rest[1..].split(quote).next().map(str::trim);
        }
        None
    };
    let width = attr("width").and_then(pixels).filter(|w| *w > 0.0);
    let height = attr("height").and_then(pixels).filter(|h| *h > 0.0);
    if let (Some(w), Some(h)) = (width, height) {
        return Some((w, h));
    }
    let view_box: Vec<f32> = attr("viewBox")?.split([' ', ',', '\t', '\n']).filter_map(|n| n.parse().ok()).collect();
    let [_, _, w, h] = view_box[..] else { return None };
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    Some(match (width, height) {
        (Some(width), _) => (width, width * h / w),
        (_, Some(height)) => (height * w / h, height),
        _ => (w, h),
    })
}

/// A length in px; percentages and `em` are `None`: the size cannot be known from them.
fn pixels(value: &str) -> Option<f32> {
    let unit_at = value.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(value.len());
    let n: f32 = value[..unit_at].parse().ok()?;
    let scale = match value[unit_at..].trim() {
        "" | "px" => 1.0,
        "pt" => 4.0 / 3.0,
        "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        _ => return None,
    };
    Some(n * scale)
}

/// An SVG drawn into pixels: premultiplied RGBA, 8 bits per channel, rows `width * 4` bytes apart.
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    /// A line drawing to draw in the theme's ink ([`is_ink`]).
    pub ink: bool,
}

/// SVG `bytes` drawn to fit `max_w` × `max_h` pixels — the screen, the size the view fits an image to, so the page
/// and the view share one raster — but at most [`MAX_SCALE`] times its own size and [`MAX_SIDE`] on its long side.
/// `None` when it is not an SVG or cannot be read. Slow for a heavy drawing (a tenth of a second on an M1): call it
/// off the main thread. Rasters are kept ([`CACHE_BYTES`]), so a page drawn again does not draw its SVG again.
pub fn raster(bytes: &[u8], max_w: u32, max_h: u32) -> Option<Arc<Raster>> {
    let (w, h) = raster_size(bytes, max_w, max_h)?;
    let key = (fingerprint(bytes), w, h);
    if let Some(r) = cached(key) {
        return Some(r);
    }
    // One at a time: parsing megabytes of paths takes memory, and two threads asking for one drawing draw it once.
    let _drawing = DRAWING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(r) = cached(key) {
        return Some(r);
    }
    let pixmap = render(bytes, w, h)?;
    let ink = is_ink(pixmap.data(), w, h, w as usize * 4);
    let raster = Arc::new(Raster { width: w, height: h, pixels: pixmap.take(), ink });
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache.push((key, raster.clone()));
    let mut total: usize = cache.iter().map(|(_, r)| r.pixels.len()).sum();
    while total > CACHE_BYTES && cache.len() > 1 {
        total -= cache.remove(0).1.pixels.len();
    }
    Some(raster)
}

/// SVG `bytes` as a PNG fitting `max_w` × `max_h` (a cover's thumbnail), for code that decodes raster images only.
pub fn png(bytes: &[u8], max_w: u32, max_h: u32) -> Option<Vec<u8>> {
    let (w, h) = raster_size(bytes, max_w, max_h)?;
    render(bytes, w, h)?.encode_png().ok()
}

/// Rasters kept, the most recently used last.
static CACHE: Mutex<Vec<(Key, Arc<Raster>)>> = Mutex::new(Vec::new());
static DRAWING: Mutex<()> = Mutex::new(());
type Key = (u64, u32, u32);

fn cached(key: Key) -> Option<Arc<Raster>> {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let at = cache.iter().position(|(k, _)| *k == key)?;
    let entry = cache.remove(at);
    let raster = entry.1.clone();
    cache.push(entry);
    Some(raster)
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

fn raster_size(bytes: &[u8], max_w: u32, max_h: u32) -> Option<(u32, u32)> {
    if !is_svg(bytes) {
        return None;
    }
    let (w, h) = svg_size(bytes)?;
    let scale = (max_w as f32 / w).min(max_h as f32 / h).min(MAX_SCALE).min(MAX_SIDE as f32 / w.max(h));
    Some((((w * scale).round() as u32).max(1), ((h * scale).round() as u32).max(1)))
}

fn render(bytes: &[u8], w: u32, h: u32) -> Option<resvg::tiny_skia::Pixmap> {
    use resvg::usvg;
    // Nothing outside the book: an image the SVG names by path or address is not loaded; `data:` images are.
    let options = usvg::Options {
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
    let size = tree.size();
    let transform = resvg::tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap)
}

/// Whether premultiplied RGBA pixels are line art — dark, colourless lines on transparency (a schematic, an
/// engraving, a logo, Standard Ebooks' title pages), drawn in the theme's ink so that a dark theme shows them. A grid
/// of at most [`INK_GRID`] × [`INK_GRID`] points is looked at: at least 10 % of them transparent, and at least 98 %
/// of the opaque ones dark and without colour.
pub fn is_ink(pixels: &[u8], width: u32, height: u32, stride: usize) -> bool {
    let (cols, rows) = (width.min(INK_GRID) as usize, height.min(INK_GRID) as usize);
    let (mut transparent, mut opaque, mut dark) = (0, 0, 0);
    for row in 0..rows {
        for col in 0..cols {
            let (x, y) = (col * width as usize / cols, row * height as usize / rows);
            let Some(p) = pixels.get(y * stride + x * 4..y * stride + x * 4 + 4) else { return false };
            let a = p[3] as u32;
            if a < 16 {
                transparent += 1;
            }
            if a < 128 {
                continue;
            }
            opaque += 1;
            let [r, g, b] = [p[0], p[1], p[2]].map(|c| c as u32 * 255 / a);
            let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
            if hi <= 0x50 && hi - lo <= 0x20 {
                dark += 1;
            }
        }
    }
    opaque > 0 && transparent * 10 >= cols * rows && dark * 100 >= opaque * 98
}

/// How much of a file is looked at for its root element.
const HEAD: usize = 4096;
const INK_GRID: u32 = 64;
/// An SVG is not drawn bigger than this many times its own size: a small ornament does not take a screen's memory.
const MAX_SCALE: f32 = 4.0;
const MAX_SIDE: u32 = 4096;
const CACHE_BYTES: usize = 128 << 20;

#[cfg(test)]
mod tests;
