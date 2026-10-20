//! Converts a book to the Kindle's MOBI 6 (`send-to-kindle`), to check the file with Calibre's reader or on a Kindle:
//! `cargo run --release -p forzac-core --example mobi -- book.fb2.zip out.mobi`, or `… book.fb2.zip --kindle` to send
//! it to the connected Kindle as "Send to Kindle" does.

use forzac_core::state::Edits;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [book, out] = &args[..] else {
        eprintln!("usage: mobi BOOK (OUT.mobi | --kindle)");
        std::process::exit(2);
    };
    let path = std::path::Path::new(book);
    if out == "--kindle" {
        let kindle = forzac_core::kindle::find().expect("no Kindle connected");
        let sent = forzac_core::kindle::send(path, &Edits::default(), &kindle).expect("cannot send the book");
        println!("{}", sent.display());
        return;
    }
    let book = forzac_core::open(path).unwrap_or_else(|e| panic!("cannot open {book}: {e:?}"));
    std::fs::write(out, forzac_core::kindle::mobi(&book, &Edits::default())).expect("cannot write the file");
}
