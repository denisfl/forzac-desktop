## Context

The state (`core/src/state.rs`) is one text file, read again and rewritten whole on every write under a lock; the shells
write it 1 s after the last move (`remember_position` in `gtk/src/main.rs`, `Reader.onMoved` in `macos/main.swift`),
when a book window loses focus or closes, and on quit. An entry keeps the place, the share read and `opened`, the last
time the book was read (seconds); nothing keeps the first time. Showing the end page saves the place as finished
(100 %). Sync (`core/src/sync.rs`) merges per item by time; the own file holds every item the device knows.

The Android app's log (its `reading-log`): sums per (book, local day) of seconds, characters (UTF-16 units) and pages,
counted by `ReadingTally` from the reader's events with explicit time; finishes with a copy of title and authors, the
first opening and an `approximate` flag, the 30-day rule. Its `sync-reading-log` (not yet implemented) adds the device
to the log and the two sync lines this change defines.

## Goals / Non-Goals

**Goals:** the desktop logs reading by the Android rules; the log and finishes sync with the phone in the format the
phone expects; no cost while turning pages; the rules are core code tested without a shell.

**Non-Goals:** showing the log (`reading-stats`); syncing first openings; time while the window is not active.

## Decisions

**1. The log is a file of its own beside the state (core).** `reading-log` in the state's directory (the state's file
name replaced), the state's escaping, header `forzac-log 1`, written as the state is: read again under a lock file
(`.reading-log.lock`), changed, replaced atomically.

```text
forzac-log 1
read	<book>	<time>	<device>	<day>	<seconds>	<chars>	<pages>
finish	<book>	<time>	<title>	<authors>	<started>	<approximate>
first	<book>	<time>
```

`read` and `finish` lines are the sync file's lines (decision 5), so the log is what sync merges, line for line; `first`
(when the book was first opened on this computer, milliseconds) stays here. *Why not in the state:* the state is
rewritten 1 s after every turn and read again before each write; with three devices a year of the log is about 2,200
lines (~130 KB) that every turn would then parse and write. The log is written at most once a minute while reading.

**2. Counting is a pure core type (`log::Tally`).** A port of the Android `ReadingTally` with time in milliseconds and
the time zone as the offset east of UTC that the shell passes with each event (GTK `glib::DateTime::now_local`, macOS
`TimeZone.current.secondsFromGMT()`): `active(now)`, `moved(now, chars)`, `jumped(chars)`, `paused(now)`, `drain()`.
The gap since the last event counts up to 5 minutes, split at local midnight; after `paused` the next gap does not
count; a forward move counts the characters between the places, at most 3000 a minute of the time counted since the
last counted move; a jump only moves the anchor. Characters are code points: `log::Reading` (one per open book) keeps
the tally and the book's block starts in code points, made on the first move, and turns a `Locator` into characters.
`pages` is always 0.

**3. The shells feed it.** Events: any key press, click or scroll in a book window → `active`; a move by turning,
scrolling, `line`/`half` steps → `moved(locator)`; a jump (contents, link, search match, bookmark, highlight, `[` `]`,
`g` `G`, "back", the menu's slider) → `jumped(locator)`; the window losing focus or closing, and quitting → `paused`.
The shells already funnel moves through one callback (`connect_moved`, `onMoved`); the jumping functions set a flag the
callback reads. GTK keeps the `Reading` beside its `PageView` in `main.rs`; macOS keeps it in the core's `Document`
behind `forzac_reading_event(doc, kind, at, offset)` (the core reads the clock).

**4. Written with the place (core, shell's thread).** The log is appended when the shells save the place: the delayed
save 1 s after a move appends if a minute has passed since the last append; focus loss, closing and quitting always
append (`log::append(store, days)` adds each drained day to this device's line for that book and day,
`time = max(now, time + 1)`). Nothing is written during a turn, and at most a minute of reading is lost if the process
dies. The device id is the sync id (`sync.device` in the state), now made by the first append if sync has not made it.

**5. Two sync lines (core).** `read` — key (device, book, day), `5` in the record keys; the line of the latest `<time>`
wins; a line with this device's id from another file is never applied (only this device changes its days).
`finish` — key (book, time), `6`; never changed; added when missing. Sync puts every log line into the own records
(the log wins a tie), adopts the others' newer ones, applies those that are not this device's to the log in one write,
and writes them all into the own file. `<day>` is `yyyy-mm-dd`; `<started>` is `-` or milliseconds; `<approximate>`
`1`/`0`; title and authors are escaped like every field. Readers of `forzac-sync 1` skip unknown lines, so older
versions lose nothing but these lines.

**6. Finishes (core).** `log::finish(store, book, now)` when the end page is saved finished: no line is added when the
log has a finish of the book within the 30 days before `now` (from any device). `started` is the book's `first`, if
any. Re-reading after 30 days adds one more.

**7. The first log (core).** When the log file does not exist, the first access makes it with an approximate finish
for every state book read to 100 % (`progress >= library::FINISHED`), dated its `opened`, started unknown. A copy of
the state (another Mac) makes the same lines, which the stats count once (`reading-stats`).

## Risks / Trade-offs

- [Reading with the window not active is not counted] → rare on a desktop: the book window is where the keys go.
- [A process killed mid-reading loses up to a minute] → accepted, as on Android.
- [Characters differ from the phone's outside the BMP (UTF-16 vs code points)] → the stats show no characters.
- [The approximate finishes of the books read before the log are dated their last opening] → marked approximate; the
  stats count one reading once (`reading-stats`).
- [The log file grows ~60 KB a year per device] → read whole only by the stats, sync and the end page, off turning.

## Migration Plan

Nothing in the state changes. The first run of this version makes `reading-log` with the approximate finishes;
removing the file starts the log again (with the same finishes). Older versions ignore the file and the new sync lines.
