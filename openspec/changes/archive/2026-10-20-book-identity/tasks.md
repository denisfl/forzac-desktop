# Tasks

## 1. Core

- [x] 1.1 `sha2` dependency; `state::book_id` = the first 16 bytes of SHA-256 in lowercase hex; the old FNV id kept as
      `state::legacy_id` for the move; tests (a known SHA-256 vector, equal content equal id, 32 hex digits)
- [x] 1.2 `Store::rename_ids(map)`: `book`, `bookmark`, `quote` and `file` lines renamed in one write, no write when
      nothing matches; tests (round trip through the file, an entry already under the new id kept once)
- [x] 1.3 The move in `library::books_folder` guarded by `books.ids`: old ids mapped only when the file's FNV id still
      matches; a fresh state marked; tests (two books with bookmarks and a highlight, a missing file kept, a changed
      file kept, the move runs once)
- [x] 1.4 The late rename in the folder scan for newly seen files; test (a file put back after the move brings its
      highlights)
- [x] 1.5 `cargo fmt`, `cargo clippy -p forzac-core`, `cargo test -p forzac-core`, the corpus test

## 2. Shells and checks

- [x] 2.1 `macos/build.sh` and the GTK build (container) green; `forzac_book_id`'s doc comment
- [x] 2.2 By hand on the Mac with a copy of a 0.11.0 state: books, progress, bookmarks and highlights kept, covers
      remade; the same file's id equals Android's `BookImport.idOf`
- [x] 2.3 `docs/roadmap.md` status
