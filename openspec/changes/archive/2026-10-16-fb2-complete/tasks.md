## 1. Model

- [x] 1.1 Notes, links, contents, images, cover and annotation in `Book`; image and table-row blocks; link style and
      range targets
- [x] 1.2 The typesetter skips image and table blocks (drawn in `typesetting-complete`)

## 2. Parser

- [x] 2.1 Block sequence numbers and ids of sections and elements
- [x] 2.2 Notes bodies
- [x] 2.3 Links: internal, notes, external; dangling links become plain text
- [x] 2.4 Images and the cover; inline images after their paragraph
- [x] 2.5 Tables: rows, header cells, column spans, single-column tables, bold first row as header
- [x] 2.6 Contents with levels and Locators
- [x] 2.7 Annotation of the book

## 3. Robustness

- [x] 3.1 Encodings: UTF-16 with BOM, windows-1251, KOI8-R
- [x] 3.2 Truncated and malformed files keep what was read

## 4. Tests

- [x] 4.1 Unit tests for every requirement in `specs/fb2-parsing`
- [x] 4.2 Corpus test: every FB2 in `FORZAC_CORPUS` opens; also counts notes, images, links and tables found

## 5. Verification

- [x] 5.1 `cargo fmt`, `clippy -D warnings`, tests on macOS and in the Linux container
- [x] 5.2 The corpus (`~/Downloads`) passes
- [x] 5.3 The GTK shell on the Raspberry Pi (`scripts/pi-dev.sh`) still opens and turns pages of a book with notes and
      images; the macOS shell builds
