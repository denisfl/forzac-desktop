//! What Forzac remembers between launches: the reading position of every book, the recent books and the reader
//! settings, in one text file at a path the shell chooses.
//!
//! ```text
//! forzac-state 1
//! setting<TAB>theme<TAB>Dark
//! book<TAB>id<TAB>chapter<TAB>block<TAB>offset<TAB>progress<TAB>opened<TAB>path<TAB>title<TAB>authors
//! ```
//!
//! Tabs, line breaks and backslashes inside fields are escaped. Unreadable lines are skipped. Every write re-reads
//! the file and changes only its own entries, then replaces the file atomically, so other processes' writes survive.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{Block, Book, Locator};

const HEADER: &str = "forzac-state 1";

/// A remembered book.
#[derive(Debug, Clone, PartialEq)]
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
}

#[derive(Debug, Default)]
pub struct Store {
    path: PathBuf,
    books: Vec<Entry>,
    settings: BTreeMap<String, String>,
}

impl Store {
    /// The state at `path`; empty when the file is missing or unreadable.
    pub fn open(path: impl Into<PathBuf>) -> Store {
        let path = path.into();
        let (books, settings) = read(&path);
        Store { path, books, settings }
    }

    /// Where `book` (opened from `path`) was left: by its content, else by its path; clamped to the book.
    pub fn position(&self, book: &Book, path: &Path) -> Option<Locator> {
        let entry = self.books.iter().find(|e| e.id == book.id).or_else(|| self.books.iter().find(|e| e.path == path))?;
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
        };
        self.write(|books, _| {
            books.retain(|e| e.id != entry.id && e.path != entry.path);
            books.push(entry.clone());
        })
    }

    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(String::as_str)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> io::Result<()> {
        self.set_settings(&[(key, value.to_owned())])
    }

    /// Several settings in one write.
    pub fn set_settings(&mut self, values: &[(&str, String)]) -> io::Result<()> {
        self.write(|_, settings| {
            for (key, value) in values {
                settings.insert((*key).to_owned(), value.clone());
            }
        })
    }

    /// Up to `n` books whose files still exist, the last opened first.
    pub fn recent(&self, n: usize) -> Vec<&Entry> {
        let mut books: Vec<&Entry> = self.books.iter().filter(|e| e.path.is_file()).collect();
        books.sort_by_key(|e| std::cmp::Reverse(e.opened));
        books.truncate(n);
        books
    }

    /// Re-reads the file, applies `change`, and replaces the file with the result.
    fn write(&mut self, change: impl FnOnce(&mut Vec<Entry>, &mut BTreeMap<String, String>)) -> io::Result<()> {
        let (mut books, mut settings) = read(&self.path);
        change(&mut books, &mut settings);
        let mut text = String::with_capacity(64 + books.len() * 160);
        let _ = writeln!(text, "{HEADER}");
        for (key, value) in &settings {
            let _ = writeln!(text, "setting\t{}\t{}", escape(key), escape(value));
        }
        for e in &books {
            let Locator { chapter, block, offset } = e.position;
            let path = e.path.to_string_lossy();
            let (id, path, title, authors) = (escape(&e.id), escape(&path), escape(&e.title), escape(&e.authors));
            let _ = writeln!(text, "book\t{id}\t{chapter}\t{block}\t{offset}\t{:.4}\t{}\t{path}\t{title}\t{authors}", e.progress, e.opened);
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &self.path)?;
        self.books = books;
        self.settings = settings;
        Ok(())
    }
}

fn read(path: &Path) -> (Vec<Entry>, BTreeMap<String, String>) {
    let mut books = Vec::new();
    let mut settings = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(path) else { return (books, settings) };
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return (books, settings);
    }
    for line in lines {
        let f: Vec<String> = line.split('\t').map(unescape).collect();
        match f.first().map(String::as_str) {
            Some("setting") if f.len() == 3 => {
                settings.insert(f[1].clone(), f[2].clone());
            }
            Some("book") if f.len() == 10 => {
                let num = |i: usize| f[i].parse::<usize>().ok();
                let (Some(chapter), Some(block), Some(offset), Ok(progress), Ok(opened)) =
                    (num(2), num(3), num(4), f[5].parse::<f32>(), f[6].parse::<u64>())
                else {
                    continue;
                };
                books.push(Entry {
                    id: f[1].clone(),
                    path: PathBuf::from(&f[7]),
                    title: f[8].clone(),
                    authors: f[9].clone(),
                    position: Locator { chapter, block, offset },
                    progress,
                    opened,
                });
            }
            _ => {} // unknown or damaged: dropped at the next write
        }
    }
    (books, settings)
}

fn escape(s: &str) -> Cow<'_, str> {
    if !s.contains(['\\', '\t', '\n', '\r']) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(s.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', ""))
}

fn unescape(s: &str) -> String {
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

/// The identity of a book file: FNV-1a 64 over its length, first 64 KiB and last 64 KiB. Cheap for any size and
/// stable across Rust versions.
pub fn book_id(bytes: &[u8]) -> String {
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
