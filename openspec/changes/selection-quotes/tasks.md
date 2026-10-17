## 1. Core

- [x] 1.1 `state`: `quote` lines; `Store::quotes`, `Store::set_quotes`; tests
- [x] 1.2 Words: `ChapterLayout::words`, `flow_words`; `word_at`, `step`, `line_step`, `find`, `span`;
      `Book::block_range`; tests (a hyphenated word, a table cell, punctuation, hit-testing between lines)
- [x] 1.3 `quotes`: entries, `share_text`, `export` (Android's cases); `Target::Highlight`; tests
- [x] 1.4 C ABI and `macos/forzac.h`; tests

## 2. GTK shell

- [x] 2.1 Selection state; mouse: drag by words, double and triple click, `Shift`+click, handles, edge turning and
      scrolling; a click clears
- [x] 2.2 Keyboard mode: `v`, `h l j k`, arrows, `o`, `Esc`, crossing pages and the column
- [x] 2.3 The bar, the clipboard, `Enter`/`c`/`Ctrl+C`
- [x] 2.4 Highlights: storage, bands baked into pages and tiles, the stroke, `Tab` stops, clicks, the highlight's bar
- [x] 2.5 The "Highlights" tab: list, `Enter`, `d`, `c`, `Ctrl+C` export, the empty hint; help text

## 3. macOS shell

- [x] 3.1 Selection: mouse, keyboard mode, crossing pages
- [x] 3.2 The bar, `⌘C`, Edit → Copy, Share (`NSSharingServicePicker`)
- [x] 3.3 Highlights: bitmaps and tiles, the masked stroke, `Tab` stops, clicks, the highlight's bar
- [x] 3.4 The "Highlights" tab, its share button, View → Highlights; help text

## 4. Docs

- [x] 4.1 README keys; `docs/roadmap.md` (out of "Not planned for 1.0", a row for this change);
      `docs/android-parity.md` (Q2, NP2)

## 5. Verification

- [x] 5.1 fmt, clippy, tests, the corpus test; both shells build
- [x] 5.2 Pi 5: `scripts/pi-headless.sh` 0 missed frames and 0 idle CPU as before; paired page turns and scrolls with
      highlights on the pages; the stroke's frames; the time of a save and of the list with 200 highlights
- [x] 5.3 Pi 5 by hand: keyboard selection across a page and in "Scroll", Highlight, Copy (the clipboard's text), the
      list (go, delete, copy all), a larger text size keeps the markers on their words, a restart keeps them; screenshots
- [ ] 5.4 Mac by hand: the same; the mouse drag, double and triple click where it can be driven; Share; screenshots
