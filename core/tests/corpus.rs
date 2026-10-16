//! Parses every FB2 book in a local corpus: `FORZAC_CORPUS=~/Books cargo test -p forzac-core --test corpus`.
//! Books are never committed; without `FORZAC_CORPUS` the test is skipped.

use std::path::PathBuf;

/// Books with one of `extensions` under `FORZAC_CORPUS` (not recursive), or `None` when the variable is unset.
fn corpus(extensions: &[&str]) -> Option<Vec<PathBuf>> {
    let dir = std::env::var_os("FORZAC_CORPUS")?;
    let mut books: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("FORZAC_CORPUS is not a readable directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            extensions.iter().any(|ext| name.ends_with(ext))
        })
        .collect();
    books.sort();
    Some(books)
}

#[test]
fn every_fb2_book_parses() {
    let Some(books) = corpus(&[".fb2", ".fb2.zip"]) else {
        eprintln!("skipped: set FORZAC_CORPUS to a directory of books");
        return;
    };
    let mut failures = Vec::new();
    let (mut notes, mut images, mut image_blocks, mut links, mut tables, mut chapters) = (0, 0, 0, 0, 0, 0);
    let mut drawn = Drawn::default();
    for path in &books {
        match forzac_core::open(path) {
            Ok(book) if !book.chapters.is_empty() => {
                typeset_all(&book, &mut drawn);
                chapters += book.chapters.len();
                notes += book.notes.len();
                images += book.images.len();
                links += book.links.len();
                image_blocks += book
                    .chapters
                    .iter()
                    .flat_map(|c| &c.blocks)
                    .filter(|b| matches!(b, forzac_core::Block::Image { id } if book.images.contains_key(id)))
                    .count();
                tables += book
                    .chapters
                    .iter()
                    .flat_map(|c| &c.blocks)
                    .filter(|b| matches!(b, forzac_core::Block::TableRow { row: 0, .. }))
                    .count();
            }
            Ok(_) => failures.push(format!("{}: no chapters", path.display())),
            Err(e) => failures.push(format!("{}: {e:?}", path.display())),
        }
    }
    eprintln!(
        "{} FB2 books, {} failed; {chapters} chapters, {notes} notes, {images} images ({image_blocks} in the text), {links} links, {tables} tables; drawn: {:?}",
        books.len(),
        failures.len(),
        drawn
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// What the pages of the corpus hold.
#[derive(Debug, Default)]
struct Drawn {
    pages: usize,
    images: usize,
    rules: usize,
    areas: usize,
    /// Glyphs no font had (drawn as a box); without a fallback provider these are the characters PT Serif lacks.
    missing_glyphs: usize,
}

/// Lays out every chapter and renders every page: the whole chain must hold for real books.
fn typeset_all(book: &forzac_core::Book, drawn: &mut Drawn) {
    let style = forzac_core::PageStyle::for_window(760.0, 980.0, 20.0);
    let hyphenator = forzac_core::hyphen::Hyphenator::for_language(book.language.as_deref()).map(std::sync::Arc::new);
    let mut typesetter = forzac_core::Typesetter::new(style, hyphenator);
    for chapter in 0..book.chapters.len() {
        let layout = forzac_core::paginate(book, chapter, &style, &mut typesetter);
        for page in 0..layout.pages.len() {
            let content = layout.render(page, &style, &mut typesetter);
            assert!(!content.runs.is_empty(), "every page has at least its number");
            drawn.images += content.images.len();
            drawn.rules += content.rules.len();
            drawn.areas += content.areas.len();
            drawn.missing_glyphs += content.runs.iter().flat_map(|r| &r.glyphs).filter(|&&g| g == 0).count();
        }
        drawn.pages += layout.pages.len();
    }
}
