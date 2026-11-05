## 1. Core

- [x] 1.1 `Book.pictures` from a pre-paginated EPUB by Android's rule — check: tests for JPEGs in the spine, XHTML pages, a text page skipped, too many text pages, right to left, a blank page, the cover
- [x] 1.2 Place, progress, log, sync and Kindle by `Book::page_count` — check: core tests
- [x] 1.3 Pages, their sizes and the direction in the C ABI — check: used by the Mac shell

## 2. Shells

- [x] 2.1 macOS: the window draws from a source; picture pages on the theme's paper, spreads, right to left, `t`/`T` — check: `--png`, `--keys`
- [x] 2.2 GTK: the same — check: `cargo clippy`, under headless Weston

## 3. Check

- [x] 3.1 `cargo test -p forzac-core`, `clippy`, `fmt`; `openspec validate fixed-layout --strict`
- [x] 3.2 Ten turns on the Mac (236 frames, 3 late, max 14.5 ms, as a book); a PDF still works
- [x] 3.3 `docs/android-parity.md` F6 and NP6, the roadmap, README
