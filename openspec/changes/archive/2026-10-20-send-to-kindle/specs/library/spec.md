## MODIFIED Requirements

### Requirement: The book's menu
A "⋯" button SHALL appear next to a book's title under the pointer and on the selected book; a click on it, or a
right click on the book, SHALL open a menu with "About the book", "Edit…" (`Editing a book's details`), "Add to shelf"
(`Shelves`), "Send to Kindle" (`send-to-kindle`) and "Remove from library". `Delete` and `d` SHALL still ask to remove
the selected book.

#### Scenario: Removing from the menu
- **WHEN** the reader clicks "⋯" under a book and chooses "Remove from library"
- **THEN** the library asks "Remove “title” from the library?" as `Removing a book` says
