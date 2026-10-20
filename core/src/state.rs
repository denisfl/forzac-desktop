//! What Forzac remembers between launches: the reading position of every book (the library's books), the bookmarks, the
//! highlights (quotes) and the reader settings, in one text file at a path the shell chooses.
//!
//! ```text
//! forzac-state 1
//! setting<TAB>theme<TAB>Dark
//! book<TAB>id<TAB>chapter<TAB>block<TAB>offset<TAB>progress<TAB>opened<TAB>path<TAB>title<TAB>authors
//! bookmark<TAB>id<TAB>chapter<TAB>block<TAB>offset
//! quote<TAB>id<TAB>chapter<TAB>block<TAB>offset<TAB>chapter<TAB>block<TAB>offset
//! shelf<TAB>id<TAB>name
//! edit<TAB>id<TAB>title|authors|series|number|genres<TAB>value
//! file<TAB>name<TAB>size<TAB>mtime<TAB>id<TAB>title<TAB>authors<TAB>series<TAB>number
//! ```
//!
//! `file` lines cache what the Books folder's files hold (`library::scan`), keyed by name, size and mtime (nanoseconds);
//! an empty id marks a file that is not a readable book. A `file` line of 0.11 and before (no series) is dropped, so the
//! file is read again.
//!
//! Tabs, line breaks and backslashes inside fields are escaped. Unreadable lines are skipped. Every write re-reads
//! the file and changes only its own entries, then replaces the file atomically, so other processes' writes survive.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{Block, Book, Locator, Series};

const HEADER: &str = "forzac-state 1";

/// A remembered book.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Entry {
    pub id: String,
    pub path: PathBuf,
    pub title: String,
    pub authors: String,
    pub position: Locator,
    /// How far the book has been read, 0 to 1.
    pub progress: f32,
    /// When it was last opened or read, in seconds since the Unix epoch.
    pub opened: u64,
    /// From the Books folder's cache (`library::scan`), not kept in `book` lines.
    pub series: Option<Series>,
    /// The reader's shelves it is on, from `shelf` lines (`library::scan`).
    pub shelves: Vec<String>,
}

/// A book's details as the reader edited them (`metadata-editing`); `None`: the book's own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Edits {
    pub title: Option<String>,
    /// Joined by ", " as `Entry::authors`.
    pub authors: Option<String>,
    /// `Some(None)`: no series.
    pub series: Option<Option<Series>>,
    pub genres: Option<Vec<String>>,
}

impl Edits {
    pub fn is_empty(&self) -> bool {
        *self == Edits::default()
    }

    /// `entry` with these edits.
    pub fn apply(&self, entry: &mut Entry) {
        if let Some(title) = &self.title {
            entry.title.clone_from(title);
        }
        if let Some(authors) = &self.authors {
            entry.authors.clone_from(authors);
        }
        if let Some(series) = &self.series {
            entry.series.clone_from(series);
        }
    }
}

/// What a file in the Books folder holds, as last read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cached {
    pub name: String,
    pub size: u64,
    pub mtime: u64,
    /// Empty when the file is not a readable book.
    pub id: String,
    pub title: String,
    pub authors: String,
    pub series: Option<Series>,
}

#[derive(Debug, Default)]
pub struct Store {
    path: PathBuf,
    saved: Saved,
}

/// What the file holds.
#[derive(Debug, Default)]
struct Saved {
    books: Vec<Entry>,
    settings: BTreeMap<String, String>,
    /// Bookmarks by book id, in reading order.
    bookmarks: BTreeMap<String, Vec<Locator>>,
    /// Highlights by book id: their `[start, end)`, in reading order.
    quotes: BTreeMap<String, Vec<(Locator, Locator)>>,
    /// The Books folder's files, by name.
    files: Vec<Cached>,
    /// The reader's shelves by book id, in the order put on.
    shelves: BTreeMap<String, Vec<String>>,
    /// The reader's edits by book id.
    edits: BTreeMap<String, Edits>,
}

impl Store {
    /// The state at `path`; empty when the file is missing or unreadable.
    pub fn open(path: impl Into<PathBuf>) -> Store {
        let path = path.into();
        let saved = read(&path);
        Store { path, saved }
    }

