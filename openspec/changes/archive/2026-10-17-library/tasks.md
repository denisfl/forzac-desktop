## 1. Core

- [x] 1.1 `state`: `Store::books()` (every entry whose file exists, last opened first) and `Store::forget(id)`; tests
- [x] 1.2 `library`: `rank` (Android's `librarySearchRank` with transliteration), `continue_book`, the progress label's
      rule, the generated cover's style (Java `hashCode`, marble or cloth, colours, title size factor), the marbled
      paper's pixels, the empty state's sample page per language; tests
- [x] 1.3 C ABI and `macos/forzac.h`: `ForzacRecent.id`, `forzac_state_forget`, `forzac_library_rank`,
      `forzac_cover_style`, `forzac_marble`, `forzac_sample_page`
- [x] 1.4 Catalogue: the library's strings in English and Russian (web and Android words); remove the recent books'
      strings

## 2. GTK shell

- [x] 2.1 `gtk/src/library.rs`: the window (head, note, card, shelf, empty state with the sample page), `Shelf` and
      `Cover` widgets, theme CSS
- [x] 2.2 Keys (arrows, `h` `j` `k` `l`, `Home`, `End`, `Enter`, `Delete`/`d`, `/`, `Ctrl+O`, `q`/`Esc`/`Ctrl+W`), the
      removal question, search, the file dialog, dropping files with the overlay
- [x] 2.3 Cover cache: a thumbnail when a book window opens; filling missing ones in the background; loading thumbnails
      off the main thread
- [x] 2.4 `main.rs`: the library instead of the recent books (one window at a time), `--measure` and `--png` without a
      book

## 3. macOS shell

- [x] 3.1 `macos/Library.swift`: the window, layout, `CoverView`, empty state with the sample page
- [x] 3.2 Keys, the removal question, search, `⌘O`, dropping files with the overlay
- [x] 3.3 Cover cache as in GTK (ImageIO)
- [x] 3.4 `main.swift`: the library instead of `RecentWindow`; `--library`, `--keys` for it, `--state PATH`

## 4. Docs

- [x] 4.1 `docs/android-parity.md`: NP3 and P4 reversed (with the date and the maintainer's words), C10, D2, N3 updated;
      `docs/roadmap.md`: "Not planned" without the library, the library in the plan; README if it names the recent books

## 5. Verification

- [x] 5.1 `cargo fmt`, `clippy -D warnings`, `cargo test -p forzac-core`, the corpus test; GTK builds in the
      `ci/debian` container; `macos/build.sh`
- [ ] 5.2 Pi 5: 50 books in the state, covers cached: `--measure` first frame ≤ 200 ms; idle 0 CPU ticks in 10 s with
      the library open
- [x] 5.3 GTK by hand in the `ci/debian` container (headless labwc, `wtype`, `grim`) instead of the Pi, English and Russian: empty state, shelf with covers and
      generated covers, card, moving with keys, search, removing (question, `Esc`, `Enter`), opening, the library after
      the last book, a non-book from the command line
- [x] 5.4 Mac by hand in English and Russian (`-AppleLanguages "(ru)"`, `--library --png`, `--keys`): the same screens
- [x] 5.5 Record the pass in `docs/android-parity.md`

Not done: 5.2 and the Pi part of 5.3. The Pi 5 was busy with `reader-menu` and its SSH host key had changed; the
maintainer chose to go on without it (2026-10-17). The container's numbers are recorded in `docs/android-parity.md`.
