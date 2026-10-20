//! Syncing reading data through a folder kept by Dropbox, iCloud Drive, Google Drive or Syncthing (`sync`), in the
//! format of `docs/sync-format.md`, shared with the Android app: one file per device in `.forzac/` of the Books folder,
//! every item with the time of its last change, the latest change winning, removals kept as tombstones.
//!
//! The own file also keeps, after the documented fields, where each item is in this device's coordinates: that is how
//! the state's changes since the last sync are found without a log of them.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{Book, Locator};
use crate::state::{Edits, Store, escape, unescape};

/// The setting that remembers this device's id.
pub const DEVICE: &str = "sync.device";
/// The sync folder inside the Books folder.
pub const DIR: &str = ".forzac";
const HEADER: &str = "forzac-sync 1";
/// Code points of text kept with a place.
const FRAGMENT: usize = 24;

/// A place in a book that both apps can find: chapter, block, an offset in code points and the text from there.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Place {
    pub chapter: usize,
    pub block: usize,
    pub offset: usize,
    pub fragment: String,
}

impl Place {
    fn key(&self) -> String {
        format!("{}.{}.{}", self.chapter, self.block, self.offset)
    }
}

/// A block's text as places count it: a table row's cells joined by line breaks (`Table::cell_starts`).
fn block_text(book: &Book, chapter: usize, block: usize) -> Option<String> {
    let block = book.chapters.get(chapter)?.blocks.get(block)?;
    Some(block.texts().iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join("\n"))
}

/// `at` as a portable place.
pub fn place_of(book: &Book, at: Locator) -> Place {
    let text = block_text(book, at.chapter, at.block).unwrap_or_default();
    let mut byte = at.offset.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    Place {
        chapter: at.chapter,
        block: at.block,
        offset: text[..byte].chars().count(),
        fragment: text[byte..].chars().take(FRAGMENT).collect(),
    }
}

/// Where `place` is in `book`: the block at its indices when its text at the offset starts with the fragment; else the
/// fragment's first occurrence searched outwards from that block; else that block's start, clamped to the book.
pub fn locate(book: &Book, place: &Place) -> Locator {
    let at_char = |text: &str, n: usize| text.char_indices().nth(n).map_or(text.len(), |(i, _)| i);
    if let Some(text) = block_text(book, place.chapter, place.block) {
        let byte = at_char(&text, place.offset);
        if text[byte..].starts_with(&place.fragment) {
            return Locator { chapter: place.chapter, block: place.block, offset: byte };
        }
    }
    if !place.fragment.is_empty() {
        let blocks: Vec<(usize, usize)> =
            book.chapters.iter().enumerate().flat_map(|(c, ch)| (0..ch.blocks.len()).map(move |b| (c, b))).collect();
        let here = blocks.iter().position(|&(c, b)| (c, b) >= (place.chapter, place.block)).unwrap_or(blocks.len());
        let mut order: Vec<usize> = (0..blocks.len()).collect();
        order.sort_by_key(|&i| i.abs_diff(here));
        for i in order {
            let (c, b) = blocks[i];
            if let Some(found) = block_text(book, c, b).and_then(|t| t.find(&place.fragment)) {
                return Locator { chapter: c, block: b, offset: found };
            }
        }
    }
    crate::state::clamp(book, Locator { chapter: place.chapter, block: place.block, offset: 0 })
}

/// What an item says.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Position { place: Place, progress: f32 },
    Bookmark(Place),
    Highlight(Place, Place),
    Shelf(String),
    Edit(Edits),
}

/// An item of a sync file.
#[derive(Debug, Clone, PartialEq)]
struct Record {
    book: String,
    time: u64,
    deleted: bool,
    item: Item,
    /// The own file only: where the item is in this device's coordinates, once it is applied.
    local: Option<String>,
}

impl Record {
    /// The item's key: what a later change of it replaces.
    fn key(&self) -> (u8, String, String) {
        match &self.item {
            Item::Position { .. } => (0, self.book.clone(), String::new()),
            Item::Bookmark(p) => (1, self.book.clone(), p.key()),
            Item::Highlight(a, b) => (2, self.book.clone(), format!("{}-{}", a.key(), b.key())),
            Item::Shelf(name) => (3, self.book.clone(), name.to_lowercase()),
            Item::Edit(_) => (4, self.book.clone(), String::new()),
        }
    }
}

fn local(at: Locator) -> String {
    format!("{}:{}:{}", at.chapter, at.block, at.offset)
}

fn local_pair(a: Locator, b: Locator) -> String {
    format!("{}-{}", local(a), local(b))
}

fn parse_local(s: &str) -> Option<Locator> {
    let mut n = s.split(':').map(|p| p.parse::<usize>().ok());
    Some(Locator { chapter: n.next()??, block: n.next()??, offset: n.next()?? })
}

