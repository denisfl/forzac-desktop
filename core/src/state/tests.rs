use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::model::{Chapter, Kind, StyledText};

/// A fresh directory for one test's state file.
fn dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("forzac-state-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn book(id: &str, chapters: &[usize]) -> Book {
    let paragraph = Block::Paragraph { text: StyledText { text: "0123456789".into(), styles: vec![] }, kind: Kind::Body };
    Book {
        id: id.into(),
        title: format!("Book\t{id}"),
        authors: vec!["A. Author".into(), "B. Author".into()],
        chapters: chapters.iter().map(|&n| Chapter { title: None, blocks: vec![paragraph.clone(); n] }).collect(),
        ..Book::default()
    }
}

fn at(chapter: usize, block: usize, offset: usize) -> Locator {
    Locator { chapter, block, offset }
}

#[test]
fn positions_and_settings_survive_a_restart() {
    let file = dir().join("state");
    let b = book("aaaa", &[4, 4]);
    let mut store = Store::open(&file);
    assert_eq!(store.position(&b, Path::new("/books/a.fb2")), None);
    store.save_position(&b, Path::new("/books/a.fb2"), at(1, 2, 5)).unwrap();
    store.set_setting("theme", "Dark").unwrap();

    let store = Store::open(&file);
    assert_eq!(store.position(&b, Path::new("/books/a.fb2")), Some(at(1, 2, 5)));
    assert_eq!(store.setting("theme"), Some("Dark"));
    let entry = &store.books[0];
    assert_eq!((entry.title.as_str(), entry.authors.as_str()), ("Book\taaaa", "A. Author, B. Author"), "tabs survive escaping");
    assert!((entry.progress - 0.8125).abs() < 1e-4, "{} ≠ (4·10 + 2·10 + 5) / 80", entry.progress);
}

#[test]
fn a_moved_file_keeps_its_position_and_a_changed_one_falls_back_to_the_path() {
    let file = dir().join("state");
    let mut store = Store::open(&file);
    store.save_position(&book("aaaa", &[3, 3]), Path::new("/old/a.epub"), at(1, 1, 0)).unwrap();
    assert_eq!(store.position(&book("aaaa", &[3, 3]), Path::new("/new/a.epub")), Some(at(1, 1, 0)), "same content elsewhere");
    assert_eq!(store.position(&book("bbbb", &[3, 3]), Path::new("/old/a.epub")), Some(at(1, 1, 0)), "new content, same path");
    assert_eq!(store.position(&book("bbbb", &[3, 3]), Path::new("/other.epub")), None);
}

#[test]
fn positions_are_clamped_to_the_book() {
    let b = book("aaaa", &[3, 2]);
    assert_eq!(clamp(&b, at(1, 1, 7)), at(1, 1, 7));
    assert_eq!(clamp(&b, at(1, 9, 7)), at(1, 1, 0), "past the chapter's end: its last block");
    assert_eq!(clamp(&b, at(5, 2, 3)), at(1, 0, 0), "past the last chapter: its start");
}

#[test]
fn writes_keep_what_another_process_wrote() {
    let file = dir().join("state");
    let mut first = Store::open(&file);
    let mut second = Store::open(&file);
    first.save_position(&book("aaaa", &[2]), Path::new("/a"), at(0, 1, 0)).unwrap();
    second.save_position(&book("bbbb", &[2]), Path::new("/b"), at(0, 0, 3)).unwrap();
    second.set_setting("text-size", "22").unwrap();
    first.set_setting("theme", "Sepia").unwrap();
    let store = Store::open(&file);
    assert_eq!(store.books.len(), 2);
    assert_eq!((store.setting("theme"), store.setting("text-size")), (Some("Sepia"), Some("22")));
}

#[test]
fn damaged_lines_are_skipped() {
    let file = dir().join("state");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let good = "book\tid1\t0\t1\t2\t0.5\t100\t/a\tT\tA";
    std::fs::write(&file, format!("{HEADER}\ngarbage\nbook\tid2\tx\t1\t2\t0.5\t100\t/b\tT\tA\n{good}\nsetting\tonly-key\n")).unwrap();
    let store = Store::open(&file);
    assert_eq!(store.books.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["id1"]);
    assert!(store.settings.is_empty());
    std::fs::write(&file, "not a state file").unwrap();
    assert!(Store::open(&file).books.is_empty());
}

#[test]
fn recent_books_are_the_existing_files_last_opened_first() {
    let d = dir();
    std::fs::create_dir_all(&d).unwrap();
    let (a, b) = (d.join("a.fb2"), d.join("b.fb2"));
    std::fs::write(&a, "x").unwrap();
    std::fs::write(&b, "x").unwrap();
    let file = d.join("state");
    std::fs::write(
        &file,
        format!(
            "{HEADER}\nbook\tA\t0\t0\t0\t0.1\t100\t{}\tA\t\nbook\tB\t0\t0\t0\t0.2\t300\t{}\tB\t\nbook\tC\t0\t0\t0\t0.3\t200\t/gone.fb2\tC\t\n",
            a.display(),
            b.display()
        ),
    )
    .unwrap();
    let store = Store::open(&file);
    assert_eq!(store.recent(10).iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["B", "A"]);
    assert_eq!(store.recent(1).len(), 1);
}

#[test]
fn book_ids_depend_on_content() {
    let a = vec![7u8; 200_000];
    let mut b = a.clone();
    b[199_999] = 8; // the end differs
    let mut c = a.clone();
    c[100_000] = 8; // only the middle differs: the same id, by design
    assert_ne!(book_id(&a), book_id(&b));
    assert_eq!(book_id(&a), book_id(&c));
    assert_ne!(book_id(b"short"), book_id(b"shorter"));
    assert_eq!(book_id(b"short").len(), 16);
}

#[test]
fn a_thousand_books_read_and_write_quickly() {
    let file = dir().join("state");
    let mut store = Store::open(&file);
    store
        .write(|books, _| {
            for i in 0..1000 {
                books.push(Entry {
                    id: format!("{i:016x}"),
                    path: PathBuf::from(format!("/home/reader/Books/Some author/A long book title number {i}.fb2.zip")),
                    title: format!("A long book title number {i}"),
                    authors: "Some Author".into(),
                    position: at(i % 40, i % 300, i),
                    progress: 0.5,
                    opened: i as u64,
                });
            }
        })
        .unwrap();
    let t = std::time::Instant::now();
    let mut store = Store::open(&file);
    let read = t.elapsed();
    store.save_position(&book("aaaa", &[2]), Path::new("/a"), at(0, 1, 0)).unwrap();
    let elapsed = t.elapsed();
    eprintln!("1000 books: read in {read:?}, read and one save in {elapsed:?}");
    assert_eq!(store.books.len(), 1001);
    // The requirement is 10 ms on a Raspberry Pi 5 in a release build; debug builds are many times slower.
    assert!(elapsed.as_millis() < 200, "{elapsed:?}");
}
