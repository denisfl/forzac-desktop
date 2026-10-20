//! The Books folder (`books-folder`): where it is, the names of the copies, reading the folder for the library with a
//! cache in the state, copying a book in, and moving the 0.9.0 library in once.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::i18n::Language;
use crate::state::{Cached, Entry, Store, book_id, legacy_id};

/// The setting that remembers the folder; unset until the folder is made and the library moved in.
pub const FOLDER: &str = "books.folder";
/// The setting that marks a state whose book ids are SHA-256 (`book-identity`); unset in states of 0.11.0 and before.
const IDS: &str = "books.ids";
/// The longest name of a copy, in characters, before ` (2)` and the extension.
const NAME_CHARS: usize = 120;
/// Extensions of the books the library reads, longest first: `.fb2.zip` is one extension.
const EXTENSIONS: [&str; 4] = [".fb2.zip", ".fb2", ".fb3", ".epub"];

/// The Books folder: the remembered one, else `~/Книги` (a Russian interface) or `~/Books` in `home`, made when missing.
/// The first time, the remembered books are moved in ([`move_in`]) before the folder is remembered; a state of 0.11.0
/// or before is moved to the new book ids once ([`move_ids`]).
pub fn books_folder(home: &Path, language: Language, store: &mut Store) -> io::Result<PathBuf> {
    if let Some(folder) = store.setting(FOLDER) {
        let folder = PathBuf::from(folder);
        std::fs::create_dir_all(&folder)?;
        move_ids(&folder, store);
        return Ok(folder);
    }
    let folder = home.join(if language == Language::Russian { "Книги" } else { "Books" });
    std::fs::create_dir_all(&folder)?;
    move_ids(&folder, store);
    move_in(&folder, store);
    store.set_setting(FOLDER, &folder.to_string_lossy())?;
    Ok(folder)
}

/// Renames the state's FNV ids of 0.11.0 and before to SHA-256 ids, once: every remembered book and every file of the
/// folder whose file still has the old id is hashed again. Books whose files are gone keep their old ids until the
/// folder scan sees their files again.
fn move_ids(folder: &Path, store: &mut Store) {
    if store.setting(IDS).is_some() {
        return;
    }
    let files = store.files().iter().filter(|f| !f.id.is_empty()).map(|f| (f.id.clone(), folder.join(&f.name)));
    let places: Vec<(String, PathBuf)> = files.chain(store.entries().iter().map(|e| (e.id.clone(), e.path.clone()))).collect();
    let mut ids = HashMap::new();
    for (old, path) in places {
        if ids.contains_key(&old) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&path)
            && legacy_id(&bytes) == old
        {
            ids.insert(old, book_id(&bytes));
        }
    }
    // A cached file that could not be hashed is read again by the next scan, which renames what its old id holds.
    let files: Vec<Cached> = store.files().iter().filter(|f| f.id.is_empty() || ids.contains_key(&f.id)).cloned().collect();
    if files.len() != store.files().len() {
        let _ = store.set_files(files);
    }
    if store.rename_ids(&ids).is_ok() {
        let _ = store.set_setting(IDS, "sha256");
    }
}

/// The remembered folder, when it has been made.
pub fn remembered_folder(store: &Store) -> Option<PathBuf> {
    store.setting(FOLDER).map(PathBuf::from)
}

/// The extension the library reads `name` by (`.fb2.zip` whole), as `name` spells it.
fn extension(name: &str) -> Option<&str> {
    let lower = name.to_lowercase();
    EXTENSIONS.iter().find(|e| lower.ends_with(*e)).map(|e| &name[name.len() - e.len()..])
}