// region the file

fn place_fields(p: &Place) -> String {
    format!("{}\t{}\t{}\t{}", p.chapter, p.block, p.offset, escape(&p.fragment))
}

fn edit_field(value: &Option<String>) -> String {
    value.as_ref().map_or("-".to_owned(), |v| format!("={}", escape(v)))
}

fn parse_edit_field(s: &str) -> Option<Option<String>> {
    match s {
        "-" => Some(None),
        s => s.strip_prefix('=').map(|v| Some(v.to_owned())),
    }
}

fn write_record(out: &mut String, r: &Record, own: bool) {
    let (book, time, deleted) = (escape(&r.book), r.time, u8::from(r.deleted));
    let _ = match &r.item {
        Item::Position { place, progress } => write!(out, "position\t{book}\t{time}\t{}\t{progress:.4}", place_fields(place)),
        Item::Bookmark(p) => write!(out, "bookmark\t{book}\t{time}\t{deleted}\t{}", place_fields(p)),
        Item::Highlight(a, b) => write!(out, "highlight\t{book}\t{time}\t{deleted}\t{}\t{}", place_fields(a), place_fields(b)),
        Item::Shelf(name) => write!(out, "shelf\t{book}\t{time}\t{deleted}\t{}", escape(name)),
        Item::Edit(e) => {
            let series = e.series.as_ref().map(|s| s.as_ref().map(|s| s.name.clone()).unwrap_or_default());
            let number = e.series.as_ref().map(|s| s.as_ref().and_then(|s| s.number).map(|n| n.to_string()).unwrap_or_default());
            let fields = [e.title.clone(), e.authors.clone(), series, number, e.genres.as_ref().map(|g| g.join(", "))];
            write!(out, "edit\t{book}\t{time}\t{}", fields.iter().map(edit_field).collect::<Vec<_>>().join("\t"))
        }
    };
    if let (true, Some(l)) = (own, &r.local) {
        let _ = write!(out, "\t{l}");
    }
    out.push('\n');
}

fn parse_place(f: &[String]) -> Option<Place> {
    Some(Place {
        chapter: f.first()?.parse().ok()?,
        block: f.get(1)?.parse().ok()?,
        offset: f.get(2)?.parse().ok()?,
        fragment: f.get(3)?.clone(),
    })
}

fn parse_record(line: &str) -> Option<Record> {
    let f: Vec<String> = line.split('\t').map(unescape).collect();
    let (book, time) = (f.get(1)?.clone(), f.get(2)?.parse::<u64>().ok()?);
    let deleted = |i: usize| f.get(i).map(|d| d == "1");
    let (deleted, item, local_at) = match f.first()?.as_str() {
        "position" => (false, Item::Position { place: parse_place(&f[3..])?, progress: f.get(7)?.parse().ok()? }, 8),
        "bookmark" => (deleted(3)?, Item::Bookmark(parse_place(&f[4..])?), 8),
        "highlight" => (deleted(3)?, Item::Highlight(parse_place(&f[4..])?, parse_place(&f[8..])?), 12),
        "shelf" => (deleted(3)?, Item::Shelf(f.get(4)?.clone()).clone(), 5),
        "edit" => {
            let v: Vec<Option<String>> = (3..8).map(|i| parse_edit_field(f.get(i)?)).collect::<Option<_>>()?;
            let series = v[2].clone().map(|name| crate::model::Series::parse(&name, v[3].as_deref()));
            let genres = v[4].clone().map(|g| g.split(", ").filter(|s| !s.is_empty()).map(str::to_owned).collect());
            (false, Item::Edit(Edits { title: v[0].clone(), authors: v[1].clone(), series, genres }), 8)
        }
        _ => return None,
    };
    Some(Record { book, time, deleted, item, local: f.get(local_at).filter(|l| !l.is_empty()).cloned() })
}

/// The records of the sync file at `path`; none when it is missing or not a sync file.
fn read(path: &Path) -> Vec<Record> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Vec::new();
    }
    lines.filter_map(parse_record).collect()
}

fn serialize(device: &str, records: &HashMap<(u8, String, String), Record>) -> String {
    let mut sorted: Vec<&Record> = records.values().collect();
    sorted.sort_by_key(|r| r.key());
    let mut out = format!("{HEADER}\ndevice\t{device}\t{}\n", escape(&device_name()));
    for r in sorted {
        write_record(&mut out, r, true);
    }
    out
}

fn device_name() -> String {
    std::env::var("HOSTNAME").or_else(|_| std::env::var("USER")).unwrap_or_default()
}

