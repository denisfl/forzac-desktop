# Design

## Context

Books are opened where they lie. The state file (`core/src/state.rs`, `forzac-state 1`) remembers each book by its id
(`book_id`: FNV-1a over the length and the first and last 64 KiB) with its path, title, authors and position; the
library lists the remembered books whose paths exist. Covers are cached by id in the shells
(`$XDG_CACHE_HOME/forzac/covers`, the macOS equivalent). On macOS a file opened from Finder or the open panel is
readable in that launch; reopening it later from the library touches a protected folder (Downloads, Documents,
Desktop) and macOS asks for access.

## Goals / Non-Goals

**Goals:** one place that owns the files; every rule about the folder (where, names, duplicates, scan, the one-time
move) in the core and tested there; the shells only pick the home folder, run the import off the UI thread, move
files to the Trash and show the folder.

**Non-Goals:** a file watcher, subfolders, a database other than the state file, any change to `book_id`.

## Decisions

**1. Where the folder is: `~/Книги` or `~/Books`, decided once, remembered.** The core decides from the home path
and the interface language (`i18n::language()`), and remembers the path as the setting `books.folder`. The home root
(not `~/Documents`) because macOS protects Documents, Desktop and Downloads with a permission prompt, and the home
root itself is not protected; Calibre's `~/Calibre Library` sits there for the same reason. A localized name because
the folder is something the reader sees every day in Finder; it is fixed after creation so switching the language
never splits the library. *Alternatives:* an app-private folder like Android's (`~/Library/Application Support`,
`~/.local/share`) — invisible, cannot be dropped into or synced; a setting — not needed until syncing asks for it.

**2. Import in the core, called off the UI thread.** `library::import(src, folder, store) -> io::Result<PathBuf>`:
if `src` is inside the folder, return it. Otherwise read the file (it is read anyway for `book_id`), compute the id;
if the folder's cache (decision 3) has that id, return that file; else parse only what is needed for the name
(title, first author), build the name (decision 4), write to `.<name>.part` in the folder and rename it into place
(atomic: a half-copied book never shows), then remember it. Over the C ABI: `forzac_import(state, path, error) ->
char*` (the path to open, freed with `forzac_string_free`), `error` as `forzac_open`'s. GTK calls it in
`gio::spawn_blocking`, macOS on a background `DispatchQueue`; the window opens when it returns. *Alternative:* shells
copy and the core only names — would duplicate the duplicate check and naming in Swift and Rust.

**3. The folder scan with a cache in the state.** A new line kind in the same file, ignored by older versions (they
skip lines they cannot read):
`file<TAB>name<TAB>size<TAB>mtime<TAB>id<TAB>title<TAB>authors`. `library::scan(folder, store)` lists the folder's
supported files (not subfolders, not dotfiles), reuses a cached line when name, size and mtime match, and otherwise
reads the file for its id, title and authors; it drops lines of files that are gone and writes the state once if
anything changed. The library = the scan's books merged with the remembered entries by id (a remembered entry gives
position, progress and "opened"; a folder-only book is "not started"); two files with one id (a sync service's
conflict copy) show once. FFI: `forzac_library_books(state, count) -> *const ForzacRecent` with the same struct the
library uses now, plus a flag for "never opened". 500 unchanged files = 500 `stat` calls, well inside 50 ms on the Pi.

**4. Names.** `Author — Title.ext`: the first author as the book gives it ("Лев Толстой"), an em dash with spaces, the
title; `/`, `:`, `\0`–`\x1f` and a leading dot replaced by `_`; trimmed to 120 characters at a character boundary;
the original extension kept, `.fb2.zip` as a whole; ` (2)`, ` (3)` while the name is taken by another id. The core
decides the name, so both shells produce the same files.

**5. Removing = Trash, in the shells.** The Trash is platform API: `gio::File::trash` (GTK), `FileManager.trashItem`
(macOS). The shell trashes first and calls `forzac_state_forget` only when that worked; bookmarks and highlights stay
keyed by id as today, so a file put back comes back with them.

**6. The one-time move.** When `books.folder` is not yet set: make the folder, then `import` every remembered entry
whose path exists outside it and point the entry at the copy (same id, so position, bookmarks and highlights carry
over), then set `books.folder`. Errors per book are logged and skipped. This runs once, at the first library show or
the first book open of this version, off the UI thread.

**7. Showing the folder.** Shell-side: `gtk::FileLauncher` / `xdg-open` on the folder (GTK), `NSWorkspace.open` /
`activateFileViewerSelecting` (macOS). The folder path comes from `forzac_books_folder(state)`.

## Risks / Trade-offs

- [The one-time move touches Downloads on macOS] → one permission prompt at most, once, on the first start of this
  version; afterwards every book lives in the home root. Said in the release notes.
- [Copies double the disk use of books opened from elsewhere] → books are small (median under 2 MB); the original is
  the reader's to delete.
- [Removing now deletes (to the Trash)] → the dialog says so; the Trash keeps the file; **BREAKING** noted in the
  release notes.
- [A sync service changes a file while the library reads it] → the cache key (size, mtime) changes and the file is
  read again next time; a half-written file that fails to parse is ignored until it parses.
- [The folder was deleted by hand while books were remembered] → it is made again empty; the remembered books whose
  files are gone are not shown, as with any missing file.

## Migration Plan

Ships in the next release. On first start: folder made, library copied in (decision 6). Rollback to 0.9.0: the state
file still has `book` lines (now pointing into the folder) that 0.9.0 reads; the `file` lines are skipped by 0.9.0.

## Open Questions

- Whether the folder should also be used for books sent to a Kindle (converted copies) or a separate one — decided in
  the Kindle change.
