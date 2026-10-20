# Design

## Context

The maintainer's Kindle (firmware 5.19.2) holds 52 `.mobi` files sent by Calibre 9; `calibre-debug --inspect-mobi` on
one shows a MOBI 6 file: header length 232, file version 6, PalmDOC-compressed 4096-byte text records with extra data
flags `0b11` (multibyte overlap, TBS for its NCX), an NCX index, JPEG records, FLIS (36 bytes), FCIS (44 bytes), EOF;
EXTH with 100, 101, 103, 106, 108, 112, 113, 116, 129, 131, 201–207, 501 `EBOK`, 503, 524, 528. The markup is
Mobipocket HTML: `<guide><reference type="toc" … filepos=0000487623 />`, `<mbp:pagebreak/>`, `<p height width align>`,
`<font size>`, `<blockquote>`, `<img recindex="00047">`, `<a filepos=…>`.

## Decisions

**1. MOBI 6, not AZW3.** What this Kindle demonstrably reads, and no KF8 indices. The cost: no CSS, Kindle's own MOBI
layout (justified, its fonts), no hyphenation.

**2. Markup with fixed-width positions.** The converter writes the markup once with `filepos=` placeholders of ten
digits and records where each target (a block of a chapter, a note, a note marker, the contents page, the first
chapter) starts; a second pass writes the numbers in. Positions are byte offsets in the uncompressed text, as MOBI 6
needs.

**3. No compression; an NCX index with TBS.** Text records hold 4096 bytes of markup each plus the overlap of a split
UTF-8 character and its count (flag 1), then the record's trailing byte sequence and its size (flag 2). The first test on
the Kindle showed that the guide's contents page alone does not make "Go to › Contents" work, so the file carries
Calibre's flat book index: an INDX header record (TAGX with tags 1–4), one INDX record (idents "00", "01"… in hex,
control byte 0b1111, offset, length, CNCX label, depth 0) and a CNCX record; each text record's TBS is Calibre's
`book_tbs` (type 2 where one entry starts, 3 where one is spanned, 6 with the count where entries end and start),
checked byte for byte against the maintainer's Calibre file.

**4. Records as Calibre lays them out.** 0: header; 1…n: text; a zero padding record when the text records' total size
is not a multiple of 4; images (first image = first non-book record); FLIS; FCIS (with the text length); EOF. The MOBI
header follows Calibre's `writer2` field by field (232 bytes); the language code is the MOBI locale (ru 0x19, en 0x09,
…). Record 0 ends with the full title and 8 KiB of zeros, as Calibre's.

**5. Finding and writing.** `kindle::find()` checks the platform's mount roots for a folder with `documents` and
`system`; `kindle::send()` converts and writes `.<name>.part` then renames, in `documents`, and writes a JPEG cover as
the thumbnail Calibre's Kindle driver writes (`system/thumbnails/thumbnail_<ASIN>_EBOK_portrait.jpg`): the Kindle does
not take a sideloaded book's cover from the file. Shells call it off the UI thread and show the message.

**6. Checked by Calibre.** During development the files are read with `ebook-convert book.mobi out.epub` and
`calibre-debug --inspect-mobi`; the unit tests check the structure (record offsets, header fields, positions resolving
to their targets, overlap bytes) without Calibre.

## Risks / Trade-offs

- Large images: MOBI 6 readers may refuse images over ~127 KB; Forzac does not resize (no image codec in the core).
  Calibre's own sample carries a 500 KB JPEG and the Kindle shows it, so the risk is accepted. The thumbnail is the
  whole cover (Calibre's are about 357 × 500); the Kindle showed it. A PNG cover gets no thumbnail (no JPEG encoder).