    /// Reads the file again: another `Store` on the same file (a background import) has written it.
    pub fn reload(&mut self) {
        self.saved = read(&self.path);
    }

    /// The state file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where `book` (opened from `path`) was left: by its content, else by its path; clamped to the book.
    pub fn position(&self, book: &Book, path: &Path) -> Option<Locator> {
        let books = &self.saved.books;
        let entry = books.iter().find(|e| e.id == book.id).or_else(|| books.iter().find(|e| e.path == path))?;
        Some(clamp(book, entry.position))
    }

    /// Remembers that `book`, opened from `path`, is read at `at`; a `finished` book (its end page shown) reads 100 %.
    pub fn save_position(&mut self, book: &Book, path: &Path, at: Locator, finished: bool) -> io::Result<()> {
        let entry = Entry {
            id: book.id.clone(),
            path: path.to_owned(),
            title: book.title.clone(),
            authors: book.authors.join(", "),
            position: at,
            progress: if finished { 1.0 } else { progress(book, at) },
            opened: now(),
            series: book.series.clone(),
            shelves: Vec::new(),
        };
        self.write(|saved| {
            saved.books.retain(|e| e.id != entry.id && e.path != entry.path);
            saved.books.push(entry.clone());
        })
    }

    pub fn setting(&self, key: &str) -> Option<&str> {
        self.saved.settings.get(key).map(String::as_str)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> io::Result<()> {
        self.set_settings(&[(key, value.to_owned())])
    }

    /// Several settings in one write.
    pub fn set_settings(&mut self, values: &[(&str, String)]) -> io::Result<()> {
        self.write(|saved| {
            for (key, value) in values {
                saved.settings.insert((*key).to_owned(), value.clone());
            }
        })
    }

    /// The bookmarks of `book`, in reading order.
    pub fn bookmarks(&self, book: &Book) -> Vec<Locator> {
        self.bookmarks_of(&book.id)
    }

    /// The bookmarks of the book `id`, in reading order.
    pub fn bookmarks_of(&self, id: &str) -> Vec<Locator> {
        self.saved.bookmarks.get(id).cloned().unwrap_or_default()
    }

    /// Replaces the bookmarks of `book` with `marks`.
    pub fn set_bookmarks(&mut self, book: &Book, marks: &[Locator]) -> io::Result<()> {
        self.set_bookmarks_of(&book.id, marks)
    }

    /// Replaces the bookmarks of the book `id` with `marks`.
    pub fn set_bookmarks_of(&mut self, id: &str, marks: &[Locator]) -> io::Result<()> {
        let mut marks = marks.to_vec();
        marks.sort();
        marks.dedup();
        self.write(|saved| {
            if marks.is_empty() {
                saved.bookmarks.remove(id);
            } else {
                saved.bookmarks.insert(id.to_owned(), marks.clone());
            }
        })
    }

    /// The highlights of `book`, in reading order.
    pub fn quotes(&self, book: &Book) -> Vec<(Locator, Locator)> {
        self.quotes_of(&book.id)
    }

    /// The highlights of the book `id`, in reading order.
    pub fn quotes_of(&self, id: &str) -> Vec<(Locator, Locator)> {
        self.saved.quotes.get(id).cloned().unwrap_or_default()
    }

    /// Replaces the highlights of `book` with `quotes`.
    pub fn set_quotes(&mut self, book: &Book, quotes: &[(Locator, Locator)]) -> io::Result<()> {
        self.set_quotes_of(&book.id, quotes)
    }

    /// Replaces the highlights of the book `id` with `quotes`.
    pub fn set_quotes_of(&mut self, id: &str, quotes: &[(Locator, Locator)]) -> io::Result<()> {
        let mut quotes = quotes.to_vec();
        quotes.sort();
        quotes.dedup();
        self.write(|saved| {
            if quotes.is_empty() {
                saved.quotes.remove(id);
            } else {
                saved.quotes.insert(id.to_owned(), quotes.clone());
            }
        })
    }

    /// The ids of the books with bookmarks, highlights, shelves or edits (`sync`).
    pub fn marked_books(&self) -> std::collections::BTreeSet<String> {
        let s = &self.saved;
        s.bookmarks.keys().chain(s.quotes.keys()).chain(s.shelves.keys()).chain(s.edits.keys()).cloned().collect()
    }

    /// Remembers that the book `id` (`book`, from `path`) is read at `at`, as of `opened` (seconds since the Unix epoch):
    /// a position that came from another device (`sync`).
    pub fn set_synced_position(&mut self, book: &Book, path: &Path, at: Locator, opened: u64) -> io::Result<()> {
        let entry = Entry {
            id: book.id.clone(),
            path: path.to_owned(),
            title: book.title.clone(),
            authors: book.authors.join(", "),
            position: at,
            progress: progress(book, at),
            opened,
            series: book.series.clone(),
            shelves: Vec::new(),
        };
        self.write(|saved| {
            saved.books.retain(|e| e.id != entry.id && e.path != entry.path);
            saved.books.push(entry.clone());
        })
    }

    /// The shelves the book `id` is on.
    pub fn shelves_of(&self, id: &str) -> &[String] {
        self.saved.shelves.get(id).map_or(&[], Vec::as_slice)
    }

    /// Every shelf with a book on it, A to Z, each once whatever its case (the first spelling kept).
    pub fn shelf_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for name in self.saved.shelves.values().flatten() {
            if !names.iter().any(|n| n.to_lowercase() == name.to_lowercase()) {
                names.push(name.clone());
            }
        }
        names.sort_by_key(|n| n.to_lowercase().replace('ё', "е"));
        names
    }

