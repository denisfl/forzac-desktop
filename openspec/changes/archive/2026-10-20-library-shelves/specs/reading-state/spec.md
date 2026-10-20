## ADDED Requirements

### Requirement: Shelves are kept
The shelves a book is on SHALL be kept in the state by the book's id, written when they change; other processes'
writes SHALL survive as for positions. Removing a book from the library SHALL keep its shelves, as it keeps its
bookmarks and highlights.

#### Scenario: Put back from the Trash
- **WHEN** a book on "Летом" is removed from the library and its file is put back into the Books folder
- **THEN** the book is on "Летом" again
