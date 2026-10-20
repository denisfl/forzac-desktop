# Tasks

## 1. Core

- [x] 1.1 `kindle::markup`: the book as Mobipocket HTML with placeholders and targets; the second pass; tests
- [x] 1.2 `kindle::mobi`: records, record 0 (PalmDOC, MOBI 6, EXTH, title), text records with overlap, images, FLIS,
      FCIS, EOF, the PalmDB; tests
- [x] 1.3 `kindle::find`, `kindle::send` (temporary file, rename); strings; C ABI; tests
- [x] 1.4 Checked with Calibre's reader on corpus books (`calibre-debug --inspect-mobi`, `ebook-convert` to EPUB: text,
      images, notes, contents; the NCX index; the TBS byte for byte as the maintainer's Calibre file); the thumbnail

## 2. Shells

- [x] 2.1 GTK: "Send to Kindle" in the book's menu, in the background, the message
- [x] 2.2 macOS: the same
- [x] 2.3 Both builds and clippy green; on the maintainer's Kindle (5.19.2): sent from the Mac library's menu (after
      macOS asked for the removable volume), the book opens with its text; after adding the NCX index and the
      thumbnail, both books are listed with covers and "Go to" lists the chapters. GTK: compiled and clippy only
- [x] 2.4 `docs/roadmap.md`, README, `CLAUDE.md` (the `mobi` example)
