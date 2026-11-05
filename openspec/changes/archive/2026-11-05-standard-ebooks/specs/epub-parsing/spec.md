## MODIFIED Requirements

### Requirement: Text elements
XHTML SHALL map to the model as follows: `h1`–`h6` and the converter classes `title`, `title1`…`title9` are headings;
`p`, `div` and other block elements are paragraphs; `blockquote` and class `cite` are citations; classes `epigraph`,
`text-author`, `subtitle`, `poem`/`stanza`/`v` keep their FB2 kinds; `pre` is code with its line breaks and spaces;
list items get "• " or "N. " (respecting `ol start`), the marker at the start of the item's first paragraph, also a
nested one (`<li><p>…</p></li>`); `br` is a line break; `hr` and class `empty-line` are empty lines; `img` and SVG
`image` are images resolved against the document's path; `head`, `script`, `style`, `nav`, audio, video, ruby
readings and elements with the `hidden` attribute are skipped. Text is collapsed and typographically normalised as in
FB2.

The `epub:type` semantics of the DAISY `z3998` vocabulary, which Standard Ebooks use:
- an element with `z3998:poem`, `z3998:verse`, `z3998:song` or `z3998:hymn` is verse: its paragraphs SHALL be verse,
  in a `blockquote` too;
- a `blockquote` with `z3998:diary` or `z3998:diary-entry` MUST NOT make its paragraphs citations; a quotation inside
  it (a letter in a diary) stays a citation;
- in a `section` with `titlepage` that holds an image, the text MUST NOT reach the book: it is a caption for screen
  readers, and the title page is the image.

A `p` inside an `hgroup` SHALL be a heading one level below the group's heading before it (a chapter's title under its
number); without one, a subtitle. A verse line (a `span` in verse) with class `i1`…`i9` SHALL start with that many
em of indent, as that many U+2003. The signs `⸺` (U+2E3A), `⸻` (U+2E3B) and `℅` (U+2105) SHALL become `——`, `———`
and `c/o` when parsed, in the text and in the contents' titles.

#### Scenario: Lists and preformatted text
- **WHEN** a document has `<ol start="3"><li>Three</li></ol>` and a `pre` block with indented lines
- **THEN** the item reads "3. Three" and the code block keeps its line breaks and indentation

#### Scenario: A list item of a paragraph
- **WHEN** a document has `<ul><li><p>Claudius, king of Denmark</p></li><li><p>Hamlet</p></li></ul>`
- **THEN** the paragraphs read "• Claudius, king of Denmark" and "• Hamlet", with no paragraph that is only "•"

#### Scenario: A Standard Ebooks poem
- **WHEN** a document has `<article epub:type="z3998:poem"><h2>Imitation of Spenser</h2><p><span>Now Morning…</span><br/><span>And her first…</span></p></article>`
- **THEN** the paragraph of its lines is verse, the lines separated by line breaks

#### Scenario: A diary with a letter
- **WHEN** a chapter has `<blockquote epub:type="z3998:diary-entry"><p>3 May. Bistritz…</p><blockquote epub:type="z3998:letter"><p>My Friend…</p></blockquote></blockquote>`
- **THEN** "3 May. Bistritz…" is a body paragraph and "My Friend…" a citation

#### Scenario: The title page of an advanced EPUB
- **WHEN** the first spine document is `<section epub:type="titlepage"><h1>Poetry</h1><p>By John Keats.</p><img src="../images/titlepage.svg"/></section>`
  and the next is a collection whose poems start with `h2` headings and top-level contents entries
- **THEN** the title page is one image and skipped as the cover, and every poem is a chapter of its own (Keats: 133
  chapters, as in the compatible EPUB)

#### Scenario: A chapter's title in hgroup
- **WHEN** a chapter starts `<hgroup><h2 epub:type="ordinal">I</h2><p epub:type="title">Of the Nature of Flatland</p></hgroup><p>I call our world Flatland…</p>`
- **THEN** it has the headings "I" and "Of the Nature of Flatland" (a level below) and then the paragraph; the
  chapter's title is "I. Of the Nature of Flatland"

