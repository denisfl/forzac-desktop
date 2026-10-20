## ADDED Requirements

### Requirement: Edits are kept
A book's edited details (title, authors, series and number, genres) SHALL be kept in the state by the book's id, only
the fields that differ from the book's own, written when saved; other processes' writes SHALL survive as for positions.
Removing a book from the library SHALL keep its edits.

#### Scenario: After a restart
- **WHEN** a book's title is edited and Forzac is restarted
- **THEN** the library shows the edited title
