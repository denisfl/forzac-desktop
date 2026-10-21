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

#[test]
fn a_clock_ahead_does_not_win_over_later_changes() {
    let (folder, path, _, mut b, book) = setup();
    // Another device whose clock is an hour ahead: a place and a bookmark.
    let ahead = now() + 3_600_000;
    let mut text = format!("{HEADER}\ndevice\tffffffffffffffff\tAhead\n");
    let place = place_of(&book, at(1, 1, 0));
    let mark = place_of(&book, at(0, 1, 0));
    write_record(
        &mut text,
        &Record { book: book.id.clone(), time: ahead, deleted: false, item: Item::Position { place, progress: 0.5 }, local: None },
        false,
    );
    write_record(&mut text, &Record { book: book.id.clone(), time: ahead, deleted: false, item: Item::Bookmark(mark), local: None }, false);
    std::fs::create_dir_all(folder.join(DIR)).unwrap();
    std::fs::write(folder.join(DIR).join("ffffffffffffffff.sync"), text).unwrap();
    // B has the book open somewhere else; the sync takes the other place and says so.
    b.save_position(&book, &path, at(0, 2, 0), false).unwrap();
    assert_eq!(sync(&folder, &mut b).unwrap(), std::slice::from_ref(&book.id));
    assert_eq!(b.position(&book, &path), Some(at(1, 1, 0)));
    assert_eq!(b.bookmarks(&book), [at(0, 1, 0)]);

    // B reads on and removes the bookmark with its own (earlier) clock: both are later on every device.
    b.save_position(&book, &path, at(1, 2, 10), false).unwrap();
    b.set_bookmarks(&book, &[]).unwrap();
    assert!(sync(&folder, &mut b).unwrap().is_empty());
    let mut c = Store::open(folder.parent().unwrap().join("c.state"));
    sync(&folder, &mut c).unwrap();
    assert_eq!(c.position(&book, &path), Some(at(1, 2, 10)));
    assert!(c.bookmarks(&book).is_empty());
}

#[test]
fn a_copied_state_takes_a_new_id() {
    let (folder, _, mut a, _, book) = setup();
    a.set_bookmarks(&book, &[at(0, 1, 0)]).unwrap();
    sync(&folder, &mut a).unwrap();
    // A second computer with a copy of A's state: same id, same own file.
    let copy = folder.parent().unwrap().join("copy.state");
    std::fs::copy(a.path(), &copy).unwrap();
    let mut a2 = Store::open(&copy);
    assert_eq!(a2.setting(DEVICE), a.setting(DEVICE));
    a2.set_bookmarks(&book, &[at(0, 1, 0), at(1, 2, 0)]).unwrap();
    sync(&folder, &mut a2).unwrap();
    // A finds a text it did not write in its file: a new id, and the copy's file is another device's.
    sync(&folder, &mut a).unwrap();
    assert_ne!(a.setting(DEVICE), a2.setting(DEVICE));
    assert_eq!(a.bookmarks(&book), [at(0, 1, 0), at(1, 2, 0)]);
    a.set_bookmarks(&book, &[at(1, 2, 0)]).unwrap();
    sync(&folder, &mut a).unwrap();
    sync(&folder, &mut a2).unwrap();
    assert_eq!(a2.bookmarks(&book), [at(1, 2, 0)]);
    let files = std::fs::read_dir(folder.join(DIR))
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "sync"))
        .count();
    assert_eq!(files, 2, "one file per computer");
}

#[test]
fn opening_a_book_does_not_beat_a_later_reading_elsewhere() {
    let (folder, path, mut a, mut b, book) = setup();
    b.save_position(&book, &path, at(0, 1, 0), false).unwrap();
    sync(&folder, &mut b).unwrap();
    // A reads on (the desktop keeps a place's time in seconds); B opens the book (its place saved again, unmoved) only
    // after A's reading, then syncs.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    a.save_position(&book, &path, at(1, 2, 0), false).unwrap();
    sync(&folder, &mut a).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    b.save_position(&book, &path, at(0, 1, 0), false).unwrap();
    assert_eq!(sync(&folder, &mut b).unwrap(), std::slice::from_ref(&book.id));
    assert_eq!(b.position(&book, &path), Some(at(1, 2, 0)));
    sync(&folder, &mut a).unwrap();
    assert_eq!(a.position(&book, &path), Some(at(1, 2, 0)), "B's old place does not come back");
}

#[test]
fn a_highlight_that_cannot_be_shown_is_not_removed() {
    let (folder, _, _, mut b, book) = setup();
    // A highlight from another app whose places are nowhere in this book's text: both fall back to one block start.
    let lost = Place { chapter: 0, block: 1, offset: 0, fragment: "нет такого".into() };
    let mut text = format!("{HEADER}\ndevice\tffffffffffffffff\tOther\n");
    let item = Item::Highlight(lost.clone(), Place { offset: 5, ..lost });
    write_record(&mut text, &Record { book: book.id.clone(), time: 5, deleted: false, item, local: None }, false);
    std::fs::create_dir_all(folder.join(DIR)).unwrap();
    std::fs::write(folder.join(DIR).join("ffffffffffffffff.sync"), text).unwrap();
    sync(&folder, &mut b).unwrap();
    sync(&folder, &mut b).unwrap();
    assert!(b.quotes(&book).is_empty());
    let own =
        std::fs::read_dir(folder.join(DIR)).unwrap().map(|e| e.unwrap().path()).find(|p| !p.ends_with("ffffffffffffffff.sync")).unwrap();
    let records = read(&own);
    assert!(records.iter().any(|r| matches!(r.item, Item::Highlight(..)) && !r.deleted), "kept, not a removal");
}

