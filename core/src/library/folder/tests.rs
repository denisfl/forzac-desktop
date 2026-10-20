use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::*;
use crate::Locator;

/// A fresh home for one test, with the state file outside the Books folder.
fn home() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("forzac-folder-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn store(home: &Path) -> Store {
    Store::open(home.join(".state"))
}

/// An FB2 book; `text` makes its content (and so its id) differ.
fn fb2(title: &str, author: &str, text: &str) -> Vec<u8> {
    let author = if author.is_empty() { String::new() } else { format!("<author><first-name>{author}</first-name></author>") };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0"><description><title-info>{author}
<book-title>{title}</book-title><lang>ru</lang></title-info></description><body><section><p>{text}</p></section></body></FictionBook>"#
    )
    .into_bytes()
}

fn write(path: &Path, bytes: &[u8]) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
    path.to_owned()
}

fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(folder).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

fn titles(books: &[Entry]) -> Vec<&str> {
    books.iter().map(|e| e.title.as_str()).collect()
}

// region 1.1 the folder

#[test]
fn the_folder_is_named_by_the_language_and_remembered() {
    let home = home();
    let mut s = store(&home);
    let folder = books_folder(&home, Language::Russian, &mut s).unwrap();
    assert_eq!(folder, home.join("Книги"));
    assert!(folder.is_dir());
    let mut s = store(&home);
    assert_eq!(books_folder(&home, Language::English, &mut s).unwrap(), home.join("Книги"), "the language switched later");
    assert!(!home.join("Books").exists());

    let other = self::home();
    assert_eq!(books_folder(&other, Language::English, &mut store(&other)).unwrap(), other.join("Books"));
}

#[test]
fn an_existing_folder_is_kept_and_a_deleted_one_made_again() {
    let home = home();
    let dune = write(&home.join("Books/dune.fb2"), &fb2("Dune", "Frank Herbert", "spice"));
    write(&home.join("Books/notes.pdf"), b"%PDF-1.4");
    write(&home.join("Books/Comics/x.fb2"), &fb2("X", "", "x"));
    let mut s = store(&home);
    let folder = books_folder(&home, Language::English, &mut s).unwrap();
    assert_eq!(names(&folder), ["Comics", "dune.fb2", "notes.pdf"], "nothing renamed or moved");
    let books = scan(&folder, &mut s);
    assert_eq!(titles(&books), ["Dune"]);
    assert_eq!(books[0].path, dune);
    assert_eq!(books[0].opened, 0, "not started");

    std::fs::remove_dir_all(&folder).unwrap();
    assert_eq!(books_folder(&home, Language::Russian, &mut store(&home)).unwrap(), folder);
    assert!(folder.is_dir());
}

// region 1.2 names

#[test]
fn copies_are_named_author_dash_title() {
    let none = |_: &str| false;
    let tolstoy = ["Лев Толстой".to_owned(), "Другой".to_owned()];
    assert_eq!(file_name("Война и мир", &tolstoy, "x.fb2", "", none), "Лев Толстой — Война и мир.fb2");
    assert_eq!(file_name("Война и мир", &[], "x.FB2.ZIP", "", none), "Война и мир.FB2.ZIP", "no author; .fb2.zip whole");
    assert_eq!(file_name("", &[], "oz.fb2.zip", "", none), "oz.fb2.zip", "neither: the original name");
    assert_eq!(file_name("", &tolstoy, "oz.epub", "", none), "Лев Толстой — oz.epub");
    assert_eq!(file_name("A/B: C\n D", &[], "x.fb3", "", none), "A_B_ C D.fb3", "forbidden characters, whitespace");
    assert_eq!(file_name(".hidden", &[], "x.epub", "", none), "_hidden.epub");
    assert_eq!(file_name("Title", &[], "book.zip", ".fb2.zip", none), "Title.fb2.zip", "the format by content");
    let long = "Ж".repeat(300);
    let name = file_name(&long, &[], "x.fb2", "", none);
    assert_eq!(name, format!("{}.fb2", "Ж".repeat(120)));
    assert!(name.len() + " (99)".len() + ".part".len() < 255, "fits a file name in bytes");
}