#### Scenario: A hidden subtitle
- **WHEN** a document has `<hgroup><h2>Sonnet</h2><p hidden="hidden" epub:type="subtitle">Happy Is England!</p></hgroup>`
- **THEN** the book has only the heading "Sonnet"

#### Scenario: A verse line's indent
- **WHEN** a document has `<article epub:type="z3998:poem"><p><span>Now Morning</span><br/><span class="i1">And her first</span></p></article>`
- **THEN** the paragraph's text is "Now Morning\n And her first"

#### Scenario: A two-em dash
- **WHEN** a paragraph reads "it⸺(not under your Protection"
- **THEN** its text is "it——(not under your Protection"

### Requirement: Inline styles
`em`, `i`, `cite`, `dfn`, `var` and CSS classes with `font-style: italic|oblique` SHALL be emphasis; `strong`, `b` and
classes with a bold `font-weight` SHALL be strong; classes with `font-variant: small-caps` (or `font-variant-caps`)
SHALL be small capitals; `sup`, `sub`, `s`/`strike`/`del` and `code`/`kbd`/`samp`/`tt` keep their styles. Only the
book's simple class selectors (`.c`, `tag.c`) are read from CSS, and the rules of `b` and `strong` themselves: small
capitals there make them small capitals, bold unless the rule sets `font-weight: normal`.

#### Scenario: Italic from a stylesheet
- **WHEN** CSS has `.calibre15 { font-style: italic }` and a span has that class
- **THEN** the span's text is emphasis

#### Scenario: Standard Ebooks' small capitals
- **WHEN** CSS has `b, strong { font-variant: small-caps; font-weight: normal; }` and a paragraph `<p>By <b>John Keats</b>.</p>`
- **THEN** "John Keats" is small capitals and not strong

#### Scenario: Gutenberg's small capitals
- **WHEN** CSS has `.smcap { font-variant: small-caps; }` and a paragraph `<p><span class="smcap">Chapter</span> One</p>`
- **THEN** "Chapter" is small capitals

#### Scenario: Plain bold
- **WHEN** the book's CSS says nothing of `b` and a paragraph is `<p><b>Attention</b></p>`
- **THEN** "Attention" is strong and not small capitals

### Requirement: Images and tables
Images in the manifest SHALL be in the book's images under their paths in the archive: raster images (JPEG, PNG, GIF,
WebP) and SVG outside the spine (`image/svg+xml`; an SVG document in the spine is read as a document), also when the
manifest labels them otherwise and their bytes show what they are. The cover (`cover-image`, `meta name="cover"`, the
guide's cover page) MAY be an SVG. Tables SHALL become table rows as in FB2: `th` or rows in `thead` are headers,
`colspan` is kept, paragraphs of a cell join with line breaks, a nested table becomes text of its cell, empty rows and
tables are dropped, `caption` is a subtitle before the table, and a one-column table is a layout wrapper whose content
becomes ordinary blocks.

#### Scenario: A data table with a caption
- **WHEN** a table has a caption, a `thead` row and two body rows
- **THEN** the book has a subtitle with the caption and three table rows, the first a header

#### Scenario: Illustrations of an advanced EPUB
- **WHEN** Standard Ebooks' advanced "Alice's Adventures in Wonderland" is opened: 44 illustrations
  `illustration-N.svg` and a cover `cover.svg` with `cover-image`
- **THEN** the book has 44 image blocks, every one with its SVG in the book's images as `image/svg+xml`, and its
  cover is `cover.svg`

#### Scenario: An SVG document in the spine
- **WHEN** the spine holds an SVG document with `<image xlink:href="p1.jpg">`
- **THEN** it is read as a document and the book shows the image `p1.jpg`
