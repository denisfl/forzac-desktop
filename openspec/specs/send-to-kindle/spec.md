# send-to-kindle Specification

## Purpose
TBD - created by archiving change send-to-kindle. Update Purpose after archive.

## Requirements

### Requirement: The Kindle's format
The core SHALL convert a book to a MOBI 6 file as Calibre's "old" MOBI output is laid out: a PalmDB with the book's
title, record 0 with a PalmDOC header (no compression, 4096-byte text records), a MOBI header of version 6 (UTF-8, the
book's language), an EXTH block and the full title; text records of exactly 4096 bytes of the markup each, a character
split across records repeated after it with its byte count (extra data flag 1), then the record's trailing byte sequence
for the NCX (extra data flag 2); one record per image (JPEG, PNG or GIF); an NCX index (a header record, one index record
and a CNCX record, Calibre's flat book index: offset, length, label and depth 0 for every contents entry up to the end
of the chapters, the MOBI header naming it as the primary index); FLIS, FCIS and EOF records. The markup SHALL hold, in order: the cover image when the book has one, a title page with
the title and authors, every chapter after a page break — headings centred and bold, larger by level, body paragraphs
justified with a first-line indent, epigraphs and citations indented, verse kept line by line, the text author right
aligned, code in a monospace face, emphasis, strong, superscript, subscript and strikethrough, images centred, table
rows as tables — then the notes, each with a link back to its first marker, and a contents page listing the book's
contents with links, which the guide names as the table of contents. Every internal link (contents, note markers,
links in the text) SHALL lead to its target's place in the file. EXTH SHALL hold the authors, the description, the year,
the genres as subjects, the language, the title, the cover (also as thumbnail), the start (the first chapter) and the
document type "EBOK", with the reader's edits to title, authors and genres applied.

#### Scenario: Calibre reads it back
- **WHEN** an FB2 with a cover, two chapters, a note and a table is converted and the file is read by Calibre's MOBI
  reader
- **THEN** the reader finds the title, authors and language, the cover, both chapters' text in order, the note with
  working links both ways and the contents page

### Requirement: Sending to a connected Kindle
"Send to Kindle" ("Отправить на Kindle") SHALL look for a Kindle connected as a disk — a mounted volume with
`documents` and `system` folders (`/Volumes/*` on macOS; `/run/media/<user>/*`, `/media/<user>/*`, `/media/*` on
Linux). Without one it SHALL say "Connect your Kindle with a cable and try again.". With one it SHALL convert the book
off the UI thread and write it into `documents` as `Author — Title.mobi` (named as `File names in the folder` says, an
earlier copy of the same name replaced), writing a temporary file first so a half-written book never shows; a JPEG
cover SHALL also be written as `system/thumbnails/thumbnail_<book id>_EBOK_portrait.jpg`, the book id being the ASIN in
EXTH 113, so the Kindle's list shows it. Then it SHALL say
"“Title” is on your Kindle. Eject it before reading.". A failure SHALL say "Couldn't write to the Kindle." and leave no
partial file.

#### Scenario: A book to the Kindle
- **WHEN** the reader's Kindle is connected and they choose "Send to Kindle" on "Конец света"
- **THEN** `/Volumes/Kindle/documents/Наталья Евдокимова — Конец света.mobi` exists, and after ejecting, the Kindle
  lists the book with its cover, opens it, and "Go to" lists its chapters
