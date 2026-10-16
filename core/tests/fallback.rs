//! Glyph fallback through a provider. A separate test binary: the provider is global to the process.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use forzac_core::fonts::{self, Face, FontSource};
use forzac_core::model::{Kind, StyledText};
use forzac_core::{Block, Book, Chapter, PageStyle, Typesetter, paginate};

/// A system font with arrows and Greek, which PT Serif lacks in part: DejaVu on Linux, Arial Unicode on macOS.
fn system_font() -> PathBuf {
    [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ]
    .iter()
    .map(Path::new)
    .find(|p| p.exists())
    .expect("needs DejaVu Sans (Linux) or Arial Unicode (macOS)")
    .to_owned()
}

#[test]
fn missing_characters_go_to_the_provider_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let font = system_font();
    let counter = calls.clone();
    fonts::set_fallback_provider(Box::new(move |ch| {
        counter.fetch_add(1, Ordering::SeqCst);
        (ch != '\u{F8FF}').then(|| (font.clone(), 0))
    }));

    let arrow = fonts::font_for(Face::Regular.id(), '→');
    assert_ne!(arrow, Face::Regular.id());
    assert!(matches!(fonts::font_source(arrow), Some(FontSource::File { .. })));
    assert!(!fonts::font_is_color(arrow));
    // Cached per character; the same file is one font for every character it has.
    assert_eq!(fonts::font_for(Face::Bold.id(), '→'), arrow);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(fonts::font_for(Face::Regular.id(), '⇒'), arrow, "an already registered font is tried first");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // Nothing has it: the book face, which draws a box.
    assert_eq!(fonts::font_for(Face::Italic.id(), '\u{F8FF}'), Face::Italic.id());

    // A paragraph with an arrow: the arrow is one run in the fallback font, the words around it stay in PT Serif.
    let text = StyledText { text: "туда → обратно".into(), styles: vec![] };
    let book =
        Book { chapters: vec![Chapter { title: None, blocks: vec![Block::Paragraph { text, kind: Kind::Body }] }], ..Book::default() };
    let style = PageStyle::for_window(600.0, 800.0, 20.0);
    let mut t = Typesetter::new(style, None);
    let page = paginate(&book, 0, &style, &mut t).render(0, &style, &mut t);
    let fonts: Vec<_> = page.runs.iter().map(|r| r.font).collect();
    assert_eq!(&fonts[..3], [Face::Regular.id(), arrow, Face::Regular.id()]);
    assert!(page.runs.iter().all(|r| r.glyphs.iter().all(|&g| g != 0)), "no missing glyphs");
}
