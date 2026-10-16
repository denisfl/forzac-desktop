## Purpose

What Forzac remembers between launches — the reading position of every book, the recent books and the reader
settings — where it is kept and when it is written.

## ADDED Requirements

### Requirement: Reading position per book
The reading position (a Locator) SHALL be remembered for every book opened and restored when the book is opened again,
whatever the window size and settings are then. A book SHALL be recognised by its content (its size and a hash of its
first and last 64 KiB), so a renamed or moved file keeps its position; a book whose content is not known but whose path
is SHALL take the position remembered for that path. A remembered position outside the book (the file changed) SHALL
fall back to the nearest valid place.

#### Scenario: Reopening a book
- **WHEN** a book is closed on page 37 of chapter 5 and opened again
- **THEN** it opens on the same page, at the same first line

#### Scenario: A moved file
- **WHEN** the book file is moved to another folder and opened from there
- **THEN** it opens where it was left

### Requirement: When the state is written
The position SHALL be written 1 s after the last change and when the window closes; never while a page-turn animation
runs. A write SHALL replace the file atomically and SHALL keep what other Forzac processes wrote (it re-reads the file
and changes only its own entries). The measurement flags (`--measure`, `--png`, `--turns`, `--chapter`, `--page`) SHALL
neither read nor write the state, so measurements do not move the reader's place.

#### Scenario: Turning several pages
- **WHEN** the reader turns ten pages in quick succession and stops
- **THEN** the state is written once, a second after the last turn

### Requirement: Recent books
Running Forzac without a file SHALL show the recent books: up to 20 books whose files still exist, the last opened
first, each with its title, authors and how far it has been read (in percent). `j`/`k` or `↓`/`↑` move the selection,
`Enter` opens the selected book where it was left, `q` or `Esc` closes the list. With no books yet, the list says how
to open one.

#### Scenario: Continue reading
- **WHEN** Forzac is started without arguments after reading two books
- **THEN** the last book read is selected first, and `Enter` opens it at its remembered page

### Requirement: Reader settings
The theme and the text size SHALL be remembered globally and applied to every book opened later. Settings chosen in
`reader-settings` use the same storage.

#### Scenario: A theme for the evening
- **WHEN** the reader switches to the dark theme and opens another book the next day
- **THEN** the other book opens in the dark theme

### Requirement: Storage
The state SHALL be kept in one file: `$XDG_DATA_HOME/forzac/state` on Linux (`~/.local/share/forzac/state` by
default) and `~/Library/Application Support/Forzac/state` on macOS. A missing, unreadable or partly damaged file SHALL
never prevent opening a book: unreadable entries are ignored. Reading or writing the state of 1000 books SHALL take at
most 10 ms on a Raspberry Pi 5.

#### Scenario: A damaged state file
- **WHEN** the state file contains garbage lines
- **THEN** books open normally, the valid entries are kept and the garbage is dropped at the next write
