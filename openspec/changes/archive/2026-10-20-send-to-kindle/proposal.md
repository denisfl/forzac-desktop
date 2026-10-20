## Why

The maintainer reads on a Kindle (firmware 5.19, a mass-storage disk over USB) and sends books there with Calibre. An
EPUB copied into the Kindle does not show; Calibre's copies do. Inspecting them (2026-10-20): Calibre sends **MOBI 6**
(file version 6, no KF8 part) — Calibre's default "old" MOBI — and the Kindle reads them. Forzac should send a book the
same way, with no Calibre and no Amazon account. Row 21 of docs/roadmap.md, which planned AZW3; MOBI 6 is what this
Kindle is proven to read, and it is far simpler to write correctly (no KF8 skeleton and fragment indices).

## What Changes

- **"Send to Kindle"** in the book's "⋯" menu: with a Kindle connected by cable, Forzac converts the book and writes
  `Author — Title.mobi` into the Kindle's `documents` folder (replacing its own earlier copy of that book), then says
  "“Title” is on your Kindle. Eject it before reading.". Without a Kindle: "Connect your Kindle with a cable and try
  again.".
- **The conversion (core):** a MOBI 6 file (PalmDB, PalmDOC header, MOBI header, EXTH, uncompressed text records with
  multibyte overlap, image records, FLIS, FCIS, EOF) with: the cover (also as the Kindle's cover), a title page,
  chapters with page breaks, headings, paragraphs justified with a first-line indent, epigraphs, verse, citations,
  emphasis, superscripts, images, tables, the notes at the end with links both ways, a contents page that the Kindle's
  "Go to › Contents" opens, an NCX index with its trailing byte sequences (the Kindle's "Go to" list of chapters), and the
  details — title, authors, language, description, year, genres — with the reader's edits (`metadata-editing`), as a
  book ("EBOK"), not a personal document.
- **The cover in the Kindle's list:** a sideloaded book shows the thumbnail `system/thumbnails/thumbnail_<ASIN>_EBOK_
  portrait.jpg`, which Calibre's Kindle driver writes; Forzac writes it from a JPEG cover. Without it the first test on
  the maintainer's Kindle (2026-10-20) showed no cover, and "Go to › Contents" did not work without the NCX index.

## Non-goals

- AZW3/KF8, Kindle's hyphenation and typographic CSS (MOBI 6 has none); nested contents in the Kindle's list (the NCX
  is flat, as Calibre's); resizing the cover for the thumbnail; Kindles from 2024 on (MTP, no disk); Send to Kindle by
  e-mail; PalmDOC compression (files are larger, still small next to the Kindle's space).

## Capabilities

### New Capabilities
- `send-to-kindle`: the conversion to MOBI 6 and the copy onto a connected Kindle.

### Modified Capabilities
- `library`: "Send to Kindle" in the book's menu.

## Impact

- `core/`: `kindle` module (markup, records, EXTH, finding the Kindle, sending); strings; C ABI; tests, with the file
  checked by Calibre's reader (`ebook-convert`) during development.
- `gtk/`, `macos/`: the menu item, the work in the background, the message.
- Docs: `docs/roadmap.md` (row 21: MOBI 6), README.
