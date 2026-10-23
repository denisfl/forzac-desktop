## Context

`reading-log` gives `reading-log` beside the state: `read` lines (device, book, day, seconds) of every device, `finish`
lines with title, authors and `approximate`, `first` openings of this computer. The Android app's screen
(`StatsScreen.kt`), its picture (`StatsImage.kt`) and its rules (`StatsCalculator.kt`, `CalendarGrid.kt`,
`CalendarLayout`) are the reference. The library is native in both shells (Golos Text for the interface, the book
serifs for headings, the reader's theme: paper, ink, muted, line, sheet, link); the settings are a window of their own
on macOS and a sheet over the library on Linux. The end page is a core page (glyph runs and areas) drawn by the shells'
page drawing; its areas are what `Tab` and clicks reach.

## Goals / Non-Goals

**Goals:** the phone's numbers, calendar and picture on the desktop; one place for the rules and the geometry (the
core, tested); keyboard first; nothing drawn while idle.

**Non-Goals:** a week or all time, streaks, goals; covers; the Android end page's animation and buttons.

## Decisions

**1. The numbers are core code (`core/src/stats.rs`).** `Period::{Month(y, m), Year(y)}`; `compute(period, log, offset,
today)` sums the log's `read` lines of every device by day: time, days with at least a minute (the devices together),
minutes per day; books — the counted finishes of the period, the latest first, one per book. *Counted* (shared with the
Android app's `sync-reading-log`, decision 5): going through a book's finishes by time, a finish is not counted when a
counted finish of the book was less than 30 days before it, and an approximate one is not counted either when the
book has an earlier finish or an exact one less than 30 days after it. `level(minutes)`: 0, 1–14, 15–29, 30–59, 60+.
`range(log)`: the first month with a day or a finish, to today. Local dates come from the offset the shell passes (as
`sync::when`).

**2. The geometry is core code too.** A port of `CalendarGrid` (the week starts on Monday in Russian, Sunday in
English, by the interface language) and `CalendarLayout`: for a period, a width and a scale, the cells (date and
rectangle), the labels (weekday or month names, rectangle, centred or not), the corner radius, whether cells hold day
numbers, the height. Month: 7 columns, gap 6, header 22, radius 8; year: `columns` months a row (3 in the window, 4 in
the picture), gap 2, column gap 20, row gap 18, header 20, six weeks each, radius 2 — in points, times `scale`. The
shells only draw it: rounded rectangles in `line` (level 0) or `link` at 30 / 50 / 75 / 100 %, today outlined in ink,
future days not filled (their numbers in muted at 60 %), numbers on levels 3–4 in paper.

**3. The texts are core code.** Through the catalogue (`tr`, `tr_count`), so both shells and the end page say the same:
the time ("35 min" under an hour, else whole hours rounded, "8 h"), the figures' captions with plural forms ("books
finished", "days with a book"), the period ("October 2026", "2026"), the day ("18 October · 52 min" / "· not read"),
the weekday and month names (Russian nominative for titles, genitive for dates), the month line ("October: 3 books ·
14 h"), the book's line ("6 h · 12 days · 7 highlights · 2 bookmarks", zero parts left out). Strings and plural rules
are the Android app's.

**4. The window is native in each shell.** macOS: `macos/Stats.swift`, a window of its own titled "Reading stats",
resizable from 420 × 560 points, brought forward when open, a flipped document view in a scroll view. Linux:
`gtk/src/stats.rs`, a window of its own transient for the window it was opened from (a dialog floats in tiling
compositors), the same parts in a `ScrolledWindow`, the calendar a `DrawingArea`-like widget drawn through
`gtk::Snapshot`. Top to bottom, as Android's screen within a column of at most 560 points: the title ("Reading stats",
serif bold 26) with the share button; "Month · Year" (the settings' segmented look); the period in serif 21 between
`‹` and `›` (inactive in line colour at the bounds); the three figures (serif 22, captions Golos 13 muted, widths
1.5 : 1 : 1); the calendar (cell numbers Golos 13, labels Golos 11 muted); the day line (Golos 14 muted, 40 high);
"Finished" (serif bold 19) and the books (title serif 17, "authors · 26 September" Golos 13 muted) or "What you finish
will appear here". Keys: `←`/`h` and `→`/`l` the neighbour periods, `m` month, `y` year, `s` share (macOS) or save
(Linux), `↑` `↓` `j` `k` `Space` `PageUp` `PageDown` `Home` `End` scroll, `Esc` (and `⌘W` on macOS) closes; also in a
Russian layout. A click on a past day shows its line. The data are read when the window opens and when it comes to the
front (the log is small: a year on three devices ~ 2,200 lines, parsed in ~1 ms); a period change computes on the main
thread (a year: 365 days, well under a frame).

*Why a window and not a sheet over the library:* the stats open from the library and from a book's end page; a window
of its own closes back to either without either knowing about it, as the settings window does on macOS.

**5. The picture is drawn by the shell from the core's layout.** 1080 × 1920, Android's `StatsImage` composition:
margins 88; the month's name in Literata 150 and the year under it in Literata 56 muted, or the year alone in 220; the
calendar at scale 2.5 (year: 4 months a row); the three figures in Literata (one size fitting all three, at most 96)
with Golos 38 captions; "Forzac"/"Форзац" Literata 44 muted, centred at the bottom; calendar and figures centred
between. macOS draws it into a `CGContext` with Core Text and passes the PNG (in the temporary directory,
`forzac-2026-10.png`) to `NSSharingServicePicker` at the share button. GTK draws it with cairo and Pango into an image surface and saves
it as a texture where the file dialog says (default `~/Pictures/forzac-2026-10.png`). No links, covers or caption text.

**6. The end page's totals (core).** `end_page` takes two optional lines: the book's (`stats::book_line`) under
"Finished" in the secondary ink, and the month's (`stats::month_line`) under it in the link ink with an area whose
target is `Target::Stats`, so a click and `Tab`/`Enter` reach it like a link; the shells open the stats window on the
current month. The book's time is shown only when the log's first day (any device) is not later than the day the book
was first opened here; the days only when the first opening is known (from today back to it, inclusive). The shells
fill the lines when the end page is drawn anew (GTK: `PageView::connect_end_lines`; macOS: `forzac_state_end_lines`
before `forzac_end_page`), reading the log once; the month counts the book as finished now (`Log::as_if_finished`), as
its finish is written with the place a second later.

**7. The library's menu.** "Reading stats" between "Add a Folder…" and "Settings…" in the head's "⋯" menu (the
settings' section), also in an empty library; `s` in the library opens it.

## Risks / Trade-offs

- [The window is far from the reading: no one opens a menu item] → the end page's month line is the main entry, as on
  Android.
- [Two shells draw the calendar and the picture twice] → the geometry, rules and texts are shared in the core; only
  rectangles and text are drawn per shell; screenshots of both.
- [The Android app counts one reading once only after its `sync-reading-log`] → until then a book finished on both
  devices within 30 days counts once here and may count twice there.
- [Light themes' level 1 (30 % link on line) is faint] → the phone's six themes passed; checked here in light and dark.

## Migration Plan

None: the stats read `reading-log`.
