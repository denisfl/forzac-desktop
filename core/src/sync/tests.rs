use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::model::{Block, Chapter, Kind, StyledText};

fn dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("forzac-sync-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An FB2 of two chapters of three paragraphs.
fn fb2() -> String {
    let chapter = |n: u32| {
        let p: String = (1..=3).map(|i| format!("<p>Глава {n}, абзац {i}: ёлки и палки, текст для поиска {n}{i}.</p>")).collect();
        format!("<section><title><p>Глава {n}</p></title>{p}</section>")
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0"><description><title-info>
        <book-title>Книга</book-title><lang>ru</lang></title-info></description><body>{}{}</body></FictionBook>"#,
        chapter(1),
        chapter(2)
    )
}

/// A Books folder with the book, and two devices' states beside it.
fn setup() -> (PathBuf, PathBuf, Store, Store, Book) {
    let root = dir();
    let folder = root.join("Books");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("book.fb2");
    std::fs::write(&path, fb2()).unwrap();
    let book = crate::open(&path).unwrap();
    let (a, b) = (Store::open(root.join("a.state")), Store::open(root.join("b.state")));
    (folder, path, a, b, book)
}

fn at(chapter: usize, block: usize, offset: usize) -> Locator {
    Locator { chapter, block, offset }
}

#[test]
fn places_travel_as_code_points_with_a_fragment() {
    let (_, _, _, _, book) = setup();
    let text = block_text(&book, 1, 2).unwrap();
    let byte = text.find("абзац").unwrap();
    let place = place_of(&book, at(1, 2, byte));
    assert_eq!(place.offset, text[..byte].chars().count(), "code points, not bytes");
    assert_eq!(place.fragment, text[byte..].chars().take(24).collect::<String>());
    assert_eq!(locate(&book, &place), at(1, 2, byte));
    // Another app numbered the blocks one further: the fragment finds the paragraph.
    let shifted = Place { block: 3, ..place.clone() };
    assert_eq!(locate(&book, &shifted), at(1, 2, byte));
    // A fragment that is nowhere: the start of that block, clamped.
    let lost = Place { chapter: 9, block: 9, offset: 3, fragment: "нет такого".into() };
    assert_eq!(locate(&book, &lost).chapter, book.chapters.len() - 1);
}

#[test]
fn every_kind_of_record_survives_its_file() {
    let records = [
        Record {
            book: "b".into(),
            time: 5,
            deleted: false,
            item: Item::Position { place: Place { chapter: 1, block: 2, offset: 3, fragment: "a\tb".into() }, progress: 0.25 },
            local: Some("1:2:6".into()),
        },
        Record { book: "b".into(), time: 6, deleted: true, item: Item::Bookmark(Place::default()), local: None },
        Record {
            book: "b".into(),
            time: 7,
            deleted: false,
            item: Item::Highlight(Place::default(), Place { offset: 4, ..Place::default() }),
            local: Some("0:0:0-0:0:4".into()),
        },
        Record { book: "b".into(), time: 8, deleted: false, item: Item::Shelf("Летом".into()), local: None },
        Record {
            book: "b".into(),
            time: 9,
            deleted: false,
            item: Item::Edit(Edits {
                title: Some("Т".into()),
                authors: None,
                series: Some(None),
                genres: Some(vec!["A".into(), "B".into()]),
            }),
            local: None,
        },
    ];
    for r in &records {
        let mut line = String::new();
        write_record(&mut line, r, true);
        assert_eq!(parse_record(line.trim_end_matches('\n')).as_ref(), Some(r), "{line}");
        let mut shared = String::new();
        write_record(&mut shared, r, false);
        assert_eq!(parse_record(shared.trim_end_matches('\n')).unwrap().local, None, "the local place stays in the own file");
    }
}

#[test]
fn two_devices_share_their_reading() {
    let (folder, path, mut a, mut b, book) = setup();
    let place = at(1, 2, 10);
    a.save_position(&book, &path, place, false).unwrap();
    a.set_bookmarks(&book, &[at(0, 1, 0)]).unwrap();
    a.set_quotes(&book, &[(at(0, 2, 0), at(0, 2, 8))]).unwrap();
    a.set_shelf(&book.id, "Летом", true).unwrap();
    a.set_edits(&book.id, Edits { title: Some("Своя".into()), ..Edits::default() }).unwrap();
    sync(&folder, &mut a).unwrap();
    sync(&folder, &mut b).unwrap();
    assert_eq!(b.position(&book, &path), Some(place));
    assert_eq!(b.bookmarks(&book), [at(0, 1, 0)]);
    assert_eq!(b.quotes(&book), [(at(0, 2, 0), at(0, 2, 8))]);
    assert_eq!(b.shelves_of(&book.id), ["Летом"]);
    assert_eq!(b.edits_of(&book.id).title.as_deref(), Some("Своя"));
    let files: Vec<String> =
        std::fs::read_dir(folder.join(DIR)).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(files.len(), 2, "one file per device: {files:?}");

    // B removes the bookmark and the shelf; A learns it.
    std::thread::sleep(std::time::Duration::from_millis(5));
    b.set_bookmarks(&book, &[]).unwrap();
    b.set_shelf(&book.id, "Летом", false).unwrap();
    sync(&folder, &mut b).unwrap();
    sync(&folder, &mut a).unwrap();
    assert!(a.bookmarks(&book).is_empty(), "the removal reaches A");
    assert!(a.shelves_of(&book.id).is_empty());
    assert_eq!(a.quotes(&book).len(), 1, "the highlight stays");

    // Nothing new: nothing written.
    let own = std::fs::read_dir(folder.join(DIR))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains(a.setting(DEVICE).unwrap()))
        .unwrap();
    let before = std::fs::metadata(&own).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    sync(&folder, &mut a).unwrap();
    assert_eq!(std::fs::metadata(&own).unwrap().modified().unwrap(), before);
}

