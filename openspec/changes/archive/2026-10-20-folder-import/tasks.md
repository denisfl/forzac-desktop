# Tasks

## 1. Core

- [x] 1.1 `library::book_files`: the walk (subfolders, dot entries, links to folders, the Books folder skipped)
- [x] 1.2 `library::import_folder` sharing `copy_in` with `import`: one scan, duplicates by id within the import, one
      state write, progress; `Added` and `Added::message`; tests (subfolders, duplicates, damaged, PDF ignored, source
      untouched, twice, the home folder, a link loop, the messages)
- [x] 1.3 Strings in English and Russian with plurals; the i18n test pairs a count's singular with its plural
- [x] 1.4 C ABI `forzac_import_folder`, `forzac_added_message`, `macos/forzac.h`; a test through the C ABI

## 2. Shells

- [x] 2.1 GTK: a dropped folder and `Ctrl+Shift+O` add it; the head's progress in place of the search button; the
      books reloaded; the message; folders queued
- [x] 2.2 macOS: a dropped folder, File › Add a Folder… `⇧⌘O`, folders in the `⌘O` panel; the progress; the books
      reloaded; the message
- [x] 2.3 Both builds and clippy green; by hand on the Mac: a folder with subfolders and a damaged file through File ›
      Add a Folder… and `⇧⌘O` ("Added 5 books. 1 file couldn't be read.", the source untouched). Not by hand: the second
      add and the Russian messages (covered by the core tests; the screen was needed for another check), the GTK
      shell (no Linux desktop at hand)
- [x] 2.4 `docs/roadmap.md` status
