## Why

The Android app keeps a reading log since 0.9.6 — how long each book was read on each day, and when it was finished —
and in 0.9.10 shows it as reading statistics (its `reading-log`, `reading-stats`). The desktop app keeps nothing of the
kind: a book's place and share read, and when it was last opened. So the desktop cannot show statistics (`reading-stats`,
the next change), and a month read half on the Mac and half on the phone is half a month on either.

Decided with the maintainer (2026-10-22): the numbers are shared with the phone. The desktop logs its own reading, and
the log travels through the sync folder (`sync`) in two new lines of the sync file; the Android app reads and writes the
same lines (its `sync-reading-log`, which refers to this change for the format). The desktop does not write these lines
today and skips them when they come.

No setting: the log is kept whenever books are read, and goes to the sync folder whenever sync does.

## What Changes

- **A reading log on this computer.** For each book and local day: seconds read, characters read (code points), pages
  (always 0 here: the desktop has no PDF or picture books). Time counts while a book window is the active window, the
  gap between two of the reader's actions counted up to 5 minutes; characters count only going forward by turning or
  scrolling, up to 3000 a minute. The Android app's rules, the same thresholds.
- **Finishes.** Showing a book's end page records it finished — the date, the title and authors, when it was first
  opened on this computer — unless the same book was finished less than 30 days before on any device whose finish has
  arrived. When the log starts, the books already at 100 % get a finish dated their last opening, marked approximate.
- **First opening.** The log remembers when each book was first opened on this computer (from now on; earlier books
  are unknown), for the end page's days and time.
- **A file of its own.** The log is kept beside the reading state in `reading-log`, not in the state: the state is read
  and rewritten after every page turn, the log at most once a minute.
- **Sync.** Two new lines in the sync file (`docs/sync-format.md`): a day of the log,
  `read <book> <time> <device> <day> <seconds> <chars> <pages>`, keyed by device, book and day and changed only by its
  device; a finish, `finish <book> <time> <title> <authors> <started> <approximate>`, keyed by book and time, never
  changed. Each device writes into its own file every line it knows, its own and the others'.
- The settings' sync explanation names reading statistics among what is the same on every device.

## Non-goals

- Showing the log: statistics, the end page's totals and the library's menu are `reading-stats`.
- Syncing the first opening, deleting the log, exporting it, a session list (starts and ends of reading).
- Counting time while a book window is open but not active (another app in front, the screen locked).

## Capabilities

### New Capabilities
- `reading-log`: the log of reading by day, finishes and first openings on this computer.

### Modified Capabilities
- `sync`: the `read` and `finish` lines, their keys and merging.
- `settings`: the sync explanation names reading statistics.

## Impact

Core: a new `core/src/log.rs` (the file, the tally, finishes), `core/src/sync.rs` (the two lines, merging into the
log), `core/src/ffi.rs` and `macos/forzac.h` (the reading events and flush). Shells: the reader's input, turns, jumps,
focus and closing feed the tally (`gtk/src/main.rs`, `gtk/src/page_view.rs`, `gtk/src/scroll.rs`, `macos/main.swift`,
`macos/PageView.swift`); the end page records a finish. `docs/sync-format.md`, `docs/android-parity.md`.
