## ADDED Requirements

### Requirement: Editing a book's details
"Edit…" ("Изменить…") in the book's menu SHALL read the book's file off the UI thread and then show a form "Edit the
book" with the fields Title, Authors, Series, Number, Genres (authors and genres separated by commas), filled with the
details as the library shows them, and the buttons "From the book", "Cancel" and "Save". "Save" (`Enter`) SHALL keep,
for each field, the value typed when it differs from the book's own: an empty title or authors SHALL mean the book's
own, an empty series SHALL mean no series, a number that is not a number SHALL be left out. "From the book" SHALL drop
all the book's edits; "Cancel" (`Esc`) SHALL change nothing. The library SHALL then show, sort, group and search the
book by its edited details, and "About the book" SHALL show them. The book's file SHALL NOT be changed.

#### Scenario: Fixing an author
- **WHEN** the reader opens "Edit…" on a book by "Clear, James", types "Джеймс Клир" in Authors and saves
- **THEN** the book shows "Джеймс Клир" under its cover, the Author view lists it under "Джеймс Клир", a search for
  "клир" finds it, and its file is byte for byte the same

#### Scenario: Back to the book's own
- **WHEN** the reader opens "Edit…" on that book again and presses "From the book"
- **THEN** the book shows the author from its file again

## MODIFIED Requirements

### Requirement: The book's menu
A "⋯" button SHALL appear next to a book's title under the pointer and on the selected book; a click on it, or a
right click on the book, SHALL open a menu with "About the book", "Edit…" (`Editing a book's details`), "Add to shelf"
(`Shelves`) and "Remove from library". `Delete` and `d` SHALL still ask to remove the selected book.

#### Scenario: Removing from the menu
- **WHEN** the reader clicks "⋯" under a book and chooses "Remove from library"
- **THEN** the library asks "Remove “title” from the library?" as `Removing a book` says