#[test]
fn three_books_with_one_name() {
    let mut taken: Vec<String> = Vec::new();
    for _ in 0..3 {
        let name = file_name("Война и мир", &["Лев Толстой".into()], "w.fb2", "", |n| taken.iter().any(|t| t == n));
        taken.push(name);
    }
    assert_eq!(taken, ["Лев Толстой — Война и мир.fb2", "Лев Толстой — Война и мир (2).fb2", "Лев Толстой — Война и мир (3).fb2"]);
}

// region 1.4 the scan

#[test]
fn the_scan_follows_the_folder() {
    let home = home();
    let folder = home.join("Books");
    let mut s = store(&home);
    write(&folder.join("a.fb2"), &fb2("A", "", "a"));
    write(&folder.join("broken.epub"), b"PK\x03\x04 not really");
    write(&folder.join("notes.pdf"), b"%PDF");
    write(&folder.join(".hidden.fb2"), &fb2("H", "", "h"));
    assert_eq!(titles(&scan(&folder, &mut s)), ["A"], "a PDF, a damaged EPUB and a dotfile ignored");
    assert_eq!(store(&home).files().len(), 2, "the damaged file is cached too");

    std::thread::sleep(std::time::Duration::from_millis(5));
    write(&folder.join("b.fb2"), &fb2("B", "", "b"));
    write(&folder.join("a.fb2"), &fb2("A2", "", "a2"));
    let books = scan(&folder, &mut s);
    assert_eq!(titles(&books), ["A2", "B"], "changed and new files read; the newest first");
    std::fs::remove_file(folder.join("b.fb2")).unwrap();
    assert_eq!(titles(&scan(&folder, &mut s)), ["A2"]);
    assert_eq!(store(&home).files().iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["a.fb2", "broken.epub"]);
}

#[test]
fn opened_books_first_and_one_entry_per_id() {
    let home = home();
    let folder = home.join("Books");
    let mut s = store(&home);
    let bytes = fb2("Twice", "", "same");
    write(&folder.join("twice.fb2"), &bytes);
    write(&folder.join("twice (conflicted copy).fb2"), &bytes);
    let read = write(&folder.join("read.fb2"), &fb2("Read", "", "r"));
    let book = crate::parse(&std::fs::read(&read).unwrap()).unwrap();
    s.save_position(&book, &read, Locator { chapter: 0, block: 0, offset: 1 }, false).unwrap();
    let books = scan(&folder, &mut s);
    assert_eq!(titles(&books), ["Read", "Twice"]);
    assert!(books[0].opened > 0 && books[1].opened == 0);

    // Renamed by hand: found by its id, with its position.
    std::fs::rename(&read, folder.join("renamed.fb2")).unwrap();
    let books = scan(&folder, &mut s);
    assert_eq!(books[0].path, folder.join("renamed.fb2"));
    assert!(books[0].opened > 0);
}

#[test]
fn five_hundred_unchanged_files_scan_quickly() {
    let home = home();
    let folder = home.join("Books");
    let mut s = store(&home);
    for i in 0..500 {
        write(&folder.join(format!("{i}.fb2")), &fb2(&format!("Book {i}"), "Author", &i.to_string()));
    }
    assert_eq!(scan(&folder, &mut s).len(), 500);
    let mut s = store(&home);
    let start = Instant::now();
    assert_eq!(scan(&folder, &mut s).len(), 500);
    let took = start.elapsed();
    assert!(took.as_millis() < 50, "500 cached files took {took:?}");
}

// region 1.5 import