// endregion

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// This device's id, made once and remembered in the state.
fn device(store: &mut Store) -> io::Result<String> {
    if let Some(id) = store.setting(DEVICE) {
        return Ok(id.to_owned());
    }
    let seed = format!("{:?}{}{}", SystemTime::now(), std::process::id(), store.path().display());
    let id = crate::state::book_id(seed.as_bytes())[..16].to_owned();
    store.set_setting(DEVICE, &id)?;
    Ok(id)
}

/// Books parsed on demand, by id.
struct Books {
    paths: HashMap<String, PathBuf>,
    parsed: HashMap<String, Option<Book>>,
}

impl Books {
    fn get(&mut self, id: &str) -> Option<&Book> {
        if !self.parsed.contains_key(id) {
            let book = self.paths.get(id).and_then(|p| crate::open(p).ok());
            self.parsed.insert(id.to_owned(), book);
        }
        self.parsed.get(id)?.as_ref()
    }
}

/// Syncs the state with the sync folder of the Books folder `folder`: finds the state's changes since the last sync,
/// takes the other devices' newer items, applies them to the state and writes this device's file. Reads books only
/// when a place must be converted.
pub fn sync(folder: &Path, store: &mut Store) -> io::Result<()> {
    store.reload();
    let device = device(store)?;
    let dir = folder.join(DIR);
    std::fs::create_dir_all(&dir)?;
    let own_path = dir.join(format!("{device}.sync"));
    let before = std::fs::read_to_string(&own_path).unwrap_or_default();
    let mut own: HashMap<_, Record> = read(&own_path).into_iter().map(|r| (r.key(), r)).collect();
    let mut paths = crate::library::folder_books(folder, store);
    for e in store.entries() {
        if !paths.contains_key(&e.id) && e.path.is_file() {
            paths.insert(e.id.clone(), e.path.clone());
        }
    }
    let mut books = Books { paths, parsed: HashMap::new() };
    let now = now();
    local_changes(store, &mut own, &mut books, now);

    // The other devices' newer items; a tombstone keeps the local place of what it removes.
    let mut adopted = Vec::new();
    for entry in std::fs::read_dir(&dir)?.flatten() {
        let path = entry.path();
        if path == own_path || path.extension().is_none_or(|e| e != "sync") {
            continue;
        }
        for mut r in read(&path) {
            let key = r.key();
            if own.get(&key).is_none_or(|o| r.time > o.time) {
                r.local = own.get(&key).and_then(|o| o.local.clone()).filter(|_| r.deleted);
                adopted.push(key.clone());
                own.insert(key, r);
            }
        }
    }
    apply(store, &mut own, &mut books, &adopted)?;

    let text = serialize(&device, &own);
    if text != before {
        let part = dir.join(format!(".{device}.sync.part"));
        std::fs::write(&part, &text)?;
        std::fs::rename(&part, &own_path)?;
    }
    Ok(())
}

/// Records in `own` the state's changes since the own file was written.
fn local_changes(store: &Store, own: &mut HashMap<(u8, String, String), Record>, books: &mut Books, now: u64) {
    // Positions: newer when the book was read after the item's time.
    for e in store.entries().to_vec() {
        let key = (0, e.id.clone(), String::new());
        let time = e.opened.max(1) * 1000;
        let changed = own.get(&key).is_none_or(|o| o.local.as_deref() != Some(local(e.position).as_str()) && time > o.time);
        if changed && let Some(book) = books.get(&e.id) {
            let item = Item::Position { place: place_of(book, e.position), progress: e.progress };
            own.insert(key, Record { book: e.id.clone(), time, deleted: false, item, local: Some(local(e.position)) });
        }
    }
    let marked = store.marked_books();
    let known: HashSet<String> = own.values().map(|r| r.book.clone()).collect();
    for id in marked.iter().chain(known.iter().filter(|id| !marked.contains(*id))) {
        // Bookmarks and highlights: new when their local place is not in the own file; removed when an applied item's
        // local place is no longer in the state.
        let marks: Vec<String> = store.bookmarks_of(id).into_iter().map(local).collect();
        let quotes: Vec<String> = store.quotes_of(id).into_iter().map(|(a, b)| local_pair(a, b)).collect();
        let held = |kind: u8, own: &HashMap<_, Record>| -> Vec<String> {
            own.values().filter(|r| r.key().0 == kind && r.book == *id && !r.deleted).filter_map(|r| r.local.clone()).collect()
        };
        let (have_marks, have_quotes) = (held(1, own), held(2, own));
        for l in marks.iter().filter(|l| !have_marks.contains(l)) {
            if let (Some(at), Some(book)) = (parse_local(l), books.get(id)) {
                let r = Record {
                    book: id.clone(),
                    time: now,
                    deleted: false,
                    item: Item::Bookmark(place_of(book, at)),
                    local: Some(l.clone()),
                };
                own.insert(r.key(), r);
            }
        }
        for l in quotes.iter().filter(|l| !have_quotes.contains(l)) {
            let pair = l.split_once('-').and_then(|(a, b)| Some((parse_local(a)?, parse_local(b)?)));
            if let (Some((a, b)), Some(book)) = (pair, books.get(id)) {
                let item = Item::Highlight(place_of(book, a), place_of(book, b));
                let r = Record { book: id.clone(), time: now, deleted: false, item, local: Some(l.clone()) };
                own.insert(r.key(), r);
            }
        }
        for r in own.values_mut().filter(|r| r.book == *id && !r.deleted) {
            let gone = match (&r.item, &r.local) {
                (Item::Bookmark(_), Some(l)) => !marks.contains(l),
                (Item::Highlight(..), Some(l)) => !quotes.contains(l),
                _ => false,
            };
            if gone {
                r.deleted = true;
                r.time = now;
            }
        }
        // Shelves.
        let shelves: Vec<String> = store.shelves_of(id).to_vec();
        for name in &shelves {
            let key = (3, id.clone(), name.to_lowercase());
            if own.get(&key).is_none_or(|r| r.deleted) {
                own.insert(key, Record { book: id.clone(), time: now, deleted: false, item: Item::Shelf(name.clone()), local: None });
            }
        }
        for r in own.values_mut().filter(|r| r.book == *id && !r.deleted) {
            if let Item::Shelf(name) = &r.item
                && !shelves.iter().any(|s| s.to_lowercase() == name.to_lowercase())
            {
                r.deleted = true;
                r.time = now;
            }
        }
        // Edits.
        let edits = store.edits_of(id);
        let key = (4, id.clone(), String::new());
        let held = own.get(&key).map(|r| if let Item::Edit(e) = &r.item { e.clone() } else { Edits::default() });
        if held.as_ref().map_or(!edits.is_empty(), |h| *h != edits) {
            own.insert(key, Record { book: id.clone(), time: now, deleted: false, item: Item::Edit(edits), local: None });
        }
    }
}

