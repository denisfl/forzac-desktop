## MODIFIED Requirements

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

## ADDED Requirements

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
