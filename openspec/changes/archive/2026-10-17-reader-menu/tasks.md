# Tasks

## 1. Core: the book's pages and the labels

- [x] 1.1 `BookPages` in `core/src/layout/pages.rs` (`count` with a stop flag, `total`, `chapter_starts`, `page_of`,
  `start`, `style`); tests: the count equals the sum of `paginate`'s chapter pages, `page_of(start(p)) == p` for every
  page, a set stop flag returns `None` — `cargo test -p forzac-core`
- [x] 1.2 `ChapterLayout::render_numbered` draws the foot "N of M" with a matching count; test: the foot reads
  "37 of 212"-style text, and "4 / 22 · N%" without a count
- [x] 1.3 `Book::section_at` and `Book::minutes_left` (characters, 1000 a minute, rounded); tests with Cyrillic text
- [x] 1.4 New strings with Russian in `core/src/i18n.rs`; the help texts list `Enter` as the menu (Linux and macOS, both
  languages); the catalogue test passes
- [x] 1.5 The C ABI (`forzac_count_*`, `forzac_book_pages`, `forzac_book_page_of`, `forzac_book_page_start`,
  `forzac_chapter_starts`, `forzac_section`, `forzac_minutes_left`; `forzac_page` draws the counted foot) in
  `core/src/ffi.rs` and `macos/forzac.h`; an ffi test runs a count through it
- [x] 1.6 The corpus test counts every corpus book and prints the slowest count —
  `FORZAC_CORPUS=~/Books cargo test -p forzac-core --test corpus`

## 2. GTK shell

- [x] 2.1 The page view counts the book in the background (after the first frame, 300 ms after a style change, stale
  jobs stopped), drops page nodes on arrival outside animations, draws the counted foot; `--measure` prints when the
  count arrived — checked with `--measure` on a long book
- [x] 2.2 `gtk/src/menu.rs`: the bars, the slider, the labels, the fade and slide, in the theme's colours; the back,
  contents and "Aa" buttons — checked by a screenshot (`--menu` test flag opens it after the first frame)
- [x] 2.3 Keys and clicks: `Enter` opens, `Esc` closes, `←`/`→`/`h`/`l` scrub, `Enter` jumps (pushed for `Ctrl+O`),
  other keys close and act; the middle third opens, a click closes; the slider's drag and release — checked with
  `--mouse` and `--keys` runs
- [x] 2.4 Lists: contents and bookmarks numbers, highlights and search "section · p. N" when counted, percent before
- [x] 2.5 Build and clippy in the `ci/debian` container as `scripts/pi-dev.sh` does

## 3. macOS shell

- [x] 3.1 `PageView.swift`: the background count through the C ABI, page bitmaps dropped on arrival
- [x] 3.2 `macos/Menu.swift`: the bars, the slider, the labels, the animation, the buttons (back to the recent books,
  contents, settings) — checked with `--png` and a `--panel menu` screenshot
- [x] 3.3 Keys and clicks as in GTK — checked with `--keys` and `--mouse`
- [x] 3.4 Lists in `Panels.swift` as in GTK
- [x] 3.5 `macos/build.sh` builds

## 4. Docs

- [x] 4.1 docs/android-parity.md: R1 and R3 reversed (decisions table, rows R1, R3, N1, S3, M2, N9), the by-hand pass of
  this change; docs/roadmap.md: the change in milestone 1.0
- [x] 4.2 The help texts and README keys mention the menu where they list keys

## 5. Verification on hardware

- [x] 5.1 `cargo fmt --all --check`, `cargo clippy` (core on the Mac, workspace in the container), `cargo test -p
  forzac-core`, the corpus test
- [ ] 5.2 Raspberry Pi 5, `scripts/pi-headless.sh` with the default, `ngl` and `gl` renderers: page turns while the
  count runs, 0 missed frames (or no more than the build before this change), idle 0 CPU ticks; the time to count the
  longest book recorded
- [ ] 5.3 Raspberry Pi 5 by hand (headless labwc, `wtype`, `grim`), English and Russian: `Enter` opens the menu, `→`
  scrubs, `Enter` jumps, `Ctrl+O` returns, `Esc` closes; the foot "N of M"; the lists' numbers
- [x] 5.4 The Mac by hand (`--panel`, `--keys`, `--mouse`, `-AppleLanguages "(ru)"`): the same, and the click in the
  middle third, the slider drag, the back button
- [x] 5.5 Screenshots described in docs/android-parity.md
