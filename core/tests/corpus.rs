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
    let (mut notes, mut images, mut links, mut tables, mut chapters, mut pages) = (0, 0, 0, 0, 0, 0);
    for path in &books {
        match forzac_core::open(path) {
            Ok(book) if !book.chapters.is_empty() => {
                pages += typeset_all(&book);
                chapters += book.chapters.len();
                notes += book.notes.len();
                images += book.images.len();
                links += book.links.len();
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
        "{} FB2 books, {} failed; {chapters} chapters, {pages} pages, {notes} notes, {images} images, {links} links, {tables} tables",
        books.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Lays out every chapter and renders every page: the whole chain must hold for real books. Returns the page count.
fn typeset_all(book: &forzac_core::Book) -> usize {
    let style = forzac_core::PageStyle::for_window(760.0, 980.0, 20.0);
    let hyphenator = forzac_core::hyphen::Hyphenator::for_language(book.language.as_deref()).map(std::rc::Rc::new);
    let mut typesetter = forzac_core::Typesetter::new(style, hyphenator);
    let mut pages = 0;
    for chapter in 0..book.chapters.len() {
        let layout = forzac_core::paginate(book, chapter, &style, &mut typesetter);
        for page in 0..layout.pages.len() {
            assert!(!layout.render(page, &style, &mut typesetter).is_empty());
        }
        pages += layout.pages.len();
    }
    pages
}
