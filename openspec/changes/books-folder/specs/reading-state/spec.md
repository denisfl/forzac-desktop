## MODIFIED Requirements

### Requirement: Forgetting a book
The state SHALL forget a book on request (removing it from the library, which also moves its file to the Trash): its
entry with the reading position goes at the next write, its bookmarks and highlights stay, so a file with the same
content opened or put into the Books folder again brings them back. A remembered book's path SHALL be its file in the
Books folder; every remembered book whose file exists SHALL be listed for the library, the last opened first, and the
library adds the folder's books that were never opened.

#### Scenario: Forgetting and reopening
- **WHEN** a book with two bookmarks is removed from the library and its file is opened again
- **THEN** it opens at its start, with both bookmarks
