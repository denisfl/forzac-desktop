## MODIFIED Requirements

### Requirement: Reading position per book
The reading position (a Locator) SHALL be remembered for every book opened and restored when the book is opened again,
whatever the window size and settings are then. A book SHALL be recognised by its content — the first 16 bytes of the
SHA-256 of its file, as 32 lowercase hex digits, the id the Android app gives the same file — so a renamed or moved
file keeps its position; a book whose content is not known but whose path is SHALL take the position remembered for
that path. A remembered position outside the book (the file changed) SHALL fall back to the nearest valid place.

#### Scenario: Reopening a book
- **WHEN** a book is closed on page 37 of chapter 5 and opened again
- **THEN** it opens on the same page, at the same first line

#### Scenario: A moved file
- **WHEN** the book file is moved to another folder and opened from there
- **THEN** it opens where it was left

#### Scenario: The same id as on Android
- **WHEN** the same `.fb2.zip` file is in the Android app and in the Books folder
- **THEN** both apps give it the same id

## ADDED Requirements

### Requirement: Moving the state to the new ids
On the first start of a version with SHA-256 ids, every remembered book, bookmark list, highlight list and cached
Books folder file whose file can be read SHALL be renamed from its old id (FNV-1a 64 over the file's length and its
first and last 64 KiB) to the new one, once, keeping positions, progress, bookmarks and highlights. Data of a book whose
file cannot be read SHALL keep its old id; when a file whose old id holds data appears in the Books folder later, that
data SHALL be renamed to the file's new id the next time the library reads the folder. The move SHALL run off the UI
thread, with the Books folder (`books-folder`).

#### Scenario: Updating from 0.11.0
- **WHEN** a reader with two books, one with three bookmarks and a highlight, starts this version
- **THEN** both books are in the library with their progress, and the bookmarked one opens where it was left with its
  three bookmarks and its highlight

#### Scenario: A book in the Trash during the update
- **WHEN** a book with two highlights was removed to the Trash in 0.11.0, this version starts, and then the file is
  put back into the Books folder
- **THEN** the library shows the book, and opening it shows both highlights
