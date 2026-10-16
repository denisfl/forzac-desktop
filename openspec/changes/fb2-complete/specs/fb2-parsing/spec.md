## Purpose

How FictionBook 2 books (`.fb2`, `.fb2.zip`) become the book model: which elements are read, how chapters, notes,
images, links and tables are formed, and how damaged files behave.

## ADDED Requirements

### Requirement: Recognised by content
The parser SHALL accept a FictionBook 2 document (root element `FictionBook`) and a zip archive containing one, whatever
the file name. Anything else MUST be rejected as "not a book". A book without any text MUST be rejected as "no text".

#### Scenario: A zipped book
- **WHEN** a `.fb2.zip` holding `book.fb2` is opened
- **THEN** the book inside is read

#### Scenario: Not a book
- **WHEN** an XHTML file or an arbitrary binary is opened
- **THEN** the result is "not a book"

### Requirement: Encodings
The text SHALL be decoded according to the XML declaration or the byte order mark, including UTF-8, UTF-16,
windows-1251 and KOI8-R. Named HTML entities common in books (`&nbsp;`, `&mdash;`, `&laquo;` and others) SHALL be
resolved without a DTD.

#### Scenario: A windows-1251 book
- **WHEN** a book declares `encoding="windows-1251"`
- **THEN** its Cyrillic text reads correctly

### Requirement: Chapters and contents
Top-level sections of the main body SHALL become chapters. A section that holds only subsections and a title, epigraph
or similar (a "part") SHALL be opened up: its subsections become chapters and its own blocks go to the start of the
first of them. A section with text of its own SHALL be one chapter with all its subsections. Blocks before the first
section SHALL form a chapter of their own. The contents SHALL list every titled section with its nesting level and
the position of its first block.

#### Scenario: A book with parts
- **WHEN** the body has a part "Part One" with chapters "1" and "2"
- **THEN** there are two chapters, the first starting with the part's title, and the contents list "Part One" at level 1
  and "1", "2" at level 2

### Requirement: Text elements
Paragraphs, titles, subtitles, epigraphs with their authors, poems (stanzas, verses), citations, dates, empty lines
and annotations in the body SHALL be read in any position the format allows, each with its kind (body, epigraph, verse,
citation, subtitle, author). Whitespace SHALL be collapsed as in XML, and the text SHALL be typographically
normalised: an em dash for a spaced hyphen and for a dialogue hyphen, a no-break space before an em dash and after
one-letter Russian words.

#### Scenario: A poem inside an epigraph
- **WHEN** an epigraph contains a poem and a text author
- **THEN** the verses are verse paragraphs and the author an author paragraph, in order

### Requirement: Inline styles
Emphasis, strong, superscript, subscript, strikethrough and code SHALL become style ranges over the paragraph text.
A note reference (`a type="note"`) SHALL become a note marker pointing at the note's id, with brackets and braces
removed from its text.

#### Scenario: A note marker
- **WHEN** a paragraph contains `<a l:href="#n1" type="note">[1]</a>`
- **THEN** the text contains "1" marked as a note reference to `n1`

### Requirement: Notes
Bodies named `notes` or `comments` SHALL be read as notes: each section with an id becomes a note with its title and
blocks; sections without an id are searched for nested ones.

#### Scenario: Footnotes
- **WHEN** the notes body holds sections `n1` and `n2`
- **THEN** both notes are available by id with their text

### Requirement: Links
A link to `#id` SHALL point to the place in the book where the element or section with that id begins (and ends,
within one chapter); a link to a note id SHALL point to the note; a link to an `http`, `https` or `mailto` address
SHALL be external. A link whose target does not exist SHALL be plain text.

#### Scenario: A link to another chapter
- **WHEN** a paragraph links to `#ch5` and a section has `id="ch5"`
- **THEN** the link target is the first block of that section

#### Scenario: A dangling link
- **WHEN** a link points to an id that is nowhere in the book
- **THEN** the text carries no link

### Requirement: Images
Images (`<binary>`, base64) SHALL be decoded and available by id with their media type; an `<image>` block SHALL place
an image in the text, and the cover SHALL be taken from the title info. Undecodable images SHALL be skipped.

#### Scenario: An illustrated chapter
- **WHEN** a section contains `<image l:href="#pic1"/>` and the book has `<binary id="pic1">`
- **THEN** the chapter has an image block for `pic1` and its bytes are available

### Requirement: Tables
Table rows SHALL become row blocks of a shared table with header cells (`th`) and column spans. A table with a single
column SHALL become plain paragraphs. When no row is a header and every non-empty cell of the first row is entirely
bold, the first row SHALL be treated as the header.

#### Scenario: A data table
- **WHEN** a section contains a table with a header row and three data rows
- **THEN** there are four row blocks of one table, the first marked as the header

### Requirement: Damaged files
A file that breaks off or turns malformed after the root element SHALL yield everything read before the damage,
including the section being read. An archive that unpacks to more than 256 MB MUST be rejected.

#### Scenario: A truncated book
- **WHEN** a book ends in the middle of its third chapter
- **THEN** the first two chapters and the read part of the third are available

### Requirement: Real books
Every FB2 book in the local corpus (`FORZAC_CORPUS`) SHALL open with at least one chapter.

#### Scenario: The corpus test
- **WHEN** the corpus test runs over a directory of books
- **THEN** it reports how many books were read and fails listing any that did not open
