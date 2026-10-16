## 1. Core

- [x] 1.1 `search`: the Android rules (normalisation, soft hyphens, spaces, two characters), snippets, section and
      position; tests ported from `SearchTest.kt`
- [x] 1.2 Lines keep glyph text offsets; `marks` for a page and for a column range; tests
- [x] 1.3 C ABI: search results, marks for a page and a column range

## 2. Shells

- [x] 2.1 GTK: the search panel, live search on a worker, `Enter`/click, highlight in Slide and Scroll, `n`/`N`, `Esc`
- [x] 2.2 macOS: the same, `⌘F`, a menu item
- [x] 2.3 Help and README

## 3. Verification

- [x] 3.1 fmt, clippy, tests; time of a whole-book search on the Pi
- [x] 3.2 Pi and Mac: the panel in three themes, a match highlighted in Slide and Scroll, `n`/`N` across chapters
