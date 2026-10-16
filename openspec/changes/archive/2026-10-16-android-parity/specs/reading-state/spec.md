## MODIFIED Requirements

### Requirement: Recent books
Running Forzac without a file, and closing the last book (`reader-window`), SHALL show the recent books: up to 20
books whose files still exist, the last opened first, each with its title, authors and how far it has been read (in
percent; a finished book reads 100 %), in the reader's theme and interface font. In both shells `j`/`k` or `↓`/`↑`
move the selection, `Enter` (or a double click) opens the selected book where it was left, `q` or `Esc` closes the
list. With no books yet, the list says how to open one.

#### Scenario: Continue reading
- **WHEN** Forzac is started without arguments after reading two books
- **THEN** the last book read is selected first, and `Enter` opens it at its remembered page

#### Scenario: A dark evening
- **WHEN** the reader's theme is Wave and the recent books are shown
- **THEN** the list is drawn in Wave's paper, ink and secondary colours
