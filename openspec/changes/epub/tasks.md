## 1. Entry point

- [ ] 1.1 `forzac_core::Error` (`NotABook`, `NoText`, `Protected`) and `parse(bytes)` with detection by content
- [ ] 1.2 Shared zip unpacking with the 256 MB limit; shells and the C ABI use `open` as before

## 2. FB3

- [ ] 2.1 Package: relationships, part names, description metadata, core properties, cover, images
- [ ] 2.2 FB2 reader: `fb3-body` and the FB3 elements, images through a resolver
- [ ] 2.3 Tests ported from Android's `Fb3ParserTest`

## 3. EPUB

- [ ] 3.1 XHTML reader: blocks, kinds, styles, lists, pre, tables, images, anchors, note references, hidden asides
- [ ] 3.2 Package: container, OPF metadata and cover, manifest, spine, CSS classes, DRM check
- [ ] 3.3 Contents from nav or NCX; chapter assembly; heading normalisation
- [ ] 3.4 Notes, notes documents, link targets, broken references and links as text
- [ ] 3.5 Tests ported from Android's `EpubParserTest` (all but MathML and fixed layout)

## 4. Corpus and verification

- [ ] 4.1 Corpus test over EPUB and FB3 with Android's checks, typesetting every page
- [ ] 4.2 Parse time of the largest corpus EPUB on the Raspberry Pi 5 (≤ 500 ms)
- [ ] 4.3 An EPUB and an FB3 book in the GTK shell on the Pi and in the macOS shell (screenshots)
- [ ] 4.4 fmt, clippy, tests on macOS and Linux
