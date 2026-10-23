## 1. Core

- [x] 1.1 `core/src/stats.rs`: `Period`, `compute` (time, days with a book, minutes by day, counted finishes, books one per book, the latest first), `level`, `range`; tests on the `reading-stats` scenarios (read twice in a year, a removed book, New Year's midnight, a moment, phone and Mac, half a minute on each, offline on two devices, approximate dates from two devices)
- [x] 1.2 The calendar's geometry: the week's start by language, a month's grid, `CalendarLayout` for a month and a year (columns, scale); tests (February, a six-week month, both week starts, a year's twelve blocks)
- [x] 1.3 The texts: time, captions with plural forms, period, day line, weekday and month names (RU nominative and genitive), list date, month line, book line; in the catalogue; tests in both languages
- [x] 1.4 The end page: the book's and the month's lines, the month's area `Target::Stats`; tests (lines placed under "Finished", the area focusable); the book's line rules (time only when the log began by the first opening, days only with a first opening)
- [x] 1.5 The C ABI for the macOS shell: the period's stats, the range, the layout, the texts, the end lines; `forzac.h`

## 2. macOS

- [x] 2.1 `macos/Stats.swift`: the window (title and share, segmented, period with arrows, figures, calendar, day line, finished list or the empty line), the theme, the keys, one window
- [x] 2.2 The picture: 1080 × 1920 into a `CGContext`, PNG in the temporary directory, `NSSharingServicePicker` at the share button
- [x] 2.3 Entries: "Reading stats" in the library's "⋯" menu and `s`; the end page's lines and its month opening the stats

## 3. GTK

- [x] 3.1 `gtk/src/stats.rs`: the same window, transient for the window it opens from; drawn with cairo and Pango; keys
- [x] 3.2 The picture rendered to a texture and saved with the file dialog
- [x] 3.3 Entries: the menu item and `s` in the library; the end page's lines and month

## 4. Docs

- [x] 4.1 `docs/android-parity.md` (M7 and a stats row), the roadmap (the library's keys are not in the help, which is the reader's)

## 5. Check

- [x] 5.1 `cargo test -p forzac-core`, clippy, fmt, `macos/build.sh`; the GTK shell builds and passes clippy in a container; under a headless Weston in it, `--stats DIR` drew the window and the picture of a month and a year in Light and Forest, Russian and English (the window's keys, clicks and the file dialog were not run on Linux)
- [x] 5.2 Mac screenshots on a test log (14 months, a removed book, a phone file): month, year, an empty month, a day clicked, the end page with totals — light and dark themes, Russian and English; the picture of a month and a year — done 2026-10-22 (the window and pictures in Light/Russian and Dark/English, an empty log, the end page in both); the share menu itself was not opened by hand
- [x] 5.3 Times on the Mac: the window opens, a period changes, the picture is made (a year of three devices, 50 books) — the core's month 0.15 ms and year 0.2 ms (release, 782 lines, 50 finishes), the picture 30 ms on the M1
- [x] 5.4 (not measured: archived without it at the maintainer's request, 2026-10-23) On the Pi 5 (with the maintainer's consent): the same screens in GTK, the window within 150 ms, a period within 50 ms