#[test]
fn data_waits_for_its_book() {
    let (folder, path, mut a, _, book) = setup();
    a.save_position(&book, &path, at(1, 1, 0), false).unwrap();
    a.set_bookmarks(&book, &[at(1, 2, 0)]).unwrap();
    sync(&folder, &mut a).unwrap();
    // C has the same sync folder but not the book yet.
    let other = folder.parent().unwrap().join("Elsewhere");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::create_dir_all(other.join(DIR)).unwrap();
    for f in std::fs::read_dir(folder.join(DIR)).unwrap() {
        let f = f.unwrap();
        std::fs::copy(f.path(), other.join(DIR).join(f.file_name())).unwrap();
    }
    let mut c = Store::open(folder.parent().unwrap().join("c.state"));
    sync(&other, &mut c).unwrap();
    assert!(c.bookmarks(&book).is_empty(), "no book, nothing applied");
    std::fs::copy(&path, other.join("book.fb2")).unwrap();
    sync(&other, &mut c).unwrap();
    assert_eq!(c.bookmarks(&book), [at(1, 2, 0)], "applied once the book arrived");
    assert_eq!(c.position(&book, &other.join("book.fb2")), Some(at(1, 1, 0)));
}

#[test]
fn a_block_of_a_table_row_counts_its_cells_joined() {
    let row = std::sync::Arc::new(crate::model::Table {
        rows: vec![vec![
            crate::model::TableCell { text: StyledText { text: "А".into(), styles: vec![] }, header: false, col_span: 1 },
            crate::model::TableCell { text: StyledText { text: "Б".into(), styles: vec![] }, header: false, col_span: 1 },
        ]],
    });
    let book = Book {
        chapters: vec![Chapter {
            title: None,
            blocks: vec![
                Block::Paragraph { text: StyledText { text: "x".into(), styles: vec![] }, kind: Kind::Body },
                Block::TableRow { table: row, row: 0 },
            ],
        }],
        ..Book::default()
    };
    assert_eq!(block_text(&book, 0, 1).as_deref(), Some("А\nБ"));
}
