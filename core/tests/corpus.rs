//! Opens every book in a local corpus and typesets every page:
//! `FORZAC_CORPUS=~/Books cargo test --release -p forzac-core --test corpus -- --nocapture`.
//! Books are never committed; without `FORZAC_CORPUS` the test is skipped.

use std::collections::BTreeMap;
use std::path::PathBuf;

use forzac_core::model::Style;
use forzac_core::{Block, Book, Error};

/// FB2, FB3 and EPUB files under `FORZAC_CORPUS` (not recursive), or `None` when the variable is unset.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = std::env::var_os("FORZAC_CORPUS")?;
    let mut books: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("FORZAC_CORPUS is not a readable directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| format(p).is_some())
        .collect();
    books.sort();
    Some(books)
}

fn format(path: &std::path::Path) -> Option<&'static str> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    [(".fb2", "FB2"), (".fb2.zip", "FB2"), (".fb3", "FB3"), (".epub", "EPUB")].iter().find(|(ext, _)| name.ends_with(ext)).map(|(_, f)| *f)
}

/// What a format's books hold, and what their pages draw.
#[derive(Debug, Default)]
struct Counts {
    books: usize,
    chapters: usize,
    contents: usize,
    notes: usize,
    note_markers: usize,
    links: usize,
    images: usize,
    /// Image blocks without a raster image in the book (SVG, missing files): they take no room.
    unshown_images: usize,
    tables: usize,
    pages: usize,
    drawn_images: usize,
    rules: usize,
    areas: usize,
    /// Glyphs no font had (drawn as a box); without a fallback provider these are the characters PT Serif lacks.
    missing_glyphs: usize,
}

#[test]
fn every_book_opens() {
    let Some(books) = corpus() else {
        eprintln!("skipped: set FORZAC_CORPUS to a directory of books");
        return;
    };
    let mut failures = Vec::new();
    let mut without_text = Vec::new();
    let mut counts: BTreeMap<&str, Counts> = BTreeMap::new();
    for path in &books {
        let name = path.file_name().unwrap().to_string_lossy();
        let book = match forzac_core::open(path) {
            Ok(book) => book,
            // Picture books (fixed-layout pages of images) have no text; they are not supported.
            Err(Error::NoText) => {
                without_text.push(name.into_owned());
                continue;
            }
            Err(e) => {
                failures.push(format!("{name}: {e:?}"));
                continue;
            }
        };
        let c = counts.entry(format(path).unwrap()).or_default();
        if let Err(problem) = check(&book) {
            failures.push(format!("{name}: {problem}"));
        }
        let blocks: Vec<&Block> = book.chapters.iter().flat_map(|ch| &ch.blocks).collect();
        let markers = blocks.iter().filter_map(|b| b.text()).flat_map(|t| &t.styles).filter(|s| s.style == Style::NoteRef).count();
        let unshown = blocks.iter().filter(|b| matches!(b, Block::Image { id } if !book.images.contains_key(id))).count();
        let pages_before = c.pages;
        typeset_all(&book, c);
        eprintln!(
            "{:<5} ch {:>4}  toc {:>4}  pages {:>5}  notes {:>4}/{:>4}  links {:>5}  img {:>4} (unshown {:>3})  {}",
            format(path).unwrap(),
            book.chapters.len(),
            book.toc.len(),
            c.pages - pages_before,
            markers,
            book.notes.len(),
            book.links.len(),
            book.images.len(),
            unshown,
            name.chars().take(60).collect::<String>()
        );
        c.books += 1;
        c.chapters += book.chapters.len();
        c.contents += book.toc.len();
        c.notes += book.notes.len();
        c.note_markers += markers;
        c.links += book.links.len();
        c.images += book.images.len();
        c.unshown_images += unshown;
        c.tables += blocks.iter().filter(|b| matches!(b, Block::TableRow { row: 0, .. })).count();
    }
    for (format, c) in &counts {
        eprintln!("{format}: {c:?}");
    }
    eprintln!("{} books, {} failed, {} without text: {}", books.len(), failures.len(), without_text.len(), without_text.join(", "));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A title, chapters, every note marker leads to its note, every contents entry points inside the book.
fn check(book: &Book) -> Result<(), String> {
    if book.title.trim().is_empty() {
        return Err("no title".into());
    }
    if book.chapters.is_empty() {
        return Err("no chapters".into());
    }
    let markers = book.chapters.iter().flat_map(|c| &c.blocks).filter_map(|b| b.text()).flat_map(|t| &t.styles);
    if let Some(m) = markers.filter(|s| s.style == Style::NoteRef).find(|s| !s.target.as_ref().is_some_and(|t| book.notes.contains_key(t)))
    {
        return Err(format!("note marker without a note: {:?}", m.target));
    }
    if let Some(e) =
        book.toc.iter().find(|e| e.target.chapter >= book.chapters.len() || e.target.block > book.chapters[e.target.chapter].blocks.len())
    {
        return Err(format!("contents entry outside the book: {} → {:?}", e.title, e.target));
    }
    Ok(())
}

/// Lays out every chapter and renders every page: the whole chain must hold for real books.
fn typeset_all(book: &Book, c: &mut Counts) {
    let style = forzac_core::PageStyle::for_window(760.0, 980.0, 20.0);
    let hyphenator = forzac_core::hyphen::Hyphenator::for_language(book.language.as_deref()).map(std::sync::Arc::new);
    let mut typesetter = forzac_core::Typesetter::new(style, hyphenator);
    for chapter in 0..book.chapters.len() {
        let layout = forzac_core::paginate(book, chapter, &style, &mut typesetter);
        for page in 0..layout.pages.len() {
            let content = layout.render(page, &style, &mut typesetter);
            assert!(!content.runs.is_empty(), "every page has at least its number");
            c.drawn_images += content.images.len();
            c.rules += content.rules.len();
            c.areas += content.areas.len();
            c.missing_glyphs += content.runs.iter().flat_map(|r| &r.glyphs).filter(|&&g| g == 0).count();
        }
        c.pages += layout.pages.len();
    }
}

/// A common word searched in every book of the corpus: the slowest whole-book search, against the budget of 100 ms on a
/// Raspberry Pi 5 (`FORZAC_SEARCH_BUDGET_MS`, by default 100).
#[test]
fn whole_book_search_is_fast() {
    let Some(books) = corpus() else { return };
    let budget: u128 = std::env::var("FORZAC_SEARCH_BUDGET_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(100);
    let mut slowest = (0, String::new(), 0, 0);
    for path in &books {
        let Ok(book) = forzac_core::open(path) else { continue };
        let text: usize = book.chapters.iter().flat_map(|c| &c.blocks).map(Block::text_len).sum();
        for word in ["что", "the"] {
            let t = std::time::Instant::now();
            let hits = forzac_core::search::search(&book, word).len();
            let ms = t.elapsed().as_millis();
            if ms >= slowest.0 {
                slowest = (ms, path.file_name().unwrap().to_string_lossy().into_owned(), text, hits);
            }
        }
    }
    println!("slowest search: {} ms, {} ({} KB of text, {} matches)", slowest.0, slowest.1, slowest.2 / 1024, slowest.3);
    assert!(slowest.0 <= budget, "{} ms over {budget} ms", slowest.0);
}