    /// Puts the book `id` on the shelf `name` (trimmed; an existing shelf's spelling whatever the case) or takes it
    /// off. An empty name changes nothing.
    pub fn set_shelf(&mut self, id: &str, name: &str, on: bool) -> io::Result<()> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            return Ok(());
        }
        let name = self.shelf_names().into_iter().find(|n| n.to_lowercase() == name.to_lowercase()).unwrap_or(name);
        self.write(|saved| {
            let shelves = saved.shelves.entry(id.to_owned()).or_default();
            shelves.retain(|s| s.to_lowercase() != name.to_lowercase());
            if on {
                shelves.push(name.clone());
            }
            if shelves.is_empty() {
                saved.shelves.remove(id);
            }
        })
    }

    /// The reader's edits of the book `id`.
    pub fn edits_of(&self, id: &str) -> Edits {
        self.saved.edits.get(id).cloned().unwrap_or_default()
    }

    /// Replaces the edits of the book `id`; empty edits drop them.
    pub fn set_edits(&mut self, id: &str, edits: Edits) -> io::Result<()> {
        self.write(|saved| {
            if edits.is_empty() {
                saved.edits.remove(id);
            } else {
                saved.edits.insert(id.to_owned(), edits.clone());
            }
        })
    }

    /// Up to `n` books whose files still exist, the last opened first.
    pub fn recent(&self, n: usize) -> Vec<&Entry> {
        let mut books = self.books();
        books.truncate(n);
        books
    }

    /// The library: every book whose file still exists, the last opened first.
    pub fn books(&self) -> Vec<&Entry> {
        let mut books: Vec<&Entry> = self.saved.books.iter().filter(|e| e.path.is_file()).collect();
        books.sort_by_key(|e| std::cmp::Reverse(e.opened));
        books
    }

    /// Every remembered book, whether its file exists or not, in the file's order.
    pub fn entries(&self) -> &[Entry] {
        &self.saved.books
    }

    /// Points the remembered book `id` at `path` (its copy in the Books folder).
    pub fn set_path(&mut self, id: &str, path: &Path) -> io::Result<()> {
        self.write(|saved| {
            for e in saved.books.iter_mut().filter(|e| e.id == id) {
                e.path = path.to_owned();
            }
        })
    }

    /// The Books folder's files as last read.
    pub fn files(&self) -> &[Cached] {
        &self.saved.files
    }

    /// Replaces what is known of the Books folder's files.
    pub fn set_files(&mut self, files: Vec<Cached>) -> io::Result<()> {
        self.write(|saved| saved.files = files)
    }

    /// Forgets the book `id` (removed from the library): its entry and reading position. Its bookmarks and highlights
    /// stay, so opening the file again brings them back.
    pub fn forget(&mut self, id: &str) -> io::Result<()> {
        self.write(|saved| saved.books.retain(|e| e.id != id))
    }

    /// Renames book ids, old to new, in the entries, bookmarks, highlights and the Books folder's files
    /// (`book-identity`). Data already under a new id is merged; of two entries for one id the later opened stays.
    /// Nothing is written when no old id is in use.
    pub fn rename_ids(&mut self, ids: &HashMap<String, String>) -> io::Result<()> {
        let (s, old) = (&self.saved, |id: &String| ids.contains_key(id));
        let used = s.books.iter().any(|e| old(&e.id))
            || s.bookmarks.keys().any(old)
            || s.quotes.keys().any(old)
            || s.shelves.keys().any(old)
            || s.edits.keys().any(old)
            || s.files.iter().any(|f| old(&f.id));
        if !used {
            return Ok(());
        }
        self.write(|saved| {
            let rename = |id: &mut String| {
                if let Some(new) = ids.get(id) {
                    id.clone_from(new);
                }
            };
            saved.books.iter_mut().for_each(|e| rename(&mut e.id));
            saved.files.iter_mut().for_each(|f| rename(&mut f.id));
            let mut latest: HashMap<String, u64> = HashMap::new();
            for e in &saved.books {
                let t = latest.entry(e.id.clone()).or_default();
                *t = (*t).max(e.opened);
            }
            let mut seen = HashSet::new();
            saved.books.retain(|e| e.opened == latest[&e.id] && seen.insert(e.id.clone()));
            for (old, new) in ids {
                if let Some(marks) = saved.bookmarks.remove(old) {
                    let all = saved.bookmarks.entry(new.clone()).or_default();
                    all.extend(marks);
                    all.sort();
                    all.dedup();
                }
                if let Some(quotes) = saved.quotes.remove(old) {
                    let all = saved.quotes.entry(new.clone()).or_default();
                    all.extend(quotes);
                    all.sort();
                    all.dedup();
                }
                if let Some(edits) = saved.edits.remove(old) {
                    saved.edits.entry(new.clone()).or_insert(edits);
                }
                if let Some(shelves) = saved.shelves.remove(old) {
                    let all = saved.shelves.entry(new.clone()).or_default();
                    all.extend(shelves.into_iter().filter(|s| !all.contains(s)).collect::<Vec<_>>());
                }
            }
        })
    }

    /// Re-reads the file, applies `change`, and replaces the file with the result.
    fn write(&mut self, change: impl FnOnce(&mut Saved)) -> io::Result<()> {
        let mut saved = read(&self.path);
        change(&mut saved);
        let mut text = String::with_capacity(64 + saved.books.len() * 160);
        let _ = writeln!(text, "{HEADER}");
        for (key, value) in &saved.settings {
            let _ = writeln!(text, "setting\t{}\t{}", escape(key), escape(value));
        }
        for e in &saved.books {
            let Locator { chapter, block, offset } = e.position;
            let path = e.path.to_string_lossy();
            let (id, path, title, authors) = (escape(&e.id), escape(&path), escape(&e.title), escape(&e.authors));
            let _ = writeln!(text, "book\t{id}\t{chapter}\t{block}\t{offset}\t{:.4}\t{}\t{path}\t{title}\t{authors}", e.progress, e.opened);
        }
        for (id, marks) in &saved.bookmarks {
            for Locator { chapter, block, offset } in marks {
                let _ = writeln!(text, "bookmark\t{}\t{chapter}\t{block}\t{offset}", escape(id));
            }
        }
        for (id, quotes) in &saved.quotes {
            for (a, b) in quotes {
                let _ = writeln!(
                    text,
                    "quote\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    escape(id),
                    a.chapter,
                    a.block,
                    a.offset,
                    b.chapter,
                    b.block,
                    b.offset
                );
            }
        }
        for (id, shelves) in &saved.shelves {
            for name in shelves {
                let _ = writeln!(text, "shelf\t{}\t{}", escape(id), escape(name));
            }
        }
        for (id, e) in &saved.edits {
            let id = escape(id);
            let mut line = |field: &str, value: &str| {
                let _ = writeln!(text, "edit\t{id}\t{field}\t{}", escape(value));
            };
            if let Some(title) = &e.title {
                line("title", title);
            }
            if let Some(authors) = &e.authors {
                line("authors", authors);
            }
            if let Some(series) = &e.series {
                line("series", series.as_ref().map_or("", |s| s.name.as_str()));
                if let Some(n) = series.as_ref().and_then(|s| s.number) {
                    line("number", &n.to_string());
                }
            }
            if let Some(genres) = &e.genres {
                line("genres", &genres.join("\n"));
            }
        }
        for f in &saved.files {
            let (name, id, title, authors) = (escape(&f.name), escape(&f.id), escape(&f.title), escape(&f.authors));
            let series = f.series.as_ref().map(|s| escape(&s.name).into_owned()).unwrap_or_default();
            let number = f.series.as_ref().and_then(|s| s.number).map(|n| n.to_string()).unwrap_or_default();
            let _ = writeln!(text, "file\t{name}\t{}\t{}\t{id}\t{title}\t{authors}\t{series}\t{number}", f.size, f.mtime);
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // A name of its own per write: a background import writes the same file from another thread.
        static WRITES: AtomicUsize = AtomicUsize::new(0);
        let name = self.path.file_name().unwrap_or_default().to_string_lossy();
        let tmp = self.path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), WRITES.fetch_add(1, Ordering::Relaxed)));
        let written = std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &self.path));
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        written?;
        self.saved = saved;
        Ok(())
    }
}

