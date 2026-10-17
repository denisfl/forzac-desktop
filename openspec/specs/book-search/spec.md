# book-search Specification

## Purpose
TBD - created by archiving change book-search. Update Purpose after archive.

## Requirements

### Requirement: Matching
Search SHALL find the query in the book's paragraphs, headings and table cells, ignoring case and the difference
between "ё" and "е"; a soft hyphen in the text does not interrupt a match, a non-breaking space equals a space, and a
space in the query matches any run of white space. A match does not run from one cell into the next. Going to a match
in a table SHALL highlight it in its cell. A query shorter than two characters (spaces aside) finds nothing. Notes are
not searched.

#### Scenario: ё and case
- **WHEN** the book has "Ещё немного." and "ЕЩЕ раз и еще." and the reader searches "еще"
- **THEN** three matches are listed in the order of the book

#### Scenario: A word only in a table
- **WHEN** the reader searches a word that is only in a table cell and goes to the match
- **THEN** the page with that table row is shown, the word highlighted in its cell

### Requirement: The search panel
`/` (on macOS also `⌘F`) SHALL open a panel with the search field focused, laid out like the contents panel. Matches
appear as the reader types: their count, then for each a snippet (up to 40 characters before and 80 after, cut at
words, "…" where cut) with the match in bold, and the section and the book page ("section · p. N") once the book's
pages are counted (`reader-menu`), before that the section and the position in the book in percent. `↓` moves into the
list, `j`/`k` move in it, `Enter` or a click goes to a match and closes the panel; `Esc` clears the field or closes the
panel.

#### Scenario: Finding a name
- **WHEN** the reader presses `/`, types "Пепперберг" and presses `Enter`
- **THEN** the book opens at the first match, with the match highlighted

#### Scenario: Pages of matches
- **WHEN** the book's pages are counted and the reader searches a word
- **THEN** each match shows its section and "p. N", the page it is on

### Requirement: Moving between matches
After going to a match, `n` and `N` SHALL go to the next and previous match of the same search (across chapters, not
past the ends), each highlighted on the page; `Esc` removes the highlight. Going to a match is remembered for `Ctrl+O`.

#### Scenario: The next match
- **WHEN** a match is highlighted and the reader presses `n`
- **THEN** the page with the next match is shown with that match highlighted

### Requirement: Speed
Searching a whole book SHALL take under 100 ms on a Raspberry Pi 5 for a 1 MB text and never block a frame.

#### Scenario: A long book
- **WHEN** the reader searches a common word in the longest book of the corpus
- **THEN** the results appear within 100 ms and page turns stay smooth meanwhile
