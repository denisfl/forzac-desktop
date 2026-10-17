## MODIFIED Requirements

### Requirement: When the state is written
The position SHALL be written when a book is opened (so it is in the library even if closed at once), 1 s after the
last change and when the window closes; never while a page-turn animation runs. A write SHALL replace the file atomically and SHALL keep what other Forzac processes wrote (it re-reads the file
and changes only its own entries). The measurement flags (`--measure`, `--png`, `--turns`, `--chapter`, `--page`) SHALL
neither read nor write the state of a book, so measurements do not move the reader's place; without a book they show
the library from the state without writing it.

#### Scenario: Turning several pages
- **WHEN** the reader turns ten pages in quick succession and stops
- **THEN** the state is written once, a second after the last turn

## REMOVED Requirements

### Requirement: Recent books
**Reason**: Replaced by the library (`library`), which shows every remembered book with its cover, not up to 20 in a
list.
**Migration**: The same state entries feed the library; macOS keeps File → Open Recent.

## ADDED Requirements

### Requirement: Forgetting a book
The state SHALL forget a book on request (removing it from the library): its entry with the reading position goes at
the next write, its bookmarks and highlights stay, so opening the file again brings them back. Every remembered book
whose file exists SHALL be listed for the library, the last opened first.

#### Scenario: Forgetting and reopening
- **WHEN** a book with two bookmarks is removed from the library and its file is opened again
- **THEN** it opens at its start, with both bookmarks
