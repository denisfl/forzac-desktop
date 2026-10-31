## 1. The font

- [x] 1.1 STIX Two Math from its official release into `assets/fonts/`, with `OFL.txt` — 1.5 MB, 0.8 MB compressed; TTF from the tag v2.13b171. Check: it loads under HarfRust, GTK's fontconfig set and `CGFont` (the shells at 5 and 6)
- [x] 1.2 In every package: `macos/build.sh`, `scripts/package-linux.sh`, `scripts/package-deb.sh`, both PKGBUILDs; the licence beside the others — check: the listing of each archive

## 2. The model

- [x] 2.1 `core/src/math.rs`: `MathNode`, `Formula`, operator classes, the linear text, from `Math.kt` — check: `MathTest` ported
- [x] 2.2 `Style::Math`, `Block::Formula`, `Book::math`, handled everywhere blocks are matched — check: `cargo test -p forzac-core`
- [x] 2.3 `core/src/epub/xhtml.rs` reads `math` with and without a prefix, `semantics`, the alone-in-a-paragraph rule, a malformed formula — check: tests on a hand-written EPUB with every element; the corpus test passes

## 3. Typesetting

- [x] 3.1 Per-line tops in `BlockLayout` and pagination by a slice's height — check: 30 real books, page contents hashed before and after: identical
- [x] 3.2 `core/src/layout/math.rs`: tokens, rows, fractions, scripts, limits, accents, roots, fences, matrices, spaces, from `MathLayout.kt` — check: `MathLayoutTest` ported
- [x] 3.3 The formula as a box in a paragraph's line breaking, with no break inside and none before closing punctuation; the display block centred — check: layout tests
- [x] 3.4 Shrinking to the column, `Target::Formula` only when shrunk, `layout_formula` for the view — check: layout tests
- [x] 3.5 `paths` in `PageContent`, in `ForzacPage` and `macos/forzac.h` — check: an FFI test

## 4. Text leaving the page

- [x] 4.1 `plain_with_formulas`, in quotes, copying, notes, the screen reader's note text, search snippets, the Kindle — check: core tests for each, a MOBI without U+FFFC
- [x] 4.2 The search over text with formulas finds both sides, and offsets stay right — check: core tests

## 5. macOS

- [x] 5.1 Drawing paths and the math font on the page — check: `--png` of the test EPUB, in Light and Dark
- [x] 5.2 The view of a shrunk formula, by click and `Tab`, `Enter`, zoom and panning; a formula that fits turns the page — check: `--click`, `--keys`, screenshots

## 6. Linux

- [x] 6.1 The same in GTK — check: `clippy`, screenshots under headless Weston

## 7. Check

- [x] 7.1 `cargo fmt`, `clippy`, `cargo test -p forzac-core`, the corpus test; `openspec validate formulas --strict`
- [ ] 7.2 The test EPUB beside Android's output of the same file — check: the maintainer sees both, as screenshots. The desktop's are taken; Android's are not
- [ ] 7.3 Measurements on the M1 MacBook Pro: `bench` on a chapter of 1,000 formulas; `scripts/measure.sh` and `--turns 20` on pages of formulas against a novel. Mac, 2026-10-31: a chapter of 1,000 formulas 20–22 ms cold; three runs of 20 turns at 120 Hz, 2, 3 and 7 late of about 460 (max 14.5 ms) against 5, 6 and 6 for a novel (max 14.3 ms). The Pi 5 when its waiver is lifted
- [x] 7.4 `docs/android-parity.md` F7, A14, NP5; `docs/roadmap.md`; README; the iOS session told about `paths` when this lands
- [ ] 7.5 A real textbook with MathML, if the maintainer has one, by hand — check: the maintainer's word
- [ ] 7.6 Release notes; archive
