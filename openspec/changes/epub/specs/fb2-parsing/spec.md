## MODIFIED Requirements

### Requirement: Recognised by content
The parser SHALL accept a FictionBook 2 document (root element `FictionBook`) and a zip archive containing one, whatever
the file name. Anything else MUST be rejected as "not a book". A book without any text MUST be rejected as "no text".
The format of a file SHALL be decided by its content in one entry point for all formats: a zip with
`META-INF/container.xml` is EPUB, a zip with a FictionBook 3 relationship is FB3, other zips and XML are FB2.

#### Scenario: A zipped book
- **WHEN** a `.fb2.zip` holding `book.fb2` is opened
- **THEN** the book inside is read

#### Scenario: Not a book
- **WHEN** an XHTML file or an arbitrary binary is opened
- **THEN** the result is "not a book"

#### Scenario: A misleading extension
- **WHEN** an EPUB is named `book.fb2`
- **THEN** it is read as EPUB
