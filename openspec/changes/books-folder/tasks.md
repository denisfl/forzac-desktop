# Tasks

## 1. Core

- [x] 1.1 `library::books_folder(home, language, store)`: `~/Книги` / `~/Books`, remembered as the setting
      `books.folder`, made when missing; tests (Russian and English, an existing folder kept, the remembered path wins
      after a language switch, a deleted folder made again)
- [x] 1.2 `library::file_name(title, authors, original, taken)`: `Author — Title.ext`, `.fb2.zip` kept whole, forbidden
      characters and a leading dot replaced, 120 characters at a character boundary, ` (2)`… for a taken name; tests
      (no author, no title, Cyrillic, a 300-character title, `/` and `:` in a title, three books with one name)
- [x] 1.3 `state`: the `file` line kind (name, size, mtime, id, title, authors) read, written and kept by other writes;
      tests (round trip, a 0.9.0 state without `file` lines, damaged lines dropped)
- [x] 1.4 `library::scan(folder, store)`: supported files at the top level only, the cache reused on name/size/mtime,
      gone files dropped, one write when something changed; merged with the remembered entries by id, one entry per
      id; tests in a temp folder (a new file, a changed file, a removed file, a PDF ignored, two files with one id,
      a damaged EPUB ignored) and a timing test: 500 cached files under 50 ms on the Mac (the Pi in 5.3)
- [x] 1.5 `library::import(src, folder, store)`: a file inside the folder returned as it is, a known id returns its
      copy, else `.name.part` + rename, remembered; the original untouched; tests (copy, duplicate under another name,
      a file already inside, no space or no permission → an error and no `.part` left)
- [x] 1.6 The one-time move of the remembered books (decision 6): runs when `books.folder` is unset, entries pointed at
      the copies with positions, bookmarks and highlights kept, failures skipped; tests (three books, one missing,
      one unreadable)
- [x] 1.7 C ABI: `forzac_books_folder`, `forzac_import`, `forzac_library_books` (with "never opened"), the move;
      `macos/forzac.h`; tests through the C ABI

## 2. GTK shell

- [ ] 2.1 Every open (file dialog, drop, command line, `xdg-open`, a second instance's file) goes through
      `forzac_import` in `gio::spawn_blocking`; errors as `reader-window`; the move before the first library show
- [ ] 2.2 The library from `forzac_library_books`: folder-only books as not started, their covers through the existing
      cover cache
- [ ] 2.3 Remove: the dialog text, `gio::File::trash`, forget only after the trash worked, the error message
- [ ] 2.4 "Show the Books folder": the head button, `F`, the empty state's line; `gtk::FileLauncher`; help text
- [ ] 2.5 The library's note line under the head removed; the empty state keeps only "FB2, FB2.ZIP, FB3 and EPUB are
      supported." under its rule

## 3. macOS shell

- [ ] 3.1 Every open (open panel, drop, Finder and the Dock, `⌘O`, the command line) through `forzac_import` on a
      background queue; the move before the first library show
- [ ] 3.2 The library from `forzac_library_books`, folder-only books not started
- [ ] 3.3 Remove: dialog text, `FileManager.trashItem`, forget after it worked, the error message
- [ ] 3.4 "Show the Books folder": head button, `F`, the empty state, `NSWorkspace`; the menu item; help text
- [ ] 3.5 The note line removed as in 2.5

## 4. Docs

- [ ] 4.1 README (where books live, removing goes to the Trash), `docs/roadmap.md`, the release-notes draft for the next
      version (the folder, the one-time move, **BREAKING** remove), the site's `/desktop/` page only if it says books
      open in place

## 5. Verification

- [ ] 5.1 fmt, clippy, `cargo test`, the corpus test; both shells build; CI green
- [ ] 5.2 Mac, by hand: a 0.9.0 state with books in Downloads → first start of this build copies them into `~/Книги`
      (at most one Downloads prompt), each opens where it was left; a book opened from Finder is copied once; a book
      dropped into the folder in Finder appears; remove → in the Trash, put back → highlights back; `F` opens Finder;
      no permission prompt on the second start
- [ ] 5.3 Linux, by hand on the Pi 5's Raspberry Pi OS (USB stick; the SD card holds the music server, pull it to
      boot the stick) or headless sway in the Arch container: the same checks with `xdg-open`, the file manager and
      `gio trash`; `scripts/pi-headless.sh` — library show with 500 books in the folder within budget, idle 0 CPU
