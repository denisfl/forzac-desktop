## 1. Core

- [ ] 1.1 `state`: `quote` lines; `Store::quotes`, `Store::set_quotes`; tests
- [ ] 1.2 Words: `ChapterLayout::words`, `flow_words`; `word_at`, `step`, `line_step`, `find`, `span`;
      `Book::block_range`; tests (a hyphenated word, a table cell, punctuation, hit-testing between lines)
- [ ] 1.3 `quotes`: entries, `share_text`, `export` (Android's cases); `Target::Highlight`; tests
- [ ] 1.4 C ABI and `macos/forzac.h`; tests

## 2. GTK shell

- [ ] 2.1 Selection state; mouse: drag by words, double and triple click, `Shift`+click, handles, edge turning and
      scrolling; a click clears
- [ ] 2.2 Keyboard mode: `v`, `h l j k`, arrows, `o`, `Esc`, crossing pages and the column
- [ ] 2.3 The bar, the clipboard, `Enter`/`c`/`Ctrl+C`
- [ ] 2.4 Highlights: storage, bands baked into pages and tiles, the stroke, `Tab` stops, clicks, the highlight's bar
- [ ] 2.5 The "Highlights" tab: list, `Enter`, `d`, `c`, `Ctrl+C` export, the empty hint; help text

## 3. macOS shell

- [ ] 3.1 Selection: mouse, keyboard mode, crossing pages
- [ ] 3.2 The bar, `⌘C`, Edit → Copy, Share (`NSSharingServicePicker`)
- [ ] 3.3 Highlights: bitmaps and tiles, the masked stroke, `Tab` stops, clicks, the highlight's bar
- [ ] 3.4 The "Highlights" tab, its share button, View → Highlights; help text

## 4. Docs

- [ ] 4.1 README keys; `docs/roadmap.md` (out of "Not planned for 1.0", a row for this change);
      `docs/android-parity.md` (Q2, NP2)

## 5. Verification

- [ ] 5.1 fmt, clippy, tests, the corpus test; both shells build
- [ ] 5.2 Pi 5: `scripts/pi-headless.sh` 0 missed frames and 0 idle CPU as before; paired page turns and scrolls with
      highlights on the pages; the stroke's frames; the time of a save and of the list with 200 highlights
- [ ] 5.3 Pi 5 by hand: keyboard selection across a page and in "Scroll", Highlight, Copy (the clipboard's text), the
      list (go, delete, copy all), a larger text size keeps the markers on their words, a restart keeps them; screenshots
- [ ] 5.4 Mac by hand: the same; the mouse drag, double and triple click where it can be driven; Share; screenshots
