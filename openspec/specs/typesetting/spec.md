# typesetting Specification

## Purpose
How the book model is typeset into pages: line breaking, hyphenation and justification, pagination rules, images,
tables, note markers and links, glyph fallback, and the time typesetting may take.
## Requirements
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
height, never split between pages, with half a line of space around it. An SVG's pixels are the units of its root:
its absolute `width` and `height`, else its `viewBox`. An image that is missing or cannot be read SHALL leave no gap.

#### Scenario: A large photograph
- **WHEN** a 2400×1600 image is placed in a 600-pixel column
- **THEN** it is drawn 600 pixels wide and 400 high, centred, on one page

#### Scenario: A logo in SVG
- **WHEN** a page holds an SVG with `viewBox="0 0 220 140"`
- **THEN** it takes 220 × 140 logical pixels, centred, as a PNG of 220 × 140 would

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

### Requirement: The book's language
The language that chooses hyphenation SHALL come from the book's metadata; a book that names none SHALL get it from
its text, as in the Android app: among the first 2000 letters of its paragraphs and headings, more than half Cyrillic
is Russian, anything else English. Russian and English are hyphenated; a book in another language is set without
hyphenation.

#### Scenario: An English book without a language
- **WHEN** an English FB2 has no `<lang>`
- **THEN** its long words are hyphenated by the English patterns

#### Scenario: A German book
- **WHEN** a book declares `de`
- **THEN** it is set without hyphenation and justified with wider spaces

### Requirement: Code and strikethrough
Preformatted paragraphs (code) SHALL be set in the bundled monospace face (PT Mono, OFL) at 0.85 of the text size,
keeping their line breaks and spaces; inline code SHALL be set in the same face at 0.9. Text in the strikethrough
style SHALL be drawn with a line through it, in the text's ink, at about half the x-height above the baseline.

#### Scenario: A code listing
- **WHEN** an EPUB has a `pre` block with indented lines
- **THEN** the lines are set in PT Mono with their indentation, and the characters line up in columns

#### Scenario: Struck-out text
- **WHEN** a paragraph has `<strikethrough>old</strikethrough>`
- **THEN** "old" is drawn with a line through it

### Requirement: A table for the view
The core SHALL typeset a table for the view at a given text size: columns by content without fitting them to the
column — each at least as wide as its longest word and at most 20 em, as wide as its longest line within that — with
the page's gap between columns, cell padding, line spacing, rules, bold header cells, note markers and links; it
reports the table's width and height, its drawing and the areas of its markers and links. On a Raspberry Pi 5 a table
of 100 rows SHALL be typeset for the view in at most 50 ms.

#### Scenario: A column of long text
- **WHEN** a table for the view has a column whose cells hold a sentence of 60 em
- **THEN** that column is 20 em wide and the sentence wraps inside its cells

### Requirement: List items
A list item paragraph (Markdown) SHALL be set as on Android: without a first-line indent, ragged right, hyphenated;
its wrapped lines SHALL start under the text after the marker ("• ", "1. ", "☐ ", "☑ "), not under the marker; a nested
item SHALL start 1.5 em further right per level, four levels at most. Items of one list SHALL have no spacing between
them and half a line around the list. A task's `☐` SHALL be drawn as a box the size of a letter in the text's colour,
`☑` as the box with a check mark, and a done task's text in the secondary colour; the character stays in the text.

#### Scenario: A long item
- **WHEN** a bullet item wraps to three lines
- **THEN** the second and third lines start where the text after "• " starts

#### Scenario: Tasks
- **WHEN** a Markdown file has `- [ ] сделать` and `- [x] сделано`
- **THEN** the first shows an empty box, the second a box with a check mark and its text muted, in every theme

### Requirement: Ties after short words
A no-break space that ties a one-letter word to the next (`typography`: "в", "к", "с", "у", "о", "а", "и", "я") SHALL keep them on one line in justified text, unless that would set a line with holes: then the line MAY break after the short word, at the cost of a line about three times looser than it should be. Ragged-right text MUST NOT break there. The text, and so places, quotes, bookmarks and search, MUST NOT change.

#### Scenario: A tie before a hole
- **WHEN** a justified line would have spaces three times their width because "о учительнице" cannot be split
- **THEN** "о" ends the line and the line is set without the hole

#### Scenario: A tie without a hole
- **WHEN** "в качестве" stands in a line that sets well either way
- **THEN** "в" does not end the line, as before

#### Scenario: Measured
- **WHEN** "Оцифруйся или умри" is typeset in a 760 × 980 window with the default settings
- **THEN** fewer than 15 justified lines have a space over three times its width (23 before), and fewer than one line in 100 ends in a tied short word

