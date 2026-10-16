## Purpose

How EPUB 2 and EPUB 3 books become the book model: package, reading order, chapters, contents, notes, links, images
and tables, and how unsupported or damaged content degrades.

## ADDED Requirements

### Requirement: Recognised by content
A zip archive with `META-INF/container.xml` SHALL be read as EPUB whatever its file name (`.fb2.epub` included). An
archive that unpacks to more than 256 MB MUST be rejected as "not a book". A book whose content files other than fonts
are encrypted MUST be rejected as "protected" (DRM); obfuscated fonts MUST NOT count as protection. A book without any
text MUST be rejected as "no text".

#### Scenario: A converter's EPUB with a misleading name
- **WHEN** `book.fb2.epub` is opened
- **THEN** it is read as EPUB

#### Scenario: A protected book
- **WHEN** `META-INF/encryption.xml` lists a content document
- **THEN** the result is "protected", not a book with garbage text

### Requirement: Metadata
The title, the authors (creators with role `aut` or no role; all creators when none qualifies), the language, the
description (without markup) and the cover image SHALL come from the package document. The cover is the manifest item
with property `cover-image`, else the one named by `<meta name="cover">`, else the first image of the guide's cover
document, else an image whose id or path contains "cover".

#### Scenario: An EPUB 2 package
- **WHEN** the package has `dc:title`, an author and a translator, and `<meta name="cover">`
- **THEN** the book has that title, only the author, and that cover image

### Requirement: Reading order and chapters
Documents SHALL be read in spine order; documents marked `linear="no"` and notes documents are left out of the reading
flow. A document starts a new chapter when it is the first one, starts with a heading or is the target of a contents
entry at its start; otherwise it continues the previous chapter (converters split long chapters into files). Inside a
document a new chapter starts at a heading of the book's top level that a top-level contents entry points to. A first
document that is exactly one image is the cover page and is skipped. An image listed in the spine is a page with that
image. Heading levels are normalised so the book's top level is 1; a chapter's title is its leading headings joined by
". ".

#### Scenario: A chapter split into two files
- **WHEN** `ch1.xhtml` starts with a heading and `ch1_split.xhtml` does not and has no contents entry
- **THEN** both form one chapter

#### Scenario: Several chapters in one file
- **WHEN** one document holds two `h1` headings, each the target of a top-level contents entry
- **THEN** they are two chapters

### Requirement: Contents
The contents SHALL come from the EPUB 3 navigation document (`nav epub:type="toc"`, levels from nesting), else from the
NCX. Each entry SHALL point to the block its target element starts at, in the chapter that block ended up in.

#### Scenario: A contents entry to a section inside a chapter
- **WHEN** an entry points to `ch1.xhtml#s2`, the third block of the chapter
- **THEN** the entry's target is that chapter, block 2

### Requirement: Text elements
XHTML SHALL map to the model as follows: `h1`–`h6` and the converter classes `title`, `title1`…`title9` are headings;
`p`, `div` and other block elements are paragraphs; `blockquote` and class `cite` are citations; classes `epigraph`,
`text-author`, `subtitle`, `poem`/`stanza`/`v` keep their FB2 kinds; `pre` is code with its line breaks and spaces;
list items get "• " or "N. " (respecting `ol start`); `br` is a line break; `hr` and class `empty-line` are empty
lines; `img` and SVG `image` are images resolved against the document's path; `head`, `script`, `style`, `nav`, audio,
video and ruby readings are skipped. Text is collapsed and typographically normalised as in FB2.

#### Scenario: Lists and preformatted text
- **WHEN** a document has `<ol start="3"><li>Three</li></ol>` and a `pre` block with indented lines
- **THEN** the item reads "3. Three" and the code block keeps its line breaks and indentation

### Requirement: Inline styles
`em`, `i`, `cite`, `dfn`, `var` and CSS classes with `font-style: italic|oblique` SHALL be emphasis; `strong`, `b` and
classes with a bold `font-weight` SHALL be strong; `sup`, `sub`, `s`/`strike`/`del` and `code`/`kbd`/`samp`/`tt` keep
their styles. Only the book's simple class selectors (`.c`, `tag.c`) are read from CSS.