/// A copy's name: `Author — Title.ext` with the first author, the title alone without one, the original name's stem
/// without a title; characters file systems refuse and a leading dot replaced, at most 120 characters, ` (2)`, ` (3)`…
/// while `taken` says the name is in use. The extension is `original`'s, or `fallback` when it is not a book's.
pub fn file_name(title: &str, authors: &[String], original: &str, fallback: &str, taken: impl Fn(&str) -> bool) -> String {
    let ext = extension(original).unwrap_or(fallback);
    let stem = original.strip_suffix(ext).unwrap_or(original);
    let words = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = Some(words(title)).filter(|t| !t.is_empty()).unwrap_or_else(|| words(stem));
    let base = match authors.first().map(|a| words(a)).filter(|a| !a.is_empty()) {
        Some(author) => format!("{author} — {title}"),
        None => title,
    };
    let mut base: String = base.chars().map(|c| if c == '/' || c == ':' || c.is_control() { '_' } else { c }).take(NAME_CHARS).collect();
    base.truncate(base.trim_end().len());
    if base.starts_with('.') {
        base.replace_range(..1, "_");
    }
    if base.is_empty() {
        base.push('_');
    }
    let mut name = format!("{base}{ext}");
    let mut n = 2;
    while taken(&name) {
        name = format!("{base} ({n}){ext}");
        n += 1;
    }
    name
}

/// The extension for a book whose file name does not say its format: by its content.
fn content_extension(bytes: &[u8]) -> &'static str {
    if !bytes.starts_with(b"PK") {
        return ".fb2";
    }
    match crate::archive::unzip(bytes, crate::archive::MAX_UNPACKED) {
        Ok(files) if files.contains_key(crate::epub::CONTAINER) => ".epub",
        Ok(files) if crate::fb3::is_fb3(&files) => ".fb3",
        _ => ".fb2.zip",
    }
}

fn mtime(meta: &std::fs::Metadata) -> u64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as u64)
}

/// What `path` holds: its id, title and authors, and its id of 0.11.0 and before ([`legacy_id`]); an empty id when it
/// is not a readable book.
fn read_file(name: String, meta: &std::fs::Metadata, path: &Path) -> (Cached, Option<String>) {
    let (size, mtime) = (meta.len(), mtime(meta));
    let bytes = std::fs::read(path).ok();
    match bytes.as_deref().and_then(|b| crate::parse(b).ok().map(|book| (book, legacy_id(b)))) {
        Some((b, old)) => {
            (Cached { name, size, mtime, id: b.id, title: b.title, authors: b.authors.join(", "), series: b.series }, Some(old))
        }
        None => (Cached { name, size, mtime, ..Cached::default() }, None),
    }
}

