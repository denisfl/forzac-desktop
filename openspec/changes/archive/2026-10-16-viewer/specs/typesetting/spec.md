## MODIFIED Requirements

### Requirement: Note markers and links
Note markers SHALL be set as superscripts. Text of a link SHALL be drawn in the theme's link colour. Every page (and
every stretch of the "Scroll" column) SHALL report the areas of its note markers and links with their targets, of its
images at least a third of the text column wide with the image, and of its tables with the table — one area per table
around its rows on that page, listed before the areas inside its cells — so a shell can activate them by key or click.
Notes and link previews report markers and links only.

#### Scenario: A page with a note
- **WHEN** a page contains the marker of note `n1`
- **THEN** the page lists an area around the marker with target note `n1`

#### Scenario: A page with a table
- **WHEN** a page holds the last three rows of a table and a cell has a link
- **THEN** the page lists one area around the three rows with the table as its target, then the link's area

## ADDED Requirements

### Requirement: A table for the view
The core SHALL typeset a table for the view at a given text size: columns by content without fitting them to the
column — each at least as wide as its longest word and at most 20 em, as wide as its longest line within that — with
the page's gap between columns, cell padding, line spacing, rules, bold header cells, note markers and links; it
reports the table's width and height, its drawing and the areas of its markers and links. On a Raspberry Pi 5 a table
of 100 rows SHALL be typeset for the view in at most 50 ms.

#### Scenario: A column of long text
- **WHEN** a table for the view has a column whose cells hold a sentence of 60 em
- **THEN** that column is 20 em wide and the sentence wraps inside its cells
