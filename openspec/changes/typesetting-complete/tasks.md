## 1. Page contents

- [ ] 1.1 `PageContent { runs, images, rules, areas }`; runs carry a font id and an ink
- [ ] 1.2 Link ink for link text; note markers and links produce areas with their targets
- [ ] 1.3 C ABI: font id and ink per run, images, rules, areas

## 2. Images

- [ ] 2.1 Image blocks in the core: dimensions from headers, scaling to column and page, atomic, half-line spacing,
      unreadable images skipped
- [ ] 2.2 GTK: textures from image bytes, cached per image
- [ ] 2.3 macOS: images through ImageIO

## 3. Tables

- [ ] 3.1 Column geometry (port of `columnWidths`/`squeezedWidths`, sizes 1/0.9/0.8)
- [ ] 3.2 Row layout: cells, header bold, rules; atomic rows, header kept with the next row
- [ ] 3.3 Rules drawn in both shells

## 4. Glyph fallback

- [ ] 4.1 Font ids, a fallback provider interface, runs split by font, per-character cache, memory-mapped files
- [ ] 4.2 GTK provider through fontconfig; fallback fonts loaded from the same files; colour glyphs
- [ ] 4.3 macOS provider through Core Text (callback over the C ABI)

## 5. Background layout

- [ ] 5.1 `Typesetter` and `ChapterLayout` are `Send`; the book is shared through `Arc`
- [ ] 5.2 GTK: neighbouring chapters typeset on a worker thread; boundary turns use them

## 6. Tests

- [ ] 6.1 Unit tests per requirement of `specs/typesetting` (images, areas, tables, fallback routing, pagination)
- [ ] 6.2 Corpus: every page of every book renders; count images, tables, areas

## 7. Verification

- [ ] 7.1 fmt, clippy, tests on macOS and Linux
- [ ] 7.2 Raspberry Pi 5: a book with illustrations and notes in the GTK shell (screenshots); `scripts/pi-headless.sh`
      turns across chapter boundaries at 120 Hz without missed frames inside the animation; bench of the slowest chapter
- [ ] 7.3 macOS: the same book in the AppKit shell (screenshot)
