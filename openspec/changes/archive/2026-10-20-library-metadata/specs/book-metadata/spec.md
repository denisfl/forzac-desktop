## ADDED Requirements

### Requirement: Metadata read from a book
Besides the title, authors, language, annotation and cover, the core SHALL read from a book its series (a name and,
when given, a number, which may be fractional), its genres and its year:
- FB2: the first `title-info/sequence` (`name`, `number`); every `title-info/genre`, shown by its name in the interface
  language for the common genre codes (unknown codes are not shown); the year from `title-info/date` (its `value` or
  its text), else `publish-info/year`.
- FB3: `sequence` in the description (its title's `main` text and `number`); every `fb3-classification/subject` as a
  genre; the year from `written/date`.
- EPUB: `calibre:series` and `calibre:series_index` (EPUB 2), or a `belongs-to-collection` meta with its
  `group-position` (EPUB 3); every `dc:subject` as a genre; the year as the first four digits of the first `dc:date`.
Missing or malformed values SHALL be left empty, never stop the book from opening.

#### Scenario: An FB2 book in a series
- **WHEN** an FB2 has `<sequence name="Полдень, XXII век" number="2"/>`, `<genre>sf_social</genre>` and
  `<date value="1962-01-01">1962</date>`
- **THEN** its series is "Полдень, XXII век", number 2, its genre "Социальная фантастика" in Russian and "Social science
  fiction" in English, its year 1962

#### Scenario: An EPUB from Calibre
- **WHEN** an EPUB 2 has `<meta name="calibre:series" content="Discworld"/>` and `<meta name="calibre:series_index"
  content="3.0"/>`
- **THEN** its series is "Discworld", number 3
