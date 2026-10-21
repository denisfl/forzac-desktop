# sync Specification

## Purpose
TBD - created by archiving change sync-format. Update Purpose after archive.

## Requirements

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
be found by comparing the state with the device's own file, a position's time being when the book was last read. The
time of a local change SHALL be at least one millisecond later than the time of the item it replaces in the own file,
whatever the device's clock says; a position whose place in the own file was applied on this device and differs from
the book's place SHALL be a local change.

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

### Requirement: Places across apps
A place SHALL travel as chapter, block, offset in Unicode code points and up to 24 code points of text from it. Applying
a remote place SHALL use the block at that chapter and index when its text at the offset starts with the fragment, else
the fragment's first occurrence searched outwards from that block, else the start of that block, clamped to the book.

#### Scenario: Blocks numbered differently
- **WHEN** a place arrives whose block index is one off from this app's numbering of the same chapter
- **THEN** the book opens at the paragraph whose text starts with the fragment

### Requirement: When it syncs
Forzac SHALL sync off the UI thread each time the library shows, before reading the Books folder; when the app becomes
active; and when it is left: the app or a book window loses focus, a book window closes, the app quits (after saving
the open books' places). When a sync brings a newer place for an open book, its window SHALL go there and keep the
place left for going back. The measurement flags SHALL NOT sync. Data for a book whose file is not in the Books
folder yet SHALL be kept and applied when the file arrives.

#### Scenario: The book arrives later
- **WHEN** the sync data for a book arrives before its file and the file arrives a minute later
- **THEN** the next time the library shows, the book opens at the synced place with its bookmarks

#### Scenario: From the Mac to the phone
- **WHEN** the reader reads two chapters on the Mac and switches to another app without closing the book
- **THEN** the Mac's place is in its sync file at once, and the phone opens the book there

#### Scenario: From the phone to the Mac
- **WHEN** a book is open on the Mac, the reader reads further on the phone and comes back to the Mac
- **THEN** the open book goes to the phone's place, and "back" returns to where it was on the Mac

### Requirement: A copied device
When the own sync file exists and differs from what this device last wrote there, Forzac SHALL take a new device id
and write its own file under it; the old file then counts as another device's.

#### Scenario: Migration Assistant
- **WHEN** the reading state is copied to a new Mac and both Macs sync with one Books folder
- **THEN** after both have synced twice, `.forzac/` holds one file per Mac and no conflicted copies, and both have
  each other's reading