/// Reads the folder's books (FB2, FB2.ZIP, FB3, EPUB at its top level; not dotfiles, not subfolders), parsing only the
/// files that are new or changed since the cache in `store`, which is written once when anything changed. Unreadable
/// files are ignored; files with one id count once. Data a new file's old id still holds (a book put back from the
/// Trash after the move to new ids) is renamed to its new id.
fn folder_files(folder: &Path, store: &mut Store) -> Vec<Cached> {
    let mut files = Vec::new();
    let mut changed = false;
    let mut ids = HashMap::new();
    for item in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let Ok(name) = item.file_name().into_string() else { continue };
        if name.starts_with('.') || extension(&name).is_none() {
            continue;
        }
        let Ok(meta) = item.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let known = store.files().iter().find(|c| c.name == name && c.size == meta.len() && c.mtime == mtime(&meta));
        files.push(match known {
            Some(c) => c.clone(),
            None => {
                changed = true;
                let (file, old) = read_file(name, &meta, &item.path());
                if let Some(old) = old {
                    ids.insert(old, file.id.clone());
                }
                file
            }
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    if changed || files.len() != store.files().len() {
        let _ = store.set_files(files.clone());
    }
    let _ = store.rename_ids(&ids);
    files
}

/// The library: the remembered books whose files exist (a renamed file in the folder found by its id), the last
/// opened first, then the folder's books never opened, the newest file first, as `opened == 0`. One entry per id.
/// Reads the state file first: the handle may be one kept for a background queue while another saves positions.
pub fn scan(folder: &Path, store: &mut Store) -> Vec<Entry> {
    store.reload();
    let files = folder_files(folder, store);
    let in_folder = |id: &str| files.iter().find(|f| f.id == id).map(|f| folder.join(&f.name));
    let mut remembered: Vec<Entry> = store
        .entries()
        .iter()
        .filter_map(|e| {
            let path = if e.path.is_file() { e.path.clone() } else { in_folder(&e.id)? };
            let series = files.iter().find(|f| f.id == e.id).and_then(|f| f.series.clone());
            Some(Entry { path, series, ..e.clone() })
        })
        .collect();
    remembered.sort_by_key(|e| std::cmp::Reverse(e.opened));
    let mut seen: HashSet<String> = HashSet::new();
    remembered.retain(|e| seen.insert(e.id.clone()));
    let mut new: Vec<(&Cached, u64)> =
        files.iter().filter(|f| !f.id.is_empty() && seen.insert(f.id.clone())).map(|f| (f, f.mtime)).collect();
    new.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    remembered.extend(new.into_iter().map(|(f, _)| Entry {
        id: f.id.clone(),
        path: folder.join(&f.name),
        title: f.title.clone(),
        authors: f.authors.clone(),
        series: f.series.clone(),
        ..Entry::default()
    }));
    remembered
}

/// Copies the book at `src` into `folder` and returns the copy's path: `src` itself when it is inside the folder, the
/// copy already there when the folder has the same content (by id). The copy is written to a dotfile and renamed
/// into place, so a half-copied book never shows; `src` is only read.
pub fn import(src: &Path, folder: &Path, store: &mut Store) -> Result<PathBuf, crate::Error> {
    let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_owned());
    if canonical(src).starts_with(canonical(folder)) {
        return Ok(src.to_owned());
    }
    let bytes = std::fs::read(src).map_err(|_| crate::Error::Unavailable)?;
    let id = book_id(&bytes);
    if let Some(copy) = scan(folder, store).into_iter().find(|e| e.id == id && e.path.starts_with(folder)) {
        return Ok(copy.path);
    }
    let copy = copy_in(src, &bytes, id, folder)?;
    let path = folder.join(&copy.name);
    let mut files = store.files().to_vec();
    files.push(copy);
    let _ = store.set_files(files);
    Ok(path)
}

/// Writes `bytes`, read from `src` with the id `id`, into `folder` under its copy's name and returns what the folder's
/// cache keeps of it. The copy is written to a dotfile and renamed into place, so a half-copied book never shows.
fn copy_in(src: &Path, bytes: &[u8], id: String, folder: &Path) -> Result<Cached, crate::Error> {
    let book = crate::parse(bytes)?;
    let original = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let fallback = if extension(&original).is_some() { "" } else { content_extension(bytes) };
    let name = file_name(&book.title, &book.authors, &original, fallback, |n| folder.join(n).exists());
    let (part, path) = (folder.join(format!(".{id}.part")), folder.join(&name));
    let written = std::fs::write(&part, bytes).and_then(|_| std::fs::rename(&part, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
        return Err(crate::Error::Unavailable);
    }
    let (size, mtime) = std::fs::metadata(&path).map(|m| (m.len(), mtime(&m))).unwrap_or_default();
    Ok(Cached { name, size, mtime, id, title: book.title, authors: book.authors.join(", "), series: book.series })
}

/// What adding a folder did ([`import_folder`]).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Added {
    /// Books copied into the Books folder.
    pub added: usize,
    /// Books already in the Books folder, or met twice in the added folder.
    pub known: usize,
    /// Files that could not be read or parsed.
    pub failed: usize,
}

impl Added {
    /// The message for the reader when the import ends, `None` when every book found was added: what failed, or that
    /// nothing was new. `name` is the added folder's name.
    pub fn message(&self, name: &str) -> Option<String> {
        use crate::i18n::{fill, tr, tr_count};
        let Added { added, known, failed } = *self;
        let message = if added + known + failed == 0 {
            fill(tr("No books in “{folder}”: FB2, FB3 or EPUB needed."), &[("folder", name)])
        } else if failed > 0 {
            let added = match added {
                0 => tr("No books were added.").to_owned(),
                n => tr_count(n, "Added {n} book.", "Added {n} books."),
            };
            format!("{added} {}", tr_count(failed, "{n} file couldn't be read.", "{n} files couldn't be read."))
        } else if added == 0 {
            let all = tr_count(
                known,
                "No new books in “{folder}”: the book is already in the library.",
                "No new books in “{folder}”: all {n} are already in the library.",
            );
            fill(&all, &[("folder", name)])
        } else {
            return None;
        };
        Some(message)
    }
}

/// The book files (FB2, FB2.ZIP, FB3, EPUB) in `dir` and its subfolders at any depth, in path order: not dotfiles or
/// dot-folders, not symbolic links to folders, nothing inside `skip` (the Books folder).
pub fn book_files(dir: &Path, skip: &Path) -> Vec<PathBuf> {
    let skip = skip.canonicalize().unwrap_or_else(|_| skip.to_owned());
    let (mut found, mut dirs) = (Vec::new(), vec![dir.to_owned()]);
    while let Some(dir) = dirs.pop() {
        if dir.canonicalize().is_ok_and(|d| d == skip) {
            continue;
        }
        for item in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let Ok(name) = item.file_name().into_string() else { continue };
            let Ok(kind) = item.file_type() else { continue };
            if name.starts_with('.') {
                continue;
            }
            if kind.is_dir() {
                dirs.push(item.path());
            } else if extension(&name).is_some() && item.path().is_file() {
                found.push(item.path());
            }
        }
    }
    found.sort();
    found
}

