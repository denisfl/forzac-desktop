## ADDED Requirements

### Requirement: The log by day
Forzac SHALL keep a reading log: for each (device, book, local day) the seconds read, the characters read (Unicode code
points) and the pages read (always 0 on this computer). This computer SHALL log its own reading under its device id
(`sync`), whether or not the Books folder syncs; other devices' lines SHALL come only through sync. Reading across local
midnight SHALL be split between the two days. The log SHALL be kept in `reading-log` beside the reading state, not in
the state, and SHALL NOT leave the computer except through the sync folder (`sync`). The log SHALL add no setting.

#### Scenario: An evening of reading
- **WHEN** the reader reads a book for 40 minutes, turning pages
- **THEN** the log holds about 40 minutes and the characters of the pages read for that book and day, under this
  computer's id

#### Scenario: Past midnight
- **WHEN** the reader reads from 23:50 to 00:10
- **THEN** about 10 minutes are logged on the first day and about 10 on the second

### Requirement: Time read
Time SHALL count while a book window is the active window. The gap between two of the reader's actions in it (a key
press, a click, a scroll step, a page turned) SHALL count in full, but at most 5 minutes. Time SHALL NOT count while
another window or app is active, after the book window closes, or while the library is shown.

#### Scenario: A slow page
- **WHEN** the reader reads one page for 3 minutes and presses `Space`
- **THEN** 3 minutes are logged

#### Scenario: Walked away
- **WHEN** the reader leaves the book open and active and comes back after 40 minutes
- **THEN** at most 5 minutes of those 40 are logged

#### Scenario: Another app
- **WHEN** the reader switches to the browser for an hour and back to the book
- **THEN** the hour is not logged

### Requirement: Text read
Characters SHALL count only going forward by turning pages (in a spread, by the screen of two pages) or scrolling: the
text between the previous and the new place. A jump — a contents entry, a link, a search match, a bookmark or
highlight, `[`, `]`, `g`, `G`, "back", the menu's slider — SHALL NOT count and SHALL start counting from its target.
Going back SHALL neither add nor remove. What counts in a gap SHALL NOT exceed 3000 characters a minute of the time
counted since the last counted move.

#### Scenario: Turning forward
- **WHEN** the reader reads a page for 2 minutes and presses `Space`
- **THEN** that page's characters are logged

#### Scenario: Through the contents
- **WHEN** the reader, halfway through chapter 2, goes to chapter 10 from the contents
- **THEN** chapters 2–9 are not logged; counting goes on from the start of chapter 10

#### Scenario: Flipping through
- **WHEN** the reader turns 15 pages forward in 20 seconds
- **THEN** at most 1000 characters are logged

### Requirement: Finishes
Showing a book's end page SHALL record a finish: the book, the date and time, its title and authors, and when it was
first opened on this computer, if known. When the same book has a finish less than 30 days before — on this computer or
on a device whose finish has arrived through sync — no new finish SHALL be recorded. A finish 30 days later or more
SHALL be recorded again: a book read again. Finishes and the log SHALL stay when the book is removed from the library.

#### Scenario: Finishing
- **WHEN** the reader turns past the last page of a book
- **THEN** the log holds a finish of the book dated now

#### Scenario: Back to the end page
- **WHEN** the reader finished a book, turned back and forward to the end page again
- **THEN** there is no second finish

#### Scenario: Finished on the phone yesterday
- **WHEN** the phone's finish of a book arrived yesterday and today the reader turns to its end page on the Mac
- **THEN** there is no second finish

### Requirement: First opening
The log SHALL remember when each book was first opened on this computer, from the version with the log on. A book
opened before it SHALL have no first opening.

#### Scenario: A new book
- **WHEN** the reader opens a book never opened before
- **THEN** the log holds its first opening, now

### Requirement: The first log
When the log does not exist yet, Forzac SHALL make it with a finish for every book of the reading state read to 100 %,
dated its last opening and marked approximate. Places, shares read, bookmarks and highlights SHALL NOT change.

#### Scenario: Finished before the update
- **WHEN** the reader finished a book in 0.16 and starts the version with the log
- **THEN** the log holds an approximate finish of it dated when it was last read

### Requirement: No cost while reading
Logging SHALL NOT write anything while a page turns: the log SHALL be written with the delayed save of the place (at
most once a minute) and when the book window loses focus or closes, or Forzac quits; at most a minute of the log SHALL
be lost if the process dies. Turning 50 pages on the Pi 5 at 120 Hz SHALL miss no more frames than without the log.

#### Scenario: Turning with the log
- **WHEN** `scripts/pi-headless.sh` turns 50 pages on the Pi 5
- **THEN** the missed frames are as many as before the log
