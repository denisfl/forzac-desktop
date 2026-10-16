## 1. The list and the plans

- [x] 1.1 `docs/android-parity.md`: every Android reader feature next to the desktop, with file references on both
      sides and a status per row
- [x] 1.2 The maintainer's decision for every gap, unclear and "Not planned for 1.0" row, recorded in the list
- [x] 1.3 `docs/roadmap.md`: `viewer` in 0.4, `two-page-spread` in 1.0, `accessibility` after 1.0, "Not planned for
      1.0" confirmed; proposals for the three changes; language detection out of `localization`

## 2. Core

- [x] 2.1 Language from the text when the book names none (port of `BookLanguage.kt` `guessLanguage`), set in
      `parse`; `Hyphenator::for_language` without the `ru` default; tests: an English FB2 without `<lang>`
      hyphenates in English, a `de` book has no hyphenator (T3)
- [x] 2.2 PT Mono Regular in `assets/fonts/pt-mono/` with `OFL.txt`; registered in `fonts.rs`; `Kind::Code` at 0.85
      and `Style::Code` at 0.9 shaped with it; tests: a code line's glyphs come from PT Mono and keep their spaces (T6)
- [x] 2.3 Strikethrough strokes in `PageContent::strokes` for pages, the column and notes; tests: a struck word has
      one stroke per line it spans, in its ink (T7)
- [x] 2.4 `Book::contents` lists untitled chapters as "Chapter N" when the book has no contents; test (N2)
- [x] 2.5 Link previews: port of `LinkPreview.kt` (≤ 3 blocks, ≤ 600 characters, one chapter); `layout_blocks`
      shared by notes and previews; tests: a figure with a caption previews, a chapter does not, a target across
      chapters does not (N6)
- [x] 2.6 Search in table cells: row text as cells joined by `\n`, matches inside one cell, marks in the cell,
      `text_len` with the separators; tests ported from Android's table search cases (S2)
- [x] 2.7 The end page content `end_page(book, style)`; `save_position(…, finished)` with progress 1.0; tests: a
      finished book reads 100 % in `recent`, its position is the last page's start (M7)
- [x] 2.8 `Error::Damaged` and `Error::Unavailable`; `open` maps I/O errors to `Unavailable`, an unreadable zip to
      `Damaged`; tests for each reason (E1)
- [x] 2.9 C ABI: `forzac_open` with an error out parameter, strokes in `ForzacPage`, `forzac_link_preview`,
      `forzac_end_page`, `forzac_state_save_position` with `finished`; `macos/forzac.h` updated; ABI tests

## 3. GTK shell

- [x] 3.1 PT Mono in the fontconfig configuration and `scripts/package-linux.sh`; strokes drawn (T6, T7)
- [x] 3.2 Clicks in the side thirds turn pages in "Slide" (M2)
- [x] 3.3 The end page after the last page in "Slide" and after the last paragraph in "Scroll"; saved as finished (M7)
- [x] 3.4 Link previews in the note sheet, without a label; no back-stack entry (N6)
- [x] 3.5 The "← Back · Ctrl+O" pill: shown after a jump, gone after two pages or screens away or on return, click
      returns, 150 ms fade (N8)
- [x] 3.6 Theme cross-fade: `CrossFadeNode` for 200 ms on the frame clock, CSS transitions in the panels (C4)
- [x] 3.7 Recent books in the reader's theme and fonts; shown after the last book window closes (C10, P5)
- [x] 3.8 One window per book: an open book's window is presented (D1)
- [x] 3.9 Open errors in a `gtk::AlertDialog` with the reason, the application held; recent books after it when no
      book is open (E1)
- [x] 3.10 Idle inhibition while reading, 10 minutes after the last input (X2)
- [x] 3.11 Help (`?`) and README: the click zones, the pill, the end page

## 4. macOS shell

- [x] 4.1 Strokes drawn; PT Mono through the core's bytes (T6, T7)
- [x] 4.2 Clicks in the side thirds turn pages in "Slide", drags excluded (M2)
- [x] 4.3 The end page in "Slide" and "Scroll"; saved as finished (M7)
- [x] 4.4 Link previews in `NoteSheet` (N6)
- [x] 4.5 The "← Back · ⌃O" pill (N8)
- [x] 4.6 Theme cross-fade with a `CATransition` of 0.2 s (C4)
- [x] 4.7 `RecentWindow` in the reader's theme with `j`/`k`, `q`, `Esc` (C10, D2)
- [x] 4.8 The open error's reason in the `NSAlert` (E1)
- [x] 4.9 `ProcessInfo.beginActivity(.idleDisplaySleepDisabled)` while reading, ended 10 minutes after the last
      input (X2)

## 5. Checks

- [x] 5.1 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p
      forzac-core`, the corpus test (`FORZAC_CORPUS`), `macos/build.sh`
- [x] 5.2 Timing on the Pi 5: `scripts/pi-headless.sh` with `--turns 30` and `--scrolls 20` — no missed frames,
      as before the change; a theme cross-fade misses no frame at 120 Hz; a link preview opens within 30 ms; 0 CPU
      over 10 s idle with the pill shown and after it is gone

## 6. By-hand pass (Raspberry Pi 5 and M1 MacBook Pro)

- [x] 6.1 Raspberry Pi 5 (labwc, over VNC; `PI=… scripts/pi-dev.sh`): every row of `docs/android-parity.md` whose
      status is "same" or whose decision lands in `android-parity`, with an FB2, an FB3, an EPUB with notes, links,
      tables and code, and an English book without a language; screenshots of the end page, a preview, the pill, the
      recent books in Light and Wave, an open error; the screen kept on for 3 minutes with blanking at 1 minute and
      blanking again after 10 minutes without input
- [x] 6.2 M1 MacBook Pro (macOS 14, 60 Hz): the same rows and books from Finder, the Dock and `⌘O`; the same
      screenshots; display sleep kept off while reading and back after 10 minutes
- [x] 6.3 Results recorded in the "By-hand pass" section of `docs/android-parity.md`; every finding fixed here or
      added to the list with a decision, until a pass finds nothing new