/// Adds the folder `src`: copies every book in it and its subfolders ([`book_files`]) into `folder` as [`import`]
/// does, skipping books whose content is already there or met before, and remembers the copies in one write.
/// `progress` hears (files done, files found) before each file and at the end. `src` is only read.
pub fn import_folder(src: &Path, folder: &Path, store: &mut Store, mut progress: impl FnMut(usize, usize)) -> Added {
    let files = book_files(src, folder);
    let mut ids: HashSet<String> = folder_files(folder, store).into_iter().map(|f| f.id).filter(|id| !id.is_empty()).collect();
    let (mut copies, mut added) = (Vec::new(), Added::default());
    for (i, path) in files.iter().enumerate() {
        progress(i, files.len());
        let Ok(bytes) = std::fs::read(path) else {
            added.failed += 1;
            continue;
        };
        let id = book_id(&bytes);
        if !ids.insert(id.clone()) {
            added.known += 1;
            continue;
        }
        match copy_in(path, &bytes, id, folder) {
            Ok(copy) => {
                copies.push(copy);
                added.added += 1;
            }
            Err(_) => added.failed += 1,
        }
    }
    progress(files.len(), files.len());
    if !copies.is_empty() {
        let mut all = store.files().to_vec();
        all.extend(copies);
        let _ = store.set_files(all);
    }
    added
}

/// Copies every remembered book whose file exists outside `folder` into it and points its entry at the copy, so its
/// position, bookmarks and highlights carry over; the originals stay. A book that cannot be copied is skipped.
pub fn move_in(folder: &Path, store: &mut Store) {
    let outside: Vec<(String, PathBuf)> = store
        .entries()
        .iter()
        .filter(|e| e.path.is_file() && !e.path.starts_with(folder))
        .map(|e| (e.id.clone(), e.path.clone()))
        .collect();
    for (id, path) in outside {
        if let Ok(copy) = import(&path, folder, store) {
            let _ = store.set_path(&id, &copy);
        }
    }
}

#[cfg(test)]
mod tests;
