## 1. Core

- [x] 1.1 `state`: `bookmark` lines read and written with the rest; `Store::bookmarks`, `Store::set_bookmarks`; tests
      (round trip, other books kept, damaged lines dropped)
- [x] 1.2 `bookmarks`: `toggle`, `marked`, `entries` (section, position, snippet); `ChapterLayout::page_end`; tests
      (a page's range after a re-layout, the snippet's cleaning)
- [x] 1.3 C ABI: load and save, toggle, remove, marked, places, entries, `forzac_page_end`; `macos/forzac.h`; tests

## 2. GTK shell

- [x] 2.1 `b` and the corner click: the range in "Slide" and "Scroll", saving through the store
- [x] 2.2 The ribbon over the page and in the column; sliding with the page; drop and rise on the frame clock
- [x] 2.3 The contents panel's tabs and the bookmarks list: `B`, tab keys, `Enter`, `d`/`Delete`, the empty hint
- [x] 2.4 Help text

## 3. macOS shell

- [x] 3.1 `b`, `⌘D` and the corner click; loading and saving with the state
- [x] 3.2 The ribbon layers; spring drop and rise; "Scroll"
- [x] 3.3 The panel's tabs and the bookmarks list; View → Bookmarks; help text

## 4. Docs

- [x] 4.1 README keys, `docs/android-parity.md` Q1 (the roadmap lists changes without a status: unchanged)

## 5. Verification

- [x] 5.1 fmt, clippy, tests, the corpus test; both shells build
- [x] 5.2 Pi 5: `scripts/pi-headless.sh` — page turns and scrolling with bookmarks on the pages miss no more frames than
      without, idle 0 CPU; the time of a toggle's write and of opening the list with 200 bookmarks
- [x] 5.3 Pi 5 by hand: set and remove in "Slide" and "Scroll", the drop and rise, the list (go, delete, empty hint),
      a larger text size keeps the marked text, a restart keeps the bookmarks; screenshots
- [ ] 5.4 Mac by hand: the same, plus `⌘D` and View → Bookmarks; screenshots (keys done through `--keys`; the menus,
      the corner click and a restart wait for the maintainer)

Recorded in `docs/android-parity.md` ("Bookmarks"). Not checked by hand: `B` on the Pi (no shifted keys through
`wtype`), the corner click (no pointer injection), the Mac menu items.
