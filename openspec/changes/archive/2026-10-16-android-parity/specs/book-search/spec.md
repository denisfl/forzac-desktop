## MODIFIED Requirements

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
