## Why

On 2026-10-17 the maintainer looked at the Mac build next to the Android app and the web reader
(forzac.truenrth.app/read/) and found that the desktop looks like neither: no library, only a plain list of recent
books. They decided the desktop should be the web reader 1:1, with the desktop's keys. This reverses the
`android-parity` decisions NP3 and P4 ("no library screen; recent books are enough").

## What Changes

- The recent-books window of both shells (GTK `recent_window`, macOS `RecentWindow`) becomes the library, laid out as
  the web reader's (`site/public/read/index.html` `#library`, `reader.css`, `js/app.ts`): the brand and an "Open a
  book" button in the head, a note line, a shelf of covers with the title and the author under each, a dashed "Release
  the file to open it" overlay while a file is dragged over the window, and the web's empty state (an invitation, the
  button, "or drop a file into this window", the privacy line, an illustrated book page).
- From the Android app (`LibraryScreen.kt`, `Covers.kt`): the "Continue reading" card for the last opened unfinished
  book above the shelf, the share read under each cover ("Finished" when done), generated covers for books without one
  (cloth or marbled paper, a spine, a paper label with the title and the authors), search by title or author with its
  ranking and transliteration, and a question before removing a book.
- Books still open in place: the library is the reading state's books whose files exist. Removing a book from the
  library forgets its entry (the file stays). Covers are cached as small JPEG thumbnails in the user's cache directory,
  made when a book is opened and, for books opened before this change, in the background while the library is shown;
  the state file does not grow.
- Keys: arrows and `h` `j` `k` `l` move across the shelf, `Enter` opens, `Delete` or `d` asks to remove, `/` searches,
  `Ctrl+O` / `⌘O` opens a file, `q` or `Esc` closes (Esc first leaves the search). The library shows when Forzac starts
  without a book and when the last book window closes, as the recent books do now, in the reader's theme and interface
  font.
- New strings through the catalogue in the web's and the Android app's words.
- Test flags: GTK `--png out.png` and `--measure` without a book save or time the library; macOS `--library` with
  `--png`, `--keys` and `--state PATH`.

## Non-Goals

Importing or copying books; folders or collections; sorting options; the "Continue" card's chapter title and time left
(the state does not keep a book's length or chapter names); a library inside the book window; the contents panel's
cover (it stays as it is).

## Capabilities

### New Capabilities
- `library`: the library window — shelf, covers, continue card, opening, dropping, removing, search, empty state,
  keys, cover cache.

### Modified Capabilities
- `reading-state`: "Recent books" is replaced by the library; the state can forget a book.
- `reader-window`: after the last book, and after a file that cannot be opened, the library follows.
- `macos-app`: started without a book, the Mac app shows the library.
- `linux-integration`: the library's default window size.
- `localization`: the library's strings are translated.

## Impact

`core/src/state.rs` (all books, forgetting one), a new `core/src/library.rs` (search rank, continue book, generated
cover style, marbled paper, the empty state's sample page) with its C ABI in `ffi.rs` and `macos/forzac.h`;
`core/src/i18n.rs`; GTK: a new `gtk/src/library.rs`, `main.rs` (the window's place, cover caching, flags); macOS: a
new `macos/Library.swift`, `main.swift`. Docs: `docs/android-parity.md` (NP3, P4 reversed), `docs/roadmap.md`.