### Requirement: SVG images
An SVG image SHALL show wherever a raster image shows: on the page (both modes), in the image view, in the note panel
and as the cover in the library. It SHALL be at least as sharp as the screen allows at the size it is shown, up to
the view fitted to the screen. It SHALL be drawn off the main thread, except an image a page needs before it could be
drawn ahead (the first page of an opened book); a page drawn again, a turn back, a new window size or new settings
SHALL NOT draw it again. An SVG that cannot be read SHALL be skipped as a broken image. An SVG SHALL NOT load anything
outside the book.

#### Scenario: A Tenniel illustration
- **WHEN** a page holds an illustration with `viewBox="0 0 741 1089"` of 3 MB of paths, in a column narrower than 741
- **THEN** it is drawn as wide as the column with sharp lines, and opened in the view it fills the window's height,
  sharp

#### Scenario: An SVG cover
- **WHEN** the library shows a book whose cover is an SVG holding a JPEG (`data:image/jpeg;base64,…`) and its title as
  paths
- **THEN** the cover shows the picture and the title, not a generated cover

### Requirement: Line art
An image whose background is transparent and whose opaque pixels are dark and colourless (a diagram, an engraving, a logo, a title page) SHALL be drawn in the theme's text colour with its pixels' opacity, on the page, in the view and in the note panel. Other images SHALL be drawn as they are. A theme change SHALL NOT re-typeset: only the colour at drawing changes. There is no setting for it.

#### Scenario: A diagram in the dark theme
- **WHEN** "Flatland" shows a page with a diagram (black lines on transparency) in the Dark theme
- **THEN** the diagram's lines are light, the text's colour, and clear on the dark paper

#### Scenario: A colour picture
- **WHEN** a page holds a colour illustration or a photograph, a PNG with transparent edges too
- **THEN** it is drawn in its own colours in every theme

### Requirement: Small capitals
Text in small capitals SHALL be set with lower-case letters as small capitals and capitals as they are: with the
face's own small capitals when it has them (OpenType `smcp`: Literata, Source Serif's roman), else as capitals at
0.78 of the size. The book's text, search, quotes, marks and the `Locator` SHALL NOT change: small capitals are only
how letters look. They SHALL NOT change the line height.

#### Scenario: Literata
- **WHEN** the font is Literata and "John Keats" is in small capitals
- **THEN** "J" and "K" are capitals and "ohn" and "eats" the font's small capital glyphs

#### Scenario: A face without small capitals
- **WHEN** the font is PT Serif and "John Keats" is in small capitals
- **THEN** "J" and "K" are capitals of the text's size and the other letters capitals of the smaller size, at the
  same text offsets

### Requirement: Lines within a paragraph
A line break in a paragraph (`br`) SHALL start a line without the first-line indent: the indent is the paragraph's
first line's only, as in HTML. In verse, the part of a line that does not fit the column SHALL go on 1 em right of
where its line starts (its indent included), so that it is not taken for the next line; a line's leading U+2003 are
1 em each in any font.

#### Scenario: A dedication with line breaks
- **WHEN** a paragraph is `<p>To<br/>The Inhabitants of Space<br/>And H. C.</p>`
- **THEN** "To" has the first-line indent and "The Inhabitants of Space" and "And H. C." start at the edge

#### Scenario: A long verse line
- **WHEN** a verse line indented 1 em does not fit the column
- **THEN** its wrapped part starts 2 em from the verse column's edge and the next line of the poem at its own indent

### Requirement: A line as tall as its formula
A line of a paragraph SHALL be as tall as the paragraph's rhythm, or as its tallest formula with 0.15 em above and below when that is more. Other lines MUST NOT change height, and a paragraph without formulas MUST NOT typeset differently from before, to the pixel. A page break MAY fall between such lines as between any others, and a line taller than the page SHALL be drawn as it is, as an image taller than the page is.

#### Scenario: A book without formulas
- **WHEN** a novel is typeset before and after this change
- **THEN** every page holds the same glyphs in the same places

#### Scenario: A fraction in a line
- **WHEN** a line holds a tall fraction
- **THEN** that line is taller, and the lines above and below it keep their distance from each other's text

### Requirement: A math font
A math font SHALL be bundled, under the OFL, and used only inside formulas, for characters the book's face lacks, before the platform's fallback. Characters outside formulas MUST NOT use it. Each shell SHALL rasterise exactly that file.

#### Scenario: A subset sign on a bare system
- **WHEN** a formula has `⊆` and the book's face lacks it, on a Linux system with no math fonts
- **THEN** the sign shows from the bundled font

