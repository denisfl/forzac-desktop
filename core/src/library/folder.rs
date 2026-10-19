//! The Books folder (`books-folder`): where it is, the names of the copies, reading the folder for the library with a
//! cache in the state, copying a book in, and moving the 0.9.0 library in once.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::i18n::Language;
use crate::state::{Cached, Entry, Store, book_id};

/// The setting that remembers the folder; unset until the folder is made and the library moved in.
pub const FOLDER: &str = "books.folder";
/// The longest name of a copy, in characters, before ` (2)` and the extension.
const NAME_CHARS: usize = 120;
/// Extensions of the books the library reads, longest first: `.fb2.zip` is one extension.
const EXTENSIONS: [&str; 4] = [".fb2.zip", ".fb2", ".fb3", ".epub"];

/// The Books folder: the remembered one, else `~/Книги` (a Russian interface) or `~/Books` in `home`, made when missing.
/// The first time, the remembered books are moved in ([`move_in`]) before the folder is remembered.
pub fn books_folder(home: &Path, language: Language, store: &mut Store) -> io::Result<PathBuf> {
    if let Some(folder) = store.setting(FOLDER) {
        let folder = PathBuf::from(folder);
        std::fs::create_dir_all(&folder)?;
        return Ok(folder);
    }
    let folder = home.join(if language == Language::Russian { "Книги" } else { "Books" });
    std::fs::create_dir_all(&folder)?;
    move_in(&folder, store);
    store.set_setting(FOLDER, &folder.to_string_lossy())?;
    Ok(folder)
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

/// What `path` holds: its id, title and authors; an empty id when it is not a readable book.
fn read_file(name: String, meta: &std::fs::Metadata, path: &Path) -> Cached {
    let (size, mtime) = (meta.len(), mtime(meta));
    let book = std::fs::read(path).ok().and_then(|bytes| crate::parse(&bytes).ok());
    match book {
        Some(b) => Cached { name, size, mtime, id: b.id, title: b.title, authors: b.authors.join(", ") },
        None => Cached { name, size, mtime, id: String::new(), title: String::new(), authors: String::new() },
    }
}

/// Reads the folder's books (FB2, FB2.ZIP, FB3, EPUB at its top level; not dotfiles, not subfolders), parsing only the
/// files that are new or changed since the cache in `store`, which is written once when anything changed. Unreadable
/// files are ignored; files with one id count once.
fn folder_files(folder: &Path, store: &mut Store) -> Vec<Cached> {
    let mut files = Vec::new();
    let mut changed = false;
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
                read_file(name, &meta, &item.path())
            }
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    if changed || files.len() != store.files().len() {
        let _ = store.set_files(files.clone());
    }
    files
}

/// The library: the remembered books whose files exist (a renamed file in the folder found by its id), the last
/// opened first, then the folder's books never opened, the newest file first, as `opened == 0`. One entry per id.
pub fn scan(folder: &Path, store: &mut Store) -> Vec<Entry> {
    let files = folder_files(folder, store);
    let in_folder = |id: &str| files.iter().find(|f| f.id == id).map(|f| folder.join(&f.name));
    let mut remembered: Vec<Entry> = store
        .entries()
        .iter()
        .filter_map(|e| {
            let path = if e.path.is_file() { e.path.clone() } else { in_folder(&e.id)? };
            Some(Entry { path, ..e.clone() })
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
        position: Default::default(),
        progress: 0.0,
        opened: 0,
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
    let book = crate::parse(&bytes)?;
    let original = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let fallback = if extension(&original).is_some() { "" } else { content_extension(&bytes) };
    let name = file_name(&book.title, &book.authors, &original, fallback, |n| folder.join(n).exists());
    let (part, path) = (folder.join(format!(".{id}.part")), folder.join(&name));
    let written = std::fs::write(&part, &bytes).and_then(|_| std::fs::rename(&part, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
        return Err(crate::Error::Unavailable);
    }
    if let Ok(meta) = std::fs::metadata(&path) {
        let mut files = store.files().to_vec();
        files.push(Cached { name, size: meta.len(), mtime: mtime(&meta), id, title: book.title, authors: book.authors.join(", ") });
        let _ = store.set_files(files);
    }
    Ok(path)
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