#### Scenario: Italic from a stylesheet
- **WHEN** CSS has `.calibre15 { font-style: italic }` and a span has that class
- **THEN** the span's text is emphasis

### Requirement: Notes
A link SHALL be a note reference when it has `epub:type="noteref"`, or when its fragment-bearing target text is a
short number or asterisks inside `sup` or brackets. The marker text is the number without brackets. The note text is
the target element's blocks; for an empty anchor, the blocks up to the next note target or heading. A leading heading
becomes the note's title; paragraphs that are only a back link are dropped. `aside` elements with `epub:type`
footnote, endnote or rearnote are removed from the reading flow. A note reference whose note is not found becomes plain
text.

#### Scenario: EPUB 3 footnote
- **WHEN** a paragraph links with `epub:type="noteref"` to an `aside epub:type="footnote"`
- **THEN** the paragraph has a note marker, the aside's text is the note, and the aside is not in the chapter

#### Scenario: A publisher's notes file
- **WHEN** markers `<a href="notes.html#n_10"><sup>[10]</sup></a>` point to a file of notes with "Back" links
- **THEN** the marker reads "10", the note has the note text without "Back", and the notes file is not a chapter

### Requirement: Links
Other links SHALL point to a place in the book (the target element's first block up to after its last block, or a
point for an inline anchor or a whole document), to a note (a link to a note's target opens the note), or outside the
book (`http`, `https`, `mailto`). Links to targets that are not in the reading flow, empty links and other schemes MUST
become plain text.

#### Scenario: A link to a figure
- **WHEN** a paragraph links to `<figure id="fig">` holding an image and a caption
- **THEN** the link's target covers the image and the caption

### Requirement: Images and tables
Raster images in the manifest (JPEG, PNG, GIF, WebP) SHALL be in the book's images under their paths in the archive.
Tables SHALL become table rows as in FB2: `th` or rows in `thead` are headers, `colspan` is kept, paragraphs of a cell
join with line breaks, a nested table becomes text of its cell, empty rows and tables are dropped, `caption` is a
subtitle before the table, and a one-column table is a layout wrapper whose content becomes ordinary blocks.

#### Scenario: A data table with a caption
- **WHEN** a table has a caption, a `thead` row and two body rows
- **THEN** the book has a subtitle with the caption and three table rows, the first a header

### Requirement: Damaged files
A document that fails to parse SHALL keep the blocks read before the error; a document that cannot be read at all is
left out. The book fails only when no document has text.

#### Scenario: A truncated chapter file
- **WHEN** one spine document ends in the middle of an element
- **THEN** the book opens with that chapter's text up to the break

### Requirement: Parsing time
Parsing an EPUB of up to 10 MB SHALL take at most 500 ms on a Raspberry Pi 5.

#### Scenario: Opening a large book
- **WHEN** the largest EPUB of the corpus is opened on a Raspberry Pi 5
- **THEN** parsing takes at most 500 ms

### Requirement: Real books
A corpus test SHALL open every FB2, FB3 and EPUB book in a local directory and check: a title, at least one chapter,
every note marker has its note, every contents entry points inside the book; and typeset every page. Picture books
without any text (fixed-layout pages of images) are reported, not failed. Image blocks without a raster image in the
book (SVG, missing files) are counted; they take no room on the page.

#### Scenario: The corpus test
- **WHEN** `FORZAC_CORPUS` names a directory of books
- **THEN** every book with text in it opens and passes the checks

### Requirement: Chinese and Japanese text
Text without spaces between words (Han ideographs, kana) SHALL break between characters, except before closing
punctuation, small kana and the prolonged sound mark, and after opening punctuation; justified lines of such text
SHALL fill the column by spacing the characters.

#### Scenario: A Japanese novel
- **WHEN** a paragraph of Japanese text is longer than a line
- **THEN** it wraps within the column and no line starts with "。" or "、"
