//! Parse a book and typeset every chapter: `cargo run --release -p forzac-core --example bench -- book.fb2.zip`.

use std::sync::Arc;
use std::time::Instant;

use forzac_core::hyphen::Hyphenator;
use forzac_core::{PageStyle, Typesetter, paginate};

fn main() {
    let path = std::env::args().nth(1).expect("usage: bench <book.fb2|.fb2.zip>");
    let t = Instant::now();
    let book = forzac_core::open(path.as_ref()).expect("not a book");
    let parse = t.elapsed();
    let style = PageStyle::for_window(760.0, 980.0, 20.0);
    let t = Instant::now();
    let hyphenator = Hyphenator::for_language(book.language.as_deref()).map(Arc::new);
    let hyph = t.elapsed();
    let mut typesetter = Typesetter::new(style, hyphenator);
    let t = Instant::now();
    paginate(&book, 0, &style, &mut typesetter);
    let first_chapter = t.elapsed();
    let t = Instant::now();
    let mut pages = 0;
    let mut slowest = (std::time::Duration::ZERO, 0, 0);
    for c in 0..book.chapters.len() {
        let started = Instant::now();
        let n = paginate(&book, c, &style, &mut typesetter).pages.len();
        pages += n;
        slowest = slowest.max((started.elapsed(), c, n));
    }
    let all = t.elapsed();
    // Page contents: the slowest page of the slowest chapter.
    let chapter = paginate(&book, slowest.1, &style, &mut typesetter);
    let render = (0..chapter.pages.len())
        .map(|i| {
            let t = Instant::now();
            let content = chapter.render(i, &style, &mut typesetter);
            (t.elapsed(), content.runs.len())
        })
        .max()
        .unwrap_or_default();
    println!(
        "{}: {} chapters, {} pages | parse {:?}, hyphenation patterns {:?}, first chapter {:?} (cold), whole book {:?} (warm cache), slowest chapter {:?} (#{}, {} pages), render page at most {:?} ({} runs)",
        book.title,
        book.chapters.len(),
        pages,
        parse,
        hyph,
        first_chapter,
        all,
        slowest.0,
        slowest.1,
        slowest.2,
        render.0,
        render.1
    );
}
