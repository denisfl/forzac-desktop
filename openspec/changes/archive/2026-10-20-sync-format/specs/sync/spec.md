## ADDED Requirements

### Requirement: The sync folder and files
Forzac SHALL keep its sync data in `.forzac/` inside the Books folder: one file per device, `<device>.sync`, where
`<device>` is 16 hex digits made once and remembered in the state, in the format of `docs/sync-format.md`. A device
SHALL write only its own file, replacing it atomically, and SHALL read every `.sync` file there; unreadable lines and
files SHALL be skipped.

#### Scenario: Two computers in one Dropbox
- **WHEN** the Mac and the Pi 5 use the same Books folder in Dropbox
- **THEN** `.forzac/` holds two `.sync` files, one written by each, and no conflicted copies appear

### Requirement: What syncs and how it merges
The reading position, bookmarks, highlights, shelves and edits of book details SHALL sync. Every item SHALL carry the
time of its last change; for each item the latest change across the files SHALL win; a removed bookmark, highlight or
shelf SHALL be kept as a removed item so the removal reaches the other devices. Local changes since the last sync SHALL
be found by comparing the state with the device's own file, a position's time being when the book was last read.

#### Scenario: A bookmark removed on one device
- **WHEN** a bookmark made on the Mac reaches the Pi 5, and later the Pi 5 removes it
- **THEN** after both libraries have shown once more, neither has the bookmark

#### Scenario: Read further on the other computer
- **WHEN** the reader reads chapter 3 on the Mac, and then opens the library on the Pi 5, where the book was left in
  chapter 1
- **THEN** the book on the Pi 5 opens in chapter 3 at the Mac's place

### Requirement: Places across apps
A place SHALL travel as chapter, block, offset in Unicode code points and up to 24 code points of text from it. Applying
a remote place SHALL use the block at that chapter and index when its text at the offset starts with the fragment, else
the fragment's first occurrence searched outwards from that block, else the start of that block, clamped to the book.

#### Scenario: Blocks numbered differently
- **WHEN** a place arrives whose block index is one off from this app's numbering of the same chapter
- **THEN** the book opens at the paragraph whose text starts with the fragment

### Requirement: When it syncs
Forzac SHALL sync each time the library shows, off the UI thread, before reading the Books folder; the measurement
flags SHALL NOT sync. Data for a book whose file is not in the Books folder yet SHALL be kept and applied when the file
arrives.

#### Scenario: The book arrives later
- **WHEN** the sync data for a book arrives before its file and the file arrives a minute later
- **THEN** the next time the library shows, the book opens at the synced place with its bookmarks
