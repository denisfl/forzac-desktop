## Purpose

How the book model is typeset into pages: line breaking, hyphenation and justification, pagination rules, images,
tables, note markers and links, glyph fallback, and the time typesetting may take.

## ADDED Requirements

### Requirement: Line breaking and justification
Body paragraphs and citations SHALL be justified with hyphenation by the book's language and line breaks chosen for
the whole paragraph (Knuth–Plass); verse, headings, subtitles, epigraphs and authors SHALL be ragged without stretching
spaces. Body paragraphs SHALL have a first-line indent of 1.5 em. No-break spaces SHALL never break. Every line of a
block SHALL have the same height, so the vertical rhythm holds across paragraphs.

#### Scenario: A justified paragraph
- **WHEN** a long body paragraph is typeset
- **THEN** every line but the last ends at the right edge of the column, hyphenated words show a hyphen, and the last
  line keeps natural spacing

### Requirement: Pagination rules
A page SHALL NOT start with the last line of a paragraph nor end with its first line when the paragraph has more lines.
A heading SHALL stay on the same page as at least two lines of the following text. A chapter SHALL start on a new page,
two lines lower. An empty line SHALL NOT start a page.

#### Scenario: A heading at the bottom
- **WHEN** a heading would be the last thing on a page
- **THEN** it moves to the next page together with the start of its text

### Requirement: Images
An image SHALL be shown centred, one logical pixel per image pixel, scaled down to fit the column width and the page
height, never split between pages, with half a line of space around it. An image that is missing or cannot be read
SHALL leave no gap.

#### Scenario: A large photograph
- **WHEN** a 2400×1600 image is placed in a 600-pixel column
- **THEN** it is drawn 600 pixels wide and 400 high, centred, on one page

### Requirement: Note markers and links
Note markers SHALL be set as superscripts. Text of a link SHALL be drawn in the theme's link colour. Every page SHALL
report the areas of its note markers and links with their targets, so a shell can activate them by key or click.

#### Scenario: A page with a note
- **WHEN** a page contains the marker of note `n1`
- **THEN** the page lists an area around the marker with target note `n1`

### Requirement: Tables
Column widths SHALL follow the content: the longest word of each column at least, the rest shared in proportion to the
text; when that does not fit, the table SHALL be set at 0.9 and then 0.8 of the text size, and at the smallest size
columns SHALL be shared in proportion to their longest words, with hyphenation. Rows SHALL never be split between
pages; a header row SHALL stay with the first data row. Rules SHALL follow book style: heavier above and below the
table, medium under the header, light between rows, no vertical rules. Header cells SHALL be bold.

#### Scenario: A table wider than the column
- **WHEN** a table's longest words do not fit at the text size
- **THEN** it is set at a smaller size, and no row is split between pages

### Requirement: Glyph fallback
A character the book face does not have SHALL be shaped and drawn with a fallback font that has it, found among the
system's fonts, and the shell SHALL draw it from the same font file. Colour glyphs (emoji) SHALL keep their colours.
Only when no font has the character MAY a missing-glyph box be drawn.

#### Scenario: Greek in a Russian book
- **WHEN** a paragraph quotes a Greek word and a system font covers Greek
- **THEN** the Greek letters are drawn with that font, with no boxes, and the line is justified with their real widths

### Requirement: Layout keeps the reading position
A change of window size or text size SHALL re-typeset the chapter and show the page that contains the reading
position.

#### Scenario: Resizing the window
- **WHEN** the window is resized while reading the middle of a chapter
- **THEN** the page shown after the resize starts at or just before the same line

### Requirement: Typesetting time
On a Raspberry Pi 5 a chapter of 100 pages SHALL be typeset in at most 100 ms, and turning to the next or previous
chapter SHALL NOT wait for typesetting: neighbouring chapters are typeset in the background beforehand. Producing the
contents of one page from a typeset chapter SHALL take under 1 ms.

#### Scenario: Turning into the next chapter
- **WHEN** the reader turns from the last page of a chapter to the next one at 120 Hz on a Raspberry Pi 5
- **THEN** the page-turn animation starts without waiting for layout and misses no frames inside the animation