fn read(path: &Path) -> Saved {
    let mut saved = Saved::default();
    let Ok(text) = std::fs::read_to_string(path) else { return saved };
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return saved;
    }
    for line in lines {
        let f: Vec<String> = line.split('\t').map(unescape).collect();
        let num = |i: usize| f[i].parse::<usize>().ok();
        match f.first().map(String::as_str) {
            Some("setting") if f.len() == 3 => {
                saved.settings.insert(f[1].clone(), f[2].clone());
            }
            Some("bookmark") if f.len() == 5 => {
                if let (Some(chapter), Some(block), Some(offset)) = (num(2), num(3), num(4)) {
                    let marks = saved.bookmarks.entry(f[1].clone()).or_default();
                    marks.push(Locator { chapter, block, offset });
                    marks.sort();
                    marks.dedup();
                }
            }
            Some("quote") if f.len() == 8 => {
                let place = |i: usize| Some(Locator { chapter: num(i)?, block: num(i + 1)?, offset: num(i + 2)? });
                if let (Some(start), Some(end)) = (place(2), place(5))
                    && start < end
                {
                    let quotes = saved.quotes.entry(f[1].clone()).or_default();
                    quotes.push((start, end));
                    quotes.sort();
                    quotes.dedup();
                }
            }
            Some("book") if f.len() == 10 => {
                let (Some(chapter), Some(block), Some(offset), Ok(progress), Ok(opened)) =
                    (num(2), num(3), num(4), f[5].parse::<f32>(), f[6].parse::<u64>())
                else {
                    continue;
                };
                saved.books.push(Entry {
                    id: f[1].clone(),
                    path: PathBuf::from(&f[7]),
                    title: f[8].clone(),
                    authors: f[9].clone(),
                    position: Locator { chapter, block, offset },
                    progress,
                    opened,
                    series: None,
                    shelves: Vec::new(),
                });
            }
            Some("edit") if f.len() == 4 => {
                let e = saved.edits.entry(f[1].clone()).or_default();
                match f[2].as_str() {
                    "title" => e.title = Some(f[3].clone()),
                    "authors" => e.authors = Some(f[3].clone()),
                    "series" => e.series = Some(Series::parse(&f[3], None)),
                    // After its `series` line.
                    "number" => {
                        if let Some(Some(s)) = &mut e.series {
                            s.number = f[3].parse().ok();
                        }
                    }
                    "genres" => e.genres = Some(f[3].split('\n').filter(|g| !g.is_empty()).map(str::to_owned).collect()),
                    _ => {}
                }
            }
            Some("shelf") if f.len() == 3 && !f[2].trim().is_empty() => {
                let shelves = saved.shelves.entry(f[1].clone()).or_default();
                if !shelves.contains(&f[2]) {
                    shelves.push(f[2].clone());
                }
            }
            Some("file") if f.len() == 9 => {
                let (Ok(size), Ok(mtime)) = (f[2].parse::<u64>(), f[3].parse::<u64>()) else { continue };
                saved.files.retain(|c| c.name != f[1]);
                let (id, title, authors) = (f[4].clone(), f[5].clone(), f[6].clone());
                let series = Series::parse(&f[7], Some(&f[8]));
                saved.files.push(Cached { name: f[1].clone(), size, mtime, id, title, authors, series });
            }
            _ => {} // unknown or damaged: dropped at the next write
        }
    }
    saved
}

