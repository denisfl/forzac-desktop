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
    let failures: Vec<String> = books
        .iter()
        .filter_map(|path| match forzac_core::open(path) {
            Ok(book) if !book.chapters.is_empty() => None,
            Ok(_) => Some(format!("{}: no chapters", path.display())),
            Err(e) => Some(format!("{}: {e:?}", path.display())),
        })
        .collect();
    eprintln!("{} FB2 books, {} failed", books.len(), failures.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
