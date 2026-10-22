## MODIFIED Requirements

### Requirement: What syncs and how it merges
The reading position, bookmarks, highlights, shelves and edits of book details SHALL sync, and so SHALL the reading log
and the finishes (`reading-log`), of every book, also books no longer in the library. Every item SHALL carry the time of
its last change; for each item the latest change across the files SHALL win; a removed bookmark, highlight or shelf
SHALL be kept as a removed item so the removal reaches the other devices. Local changes since the last sync SHALL be
found by comparing the state with the device's own file, a position's time being when the book was last read. The time
of a local change SHALL be at least one millisecond later than the time of the item it replaces in the own file,
whatever the device's clock says; a position whose place in the own file was applied on this device and differs from
the book's place SHALL be a local change.

A day of the log SHALL be the line `read <book> <time> <device> <day> <seconds> <chars> <pages>` keyed by device, book
and day; only the device that read SHALL change it: a line with this device's id from another file SHALL NOT be
applied. A book's reading on a day SHALL be the sum over devices. A finish SHALL be the line
`finish <book> <time> <title> <authors> <started> <approximate>` keyed by book and time; it SHALL NOT change, and
another device's finish SHALL be added to the log when the log lacks it. A device SHALL write into its own file every
day and finish it knows, its own and the others'.

#### Scenario: A bookmark removed on one device
- **WHEN** a bookmark made on the Mac reaches the Pi 5, and later the Pi 5 removes it
- **THEN** after both libraries have shown once more, neither has the bookmark

#### Scenario: Read further on the other computer
- **WHEN** the reader reads chapter 3 on the Mac, and then opens the library on the Pi 5, where the book was left in
  chapter 1
- **THEN** the book on the Pi 5 opens in chapter 3 at the Mac's place

#### Scenario: A clock behind
- **WHEN** the phone's clock is an hour ahead, its place reaches the Mac, and the reader then reads further on the Mac
- **THEN** the Mac's new place wins on every device

#### Scenario: One day on two devices
- **WHEN** the reader reads a book for 20 minutes on the Mac and 30 minutes on the phone on one day, and both sync
- **THEN** the Mac's log and both devices' files hold the Mac's 20 minutes and the phone's 30 for that day, 50 in all

#### Scenario: Read more the same day
- **WHEN** the Mac's day already went to the folder with 30 minutes, and the reader reads 10 more minutes on the Mac
  and syncs again
- **THEN** the Mac's file holds 40 minutes for that day, and so does the Pi 5's after its sync — not 70

#### Scenario: A device gone
- **WHEN** the Pi 5's days reached the Mac, and then the Pi 5's file is deleted from the folder
- **THEN** the Pi 5's days stay in the Mac's log and in the Mac's file

#### Scenario: A finish from the phone
- **WHEN** the reader finishes a book on the phone and the Mac syncs
- **THEN** the Mac's log holds the phone's finish once, however many syncs follow