#[test]
fn an_unreadable_own_file_stops_the_sync() {
    let (folder, _, mut a, _, book) = setup();
    a.set_bookmarks(&book, &[at(0, 1, 0)]).unwrap();
    sync(&folder, &mut a).unwrap();
    let own = folder.join(DIR).join(format!("{}.sync", a.setting(DEVICE).unwrap()));
    std::fs::remove_file(&own).unwrap();
    std::fs::create_dir(&own).unwrap(); // not readable as a file
    assert!(sync(&folder, &mut a).is_err());
    assert!(own.is_dir(), "not replaced");
}

#[test]
fn a_window_saves_only_what_it_changed() {
    let (_, _, mut a, _, book) = setup();
    // `other` stands for a sync or another window writing the same state file.
    let mut other = Store::open(a.path());
    let base = a.bookmarks(&book);
    other.change_bookmarks_of(&book.id, &[], &[at(1, 1, 0)]).unwrap();
    let held = a.change_bookmarks_of(&book.id, &base, &[at(0, 1, 0)]).unwrap();
    assert_eq!(held, [at(0, 1, 0), at(1, 1, 0)], "the other change is kept");
    let held = a.change_bookmarks_of(&book.id, &held, &[at(1, 1, 0)]).unwrap();
    assert_eq!(held, [at(1, 1, 0)]);
}

#[test]
fn a_synced_place_does_not_overwrite_a_page_turned_meanwhile() {
    let (_, path, mut a, _, book) = setup();
    a.save_position(&book, &path, at(0, 1, 0), false).unwrap();
    // The sync began at (0, 1, 0); the reader has turned a page since.
    a.save_position(&book, &path, at(0, 2, 0), false).unwrap();
    assert!(!a.set_synced_position(&book, &path, at(1, 2, 0), 0.5, 1, Some(at(0, 1, 0))).unwrap());
    assert_eq!(a.position(&book, &path), Some(at(0, 2, 0)));
    assert!(a.set_synced_position(&book, &path, at(1, 2, 0), 0.5, 1, Some(at(0, 2, 0))).unwrap());
    assert_eq!(a.position(&book, &path), Some(at(1, 2, 0)));
}

#[test]
fn times_in_words() {
    crate::i18n::set_language(crate::i18n::Language::English);
    // 2026-10-21 10:14 UTC, and three hours east of it.
    let now = 1_792_577_640_000;
    let hour = 3_600_000;
    assert_eq!(when(now, now, 0), "today at 10:14");
    assert_eq!(when(now - 12 * hour, now, 0), "yesterday at 22:14");
    assert_eq!(when(now - 12 * hour, now, 3 * 3600), "today at 01:14");
    assert_eq!(when(now - 10 * 24 * hour, now, 0), "11 October at 10:14");
    assert_eq!(when(now - 300 * 24 * hour, now, 0), "25 December 2025 at 10:14");
    assert_eq!(civil(0), (1970, 1, 1));
    assert_eq!(civil(-1), (1969, 12, 31));
    assert_eq!(civil(20_513), (2026, 3, 1));
    assert_eq!(civil(20_512), (2026, 2, 28));
}

#[test]
fn the_state_line() {
    crate::i18n::set_language(crate::i18n::Language::English);
    let now = 1_792_577_640_000;
    assert_eq!(state_line(None, now, 0), "Not synced yet");
    assert_eq!(state_line(Some((now, None)), now, 0), "Synced today at 10:14");
    assert_eq!(state_line(Some((now, Some("Permission denied".into()))), now, 0), "Couldn't sync: Permission denied");
}

#[test]
fn the_other_devices_by_their_files() {
    let folder = dir();
    let sync = folder.join(DIR);
    std::fs::create_dir_all(&sync).unwrap();
    std::fs::write(sync.join("aaaaaaaaaaaaaaaa.sync"), format!("{HEADER}\ndevice\taaaaaaaaaaaaaaaa\tMacBook Air\n")).unwrap();
    std::fs::write(sync.join("bbbbbbbbbbbbbbbb.sync"), format!("{HEADER}\ndevice\tbbbbbbbbbbbbbbbb\tNothing\\tPhone\n")).unwrap();
    std::fs::write(sync.join("cccccccccccccccc.sync"), format!("{HEADER}\ndevice\tcccccccccccccccc\t\n")).unwrap();
    std::fs::write(sync.join("dddddddddddddddd.sync"), "not a sync file\n").unwrap();
    std::fs::write(sync.join(".eeeeeeeeeeeeeeee.sync.part"), format!("{HEADER}\ndevice\te\tPart\n")).unwrap();
    // The phone wrote last.
    let file = std::fs::File::options().write(true).open(sync.join("bbbbbbbbbbbbbbbb.sync")).unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(60)).unwrap();
    let found = devices(&folder, Some("cccccccccccccccc"));
    let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Nothing\tPhone", "MacBook Air"]);
    assert_eq!(devices(&folder, None).len(), 3);
    assert!(devices(&folder.join("missing"), None).is_empty());
}