pub(crate) fn escape(s: &str) -> Cow<'_, str> {
    if !s.contains(['\\', '\t', '\n', '\r']) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', ""))
}

pub(crate) fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The identity of a book file, the Android app's (`BookImport.idOf`): the first 16 bytes of the SHA-256 of the whole
/// file in lowercase hex, so the same file is the same book on both.
pub fn book_id(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// The id of 0.11.0 and before: FNV-1a 64 over the file's length, first 64 KiB and last 64 KiB. Only for moving old
/// states to [`book_id`] (`library::books_folder`).
pub fn legacy_id(bytes: &[u8]) -> String {
    const PART: usize = 64 * 1024;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |data: &[u8]| {
        for &b in data {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(&(bytes.len() as u64).to_le_bytes());
    feed(&bytes[..bytes.len().min(PART)]);
    feed(&bytes[bytes.len().saturating_sub(PART)..]);
    format!("{hash:016x}")
}

/// `at` moved inside the book: a position from a changed file must not point past its end.
pub fn clamp(book: &Book, at: Locator) -> Locator {
    let Some(last) = book.chapters.len().checked_sub(1) else { return Locator::default() };
    let chapter = at.chapter.min(last);
    let blocks = book.chapters[chapter].blocks.len();
    if at.chapter > last || at.block >= blocks {
        return Locator { chapter, block: if at.chapter > last { 0 } else { blocks.saturating_sub(1) }, offset: 0 };
    }
    at
}

/// The share of the book's text before each of `places`, in one pass over the book (for the contents).
pub fn progress_of(book: &Book, places: &[Locator]) -> Vec<f32> {
    // Text before each block: block starts per chapter, then the total.
    let mut starts: Vec<Vec<usize>> = Vec::with_capacity(book.chapters.len());
    let mut total = 0;
    for chapter in &book.chapters {
        starts.push(
            chapter
                .blocks
                .iter()
                .map(|b| {
                    let start = total;
                    total += b.text_len();
                    start
                })
                .collect(),
        );
    }
    places
        .iter()
        .map(|at| {
            let chapter = starts.get(at.chapter);
            let before = chapter.and_then(|c| c.get(at.block).copied()).or_else(|| chapter.and_then(|c| c.last().copied())).unwrap_or(0);
            if total == 0 { 0.0 } else { before as f32 / total as f32 }
        })
        .collect()
}

/// The share of the book's text before `at`.
pub fn progress(book: &Book, at: Locator) -> f32 {
    let length = Block::text_len;
    let mut before = 0;
    let mut total = 0;
    for (c, chapter) in book.chapters.iter().enumerate() {
        for (i, block) in chapter.blocks.iter().enumerate() {
            let len = length(block);
            if (c, i) < (at.chapter, at.block) {
                before += len;
            } else if (c, i) == (at.chapter, at.block) {
                before += at.offset.min(len);
            }
            total += len;
        }
    }
    if total == 0 { 0.0 } else { before as f32 / total as f32 }
}

#[cfg(test)]
mod tests;
