# fb3-parsing Specification

## Purpose
How FictionBook 3 packages (`.fb3`, LitRes) become the book model.

## Requirements

### Requirement: Package
A zip archive whose `_rels/.rels` has a FictionBook 3 Book relationship SHALL be read as FB3 whatever its file name.
The description comes from that relationship, the body from the description's `body` relationship, images from the
body's `image` relationships, the cover from the package's `thumbnail` relationship (raster images only). Part names
are compared case-insensitively after decoding `%XX`; relationship targets resolve against the source part's folder
or the package root. A package without a description or body MUST be rejected as "not a book".

#### Scenario: A LitRes book
- **WHEN** an `.fb3` file is opened
- **THEN** its body, images and cover are read through the relationships

### Requirement: Metadata
The title SHALL be the description's `title/main`; authors are `subject link="author"` entries (their `title/main`,
else first, middle and last names); the language and the annotation come from the description. What the description
lacks comes from the package's core properties (`dc:title`, `dc:creator`).

#### Scenario: Authors and translators
- **WHEN** the description lists two authors and a translator
- **THEN** the book has the two authors only

### Requirement: Body
The body SHALL be read with the FB2 rules for sections, titles, paragraphs, epigraphs, poems, citations, tables,
notes and links, plus the FB3 elements: `em` and `strong`, `note` references (`href` without `#`), `notes`/`notebody`
inside the body, `img` by relationship id (inside a paragraph it becomes an image block after it), `subscription` as
the text author, `blockquote` as a citation, `div` as its content, `pre` as code, `br` as an empty line, `ul`/`ol`
lists with "• " and "N. ", and `p` directly in a `stanza` as a verse line. An image whose relationship is missing is
dropped.

#### Scenario: Footnotes in FB3
- **WHEN** a paragraph has `<note href="n_1">1</note>` and the body has `<notes><notebody id="n_1">…</notebody></notes>`
- **THEN** the paragraph has a note marker "1" leading to that note, and the notes are not a chapter
