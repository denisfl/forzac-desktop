## Why

With the reading log (`reading-log`) the desktop knows how much and when the reader read and what they finished, on
this computer and, through sync, on the phone. The Android app shows the same data since 0.9.10 as reading statistics:
a month and a year with a reading calendar ("a month in squares"), the book's totals on its end page, and a picture to
share. The maintainer asked for the same on the desktop (2026-10-22). The numbers are the phone's: both count the log of
every device.

## What Changes

- **"Reading stats" window** ("Статистика"): "Month · Year", the period with arrows to its neighbours (from the first
  month with data to the current one), three figures — time read, books finished, days with a book — the reading
  calendar and the list "Finished" of the period's books. The current month when it opens. In the reader's theme;
  the days shaded with the theme's link colour by four levels of minutes read, as on Android. A click on a day says
  "18 October · 52 min". Keys: `←` `→` (`h` `l`) for the periods, `m` and `y` for month and year, `Esc` closes.
- **Share.** On macOS a share button (and `s`) makes the period's picture, 1080 × 1920 like the phone's — the period
  large, the calendar, the three figures, a small "Forzac" — and opens the system's share menu with it. Linux has no
  share menu: the same button saves the picture ("Save Image…", the file dialog).
- **Entries.** "Reading stats" in the library's "⋯" menu (also in an empty library) and the key `s` in the library;
  and the end page.
- **The end page's totals.** Under "Finished": the book's time by the log of all devices, the days since it was first
  opened on this computer, its highlights and bookmarks ("6 h · 12 days · 7 highlights"); under them the current month
  ("October: 3 books · 14 h"), which opens the stats on that month — a click, or `Tab` and `Enter` like a link.
- **One reading, one book.** A book finished on two devices within 30 days, or marked approximately on both, counts
  once (the rule the Android app's `sync-reading-log` adopts), so the phone and the computer show the same numbers.
- No new setting.

## Non-goals

- A week or all time, streaks, goals, reminders.
- Covers in the stats or the picture, a theme for the picture other than the reader's.
- The Android end page's other parts (the closing animation, "Share highlights", "Back to library"), and a share menu on
  Linux.
- Which device the reading was on.

## Capabilities

### New Capabilities
- `reading-stats`: statistics for a month and a year, the window, the reading calendar, the picture.

### Modified Capabilities
- `reader-navigation`: the end page's totals and the month that opens the stats.
- `library`: "Reading stats" in the library's menu and the key `s`.

## Impact

Core: a new `core/src/stats.rs` (the period's numbers, the calendar's geometry, the texts), the end page in
`core/src/layout/page.rs` (two lines and a link to the stats), the catalogue, `core/src/ffi.rs` and `macos/forzac.h`.
macOS: a new `macos/Stats.swift` (the window, the calendar, the picture and sharing), `macos/Library.swift`,
`macos/main.swift`, `macos/PageView.swift`. GTK: a new `gtk/src/stats.rs`, `gtk/src/library.rs`, `gtk/src/main.rs`,
`gtk/src/page_view.rs`. Docs: `docs/android-parity.md` (M7), the help's keys.