/// Applies to the state the adopted items and the own items that wait for their book.
fn apply(
    store: &mut Store,
    own: &mut HashMap<(u8, String, String), Record>,
    books: &mut Books,
    adopted: &[(u8, String, String)],
) -> io::Result<()> {
    let waiting: Vec<_> = own
        .iter()
        .filter(|(_, r)| {
            !r.deleted && r.local.is_none() && matches!(r.item, Item::Position { .. } | Item::Bookmark(_) | Item::Highlight(..))
        })
        .map(|(k, _)| k.clone())
        .collect();
    let mut keys: Vec<_> = adopted.iter().cloned().chain(waiting).collect();
    keys.sort();
    keys.dedup();
    for key in keys {
        let Some(r) = own.get(&key).cloned() else { continue };
        let id = r.book.clone();
        let local_now = match &r.item {
            Item::Position { place, .. } => {
                let path = books.paths.get(&id).cloned();
                let Some(book) = books.get(&id) else { continue };
                let at = locate(book, place);
                let read_later = store.entries().iter().any(|e| e.id == id && e.opened * 1000 > r.time);
                if !read_later && let Some(path) = path {
                    store.set_synced_position(book, &path, at, r.time / 1000)?;
                }
                Some(local(at))
            }
            Item::Bookmark(place) => {
                let mut marks = store.bookmarks_of(&id);
                if r.deleted {
                    if let Some(at) = r.local.as_deref().and_then(parse_local) {
                        marks.retain(|m| *m != at);
                        store.set_bookmarks_of(&id, &marks)?;
                    }
                    r.local.clone()
                } else {
                    let Some(book) = books.get(&id) else { continue };
                    let at = locate(book, place);
                    marks.push(at);
                    store.set_bookmarks_of(&id, &marks)?;
                    Some(local(at))
                }
            }
            Item::Highlight(a, b) => {
                let mut quotes = store.quotes_of(&id);
                if r.deleted {
                    let pair =
                        r.local.as_deref().and_then(|l| l.split_once('-')).and_then(|(a, b)| Some((parse_local(a)?, parse_local(b)?)));
                    if let Some(pair) = pair {
                        quotes.retain(|q| *q != pair);
                        store.set_quotes_of(&id, &quotes)?;
                    }
                    r.local.clone()
                } else {
                    let Some(book) = books.get(&id) else { continue };
                    let (a, b) = (locate(book, a), locate(book, b));
                    if a < b {
                        quotes.push((a, b));
                        store.set_quotes_of(&id, &quotes)?;
                    }
                    Some(local_pair(a, b))
                }
            }
            Item::Shelf(name) => {
                store.set_shelf(&id, name, !r.deleted)?;
                None
            }
            Item::Edit(edits) => {
                store.set_edits(&id, edits.clone())?;
                None
            }
        };
        if let Some(record) = own.get_mut(&key) {
            record.local = local_now;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