#[test]
fn opening_a_book_copies_it_in_once() {
    let home = home();
    let folder = home.join("Books");
    std::fs::create_dir_all(&folder).unwrap();
    let mut s = store(&home);
    let src = write(&home.join("Downloads/oz.fb2"), &fb2("Волшебник", "Баум", "oz"));
    let copy = import(&src, &folder, &mut s).unwrap();
    assert_eq!(copy, folder.join("Баум — Волшебник.fb2"));
    assert_eq!(std::fs::read(&copy).unwrap(), std::fs::read(&src).unwrap());
    assert!(src.is_file(), "the original stays");

    let again = write(&home.join("Desktop/other name.fb2"), &std::fs::read(&src).unwrap());
    assert_eq!(import(&again, &folder, &mut s).unwrap(), copy, "the same content under another name");
    assert_eq!(import(&copy, &folder, &mut s).unwrap(), copy, "a file inside the folder opens in place");
    assert_eq!(names(&folder), ["Баум — Волшебник.fb2"]);
    assert_eq!(scan(&folder, &mut s).len(), 1);
}

#[test]
fn a_failed_copy_leaves_nothing() {
    let home = home();
    let mut s = store(&home);
    let src = write(&home.join("oz.fb2"), &fb2("Oz", "", "oz"));
    let photo = write(&home.join("photo.jpg"), b"\xff\xd8\xff not a book");
    let folder = home.join("Books");
    std::fs::create_dir_all(&folder).unwrap();
    assert_eq!(import(&photo, &folder, &mut s), Err(crate::Error::NotABook));
    assert_eq!(import(&home.join("gone.fb2"), &folder, &mut s), Err(crate::Error::Unavailable));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = import(&src, &folder, &mut s);
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(result, Err(crate::Error::Unavailable));
    }
    assert!(names(&folder).is_empty(), "no copy and no .part left");
    assert!(src.is_file());
}

// region 1.6 the one-time move

#[test]
fn the_library_moves_in_once_with_its_marks() {
    let home = home();
    let mut s = store(&home);
    let at = Locator { chapter: 0, block: 0, offset: 3 };
    let mut books = Vec::new();
    for name in ["a", "b", "c", "gone", "locked"] {
        let path = write(&home.join(format!("Downloads/{name}.fb2")), &fb2(&name.to_uppercase(), "", name));
        let book = crate::parse(&std::fs::read(&path).unwrap()).unwrap();
        s.save_position(&book, &path, at, false).unwrap();
        s.set_quotes(&book, &[(at, Locator { offset: 5, ..at })]).unwrap();
        books.push((path, book));
    }
    std::fs::remove_file(&books[3].0).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&books[4].0, std::fs::Permissions::from_mode(0o000)).unwrap();
    }

    let folder = books_folder(&home, Language::English, &mut s).unwrap();
    assert_eq!(names(&folder), ["A.fb2", "B.fb2", "C.fb2"]);
    let s = store(&home);
    for (_, book) in &books[..3] {
        let copy = folder.join(format!("{}.fb2", book.title));
        assert_eq!(s.position(book, &copy), Some(at));
        assert_eq!(s.quotes(book).len(), 1);
        assert_eq!(s.entries().iter().find(|e| e.id == book.id).unwrap().path, copy);
    }
    assert!(books[0].0.is_file(), "the originals stay");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&books[4].0, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    // Once: a book read from elsewhere later is not moved by the next start.
    let mut s = store(&home);
    books_folder(&home, Language::English, &mut s).unwrap();
    assert_eq!(names(&folder).len(), 3);
}

#[test]
fn the_scan_sees_what_another_handle_wrote() {
    // The macOS shell keeps a second handle for its background queue; the main one saves positions.
    let home = home();
    let folder = home.join("Books");
    let (a, b) = (write(&folder.join("a.fb2"), &fb2("A", "", "a")), write(&folder.join("b.fb2"), &fb2("B", "", "b")));
    let mut worker = store(&home);
    let mut main = store(&home);
    let book = |p: &Path| crate::parse(&std::fs::read(p).unwrap()).unwrap();
    main.save_position(&book(&a), &a, Locator::default(), false).unwrap();
    assert_eq!(titles(&scan(&folder, &mut worker)), ["A", "B"]);
    std::thread::sleep(std::time::Duration::from_millis(1100)); // `opened` is in seconds
    main.save_position(&book(&b), &b, Locator::default(), false).unwrap();
    let books = scan(&folder, &mut worker);
    assert_eq!(titles(&books), ["B", "A"], "the book opened last first");
    assert!(books[0].opened > 0);
}
