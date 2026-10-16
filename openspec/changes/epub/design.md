## Context

The core reads FB2 only (`core/src/fb2.rs`, a streaming quick-xml reader into the book model) and `forzac_core::open`
calls it directly. The Android app reads EPUB 2/3 (`EpubReader`, `XhtmlReader`) and FB3 (`Fb3Parser` feeding the FB2
reader); their behaviour is the reference, refined on a corpus of real books. Since `typesetting-complete` the model
and the typesetter handle notes, links, images and tables, so a parser only has to fill the model.

## Goals / Non-Goals

**Goals:** EPUB 2/3 and FB3 into the existing model with Android's rules; format detection by content in one place;
a corpus test over EPUB and FB3 books.

**Non-Goals:** MathML (formulas are read as their text), fixed-layout EPUB (read as an ordinary book), DRM removal,
publisher CSS beyond italic and bold classes, embedded fonts.

## Decisions

### One entry point, one error type
`forzac_core::open(path)` reads the file and calls `parse(bytes)`, which looks at the content: a zip with
`META-INF/container.xml` → EPUB; a zip whose `_rels/.rels` names a FictionBook 3 Book → FB3; anything else → FB2
(which accepts XML and zips with an `.fb2` entry). The error moves from `fb2::Error` to `forzac_core::Error`
(`NotABook`, `NoText`, `Protected`); the shells print it as before.

### Unpacking
A shared helper unpacks a zip into a map of names to bytes with a 256 MB total limit (a larger archive is "not a
book", like FB2's zip bomb rule). EPUB needs random access to its files; books are a few megabytes.

### EPUB: two layers, ported from Android
- `epub/xhtml.rs` — one XHTML document to blocks: a flat streaming walk where inline elements add text to the
  current paragraph and block elements close it, which survives invalid nesting (`span` around `div`, text directly in
  `div`). The paragraph's kind comes from the nearest ancestor with meaning (tag or class). Output: main blocks, hidden
  blocks (`aside` footnotes), anchors (`id` → first block, after last block, hidden or not), note reference targets,
  and which paragraphs are only a link (back links in notes). Text is collected as chars so that
  `typography::normalize` works as in FB2, then style ranges convert to byte offsets.
- `epub/mod.rs` — the package: container → OPF (metadata, manifest, spine, cover rules), CSS classes, contents (nav,
  else NCX), notes (collected from note reference targets; a document that is at least 60% note text is a notes
  document), chapter assembly (continuations, splits at top-level headings with contents entries, cover page skipped),
  heading level normalisation, contents and link targets placed through the chapter segments each document became.
- Keys: images and documents by their path in the archive; notes and place links by `path#fragment`; external links
  by the URL. These are opaque keys in the model, as FB2's ids are.

### FB3 through the FB2 reader
`fb3.rs` resolves the OPC relationships and hands the body part to the FB2 reader with an image resolver (relationship
id → part name). The FB2 reader learns the FB3 elements, which do not clash with FB2's: root `fb3-body`, `em`, `note`,
`notes`/`notebody` inside the body, `img`, `subscription`, `blockquote`, `div`, `pre`, `br`, lists, `p` in a stanza.
Metadata comes from the description part (not FB2's `title-info`).

### Line breaks in Chinese and Japanese
EPUB brings books without spaces between words; the typesetter only broke at spaces, hyphenation points and dashes, so
such a paragraph was one line running off the page. Words now also break between characters when either side is a Han
ideograph, kana or CJK punctuation, unless the next character may not start a line or the previous one may not end it
(a short kinsoku list). The break is a glue of zero width with a little stretch (0.1 em), so a justified line without
spaces fills the column by spacing the characters, as Japanese typesetting does.

### Speed
Measured on the Pi: the largest real books of the corpus parse in 106 ms (7.7 MB) and 131 ms (2.5 MB, 22 000 blocks).
The XHTML reader asks only for the attributes it uses (books put long `title` attributes on every span), skips the
duplicate-attribute check, lowercases tag names in place and deduplicates style ranges by sorting (a document can be
one paragraph of thousands of spans). What remains is mostly inflate and quick-xml itself.

### XML details
quick-xml with `check_end_names` off and empty elements expanded, as for FB2. Named HTML entities use FB2's table.
Documents are decoded to UTF-8 by FB2's BOM/declaration rule. Attribute values have `&amp;` and the like unescaped
(hrefs carry them).

## Risks / Trade-offs

- Heuristics (chapter continuation, note detection) are tuned on Android's corpus; desktop books may differ → the
  corpus test prints per-book counts (chapters, contents, notes, images) to spot outliers.
- Whole archive in memory: a 50 MB illustrated EPUB is held twice briefly (zip and unpacked) → acceptable on the
  target machines; images stay as bytes, decoded by the shells per page.
- Formulas (MathML) read as plain text can look odd in technical books → a later change if books need it.
- Very large stress samples are slower: a 10.6 MB EPUB of 36 MB of text (the Mahabharata sample, 2014 documents)
  parses in 650 ms on the Pi, a 0.9 MB EPUB of 25 MB of MathML in 310 ms. Real books are well inside the budget.
