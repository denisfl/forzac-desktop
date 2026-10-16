## Why

EPUB is the format most books come in outside the Russian-speaking world, and the Omarchy audience is international.
FB3 (LitRes) shares FB2's body format. Without them the reader covers only part of the books people have.

## What Changes

- EPUB 2 and 3: package (OPF), spine order, metadata, contents (NCX and the navigation document), XHTML to the book
  model (headings, paragraphs, emphasis, lists, block quotes, preformatted text, tables, images), footnotes (EPUB 3
  `epub:type` and common conventions), internal links.
- FB3: the OPC package, its body read by the FB2 reader.
- Format detection by content, not by extension (`.fb2.epub` is an EPUB).
- A corpus test over a local directory, as for FB2.

## Capabilities

### New Capabilities
- `epub-parsing`: what is read from EPUB 2/3 and how unsupported content degrades.
- `fb3-parsing`: FB3 packages.

### Modified Capabilities
- `fb2-parsing`: format detection moves to a shared entry point.

## Impact

New `core/src/epub/` and `core/src/fb3.rs`; ported from the Android app's `EpubReader`, `XhtmlReader`, `Fb3Parser`.

## Non-goals

Publisher CSS beyond what the model needs, fixed-layout EPUB, DRM, MathML.
