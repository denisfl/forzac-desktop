## Context

Today `core/src/epub/xhtml.rs` reads MathML as ordinary inline text, dropping annotations (`SKIP`). Android typesets formulas with its own engine: `model/Math.kt` (132 lines), `typesetting/MathLayout.kt` (352) and `MathDraw.kt`, with `FormulaViewer.kt`, from its archived change `2026-10-16-math`. That change's corpus was 10,281 formulas of IDPF's linear algebra sample, every one inline. None of the 28 EPUBs on this machine has a formula, and books are not to be downloaded, so the test material here is a hand-written EPUB built by the tests, covering each element.

What constrains the port:

- `BlockLayout` has one `line_height` for all its lines (`core/src/layout/mod.rs:225`, height = lines × line height), and pagination counts on it.
- The core shapes and the shells rasterise the same font files, so Android's use of the system's Noto Sans Math has no direct equivalent.
- A page is runs, images, rules, areas and strokes, all rectangles except glyphs. A stretched bracket or a root sign is neither.
- The iOS shell reads the same C ABI, from a pinned revision of the core.

## Goals / Non-Goals

**Goals:** a textbook's formulas read on the desktop as on Android; a book without formulas typesets exactly as before; the same model and offsets as Android, so that places sync.

**Non-Goals:** those of the proposal.

## Decisions

### 1. A formula is one character (core)

As on Android: U+FFFC with `Style::Math` whose key is in `Book::math`; `Block::Formula { key }` for a display one. Offsets, selection, search and hit testing stay character-based and agree with Android's. Rejected: inline nodes in `StyledText`, which every consumer of the text would have to learn.

### 2. The tree in the core's model, its layout beside the typesetter (core)

`core/src/math.rs`: `MathNode`, `Formula`, the linear text, a port of `Math.kt` with its tests. `core/src/layout/math.rs`: a port of `MathLayout.kt` with the same simplified rules, using the `Shaper` for tokens. Its output is a box with an ascent, a descent, a width and a list of drawing items relative to the origin. Porting rather than redesigning keeps both platforms' formulas alike.

### 3. Polylines on a page, across the C ABI (core, shells)

`PageContent` gains `paths: Vec<MathPath { points, width, ink }>`, and `ForzacPage` gains `paths` at its end. Fraction bars and matrix lines stay `Stroke`s. Shells draw a polyline with their line API (GSK path, `CGContext`). Rejected: stretched glyph variants from the math table, which need OpenType MATH support that HarfRust's shaping does not lay out for us, and differ from Android's drawing.

The iOS shell keeps building until it updates its pin; after that it has to draw `paths`, or brackets go missing. Its session is told when this lands.

### 4. Per-line heights only where needed (core)

`TextLayout` gains optional lists of line tops and baselines, empty in a paragraph without formulas, where the existing arithmetic stays, in the same order of additions. So a novel's code path, and its pixels, stay the same: hashes of what 30 real books draw on 9,187 pages, in paged and scrolling modes, are equal before and after. Pagination asks the layout for a span of lines instead of multiplying. Rejected: a list for every block, which costs memory in every book for the sake of a few.

### 5. The bundled math font: STIX Two Math (assets, core, shells)

Candidates, all OFL:

| Font | Style | For | Against |
|---|---|---|---|
| STIX Two Math | serif, made for scientific publishing | matches Literata, PT Serif and Source Serif; the fullest coverage | the largest of the three |
| Libertinus Math | serif | smaller, handsome | a narrower range of symbols |
| Noto Sans Math | sans | what Android falls back to | sans symbols in a serif book |

Recommendation: STIX Two Math, for a printed book's look. Letters and digits stay in the book's face, as on Android; only what the face lacks comes from it. The maintainer chose it on 2026-10-31. The TTF of the tag `v2.13b171` of `stipub/stixfonts` is 1.5 MB, 0.8 MB compressed. `fonts::font_for` is untouched for ordinary text; the formula layout asks for the book's face, then this font, then the platform's fallback.

GTK adds the file to its fontconfig set; the Mac makes a `CGFont` from the core's bytes, as for any bundled font. The TTF, not the smaller OTF, as a token's height comes from its glyphs' boxes in `glyf`.

### 6. Wide formulas shrink, and open in the view (core, shells)

The Android corpus found formulas wider than a phone's column; a window is wider, but a spread's column is not. As on Android: scaled down to the column, to 0.55 at the least. Only when shrunk it gets an `Area` with `Target::Table(at)`, `at` being the formula's place, and `layout_table` gives the view the formula when `at` is one. So the shells' viewers and the C ABI need nothing new, the iOS shell included: a click and `Tab` reach it as they reach a table. In a table's cell a formula has no area of its own, as the table's area opens the view.

### 7. The linear text wherever text leaves the page (core)

`Book::text`, which quotes and copying read, `Book::linear_text` for the C ABI's plain text of a note and of a short link target, and the search's snippets and the Kindle's writer each replace U+FFFC by the formula's linear text. A note's panel is typeset like the page, so there a formula stays a formula (on Android its panel has no such layout). The search runs over the raw text, where a formula is a character no query matches, so that offsets stay right.

### 8. Migration of places

No migration: in books with MathML an old offset lands within a few characters, and a block index after a display formula moves by the formulas before it in the chapter. Android's offsets are the new ones, so sync gets better. Only books with MathML are touched, and until now they were unreadable. The book's id does not depend on the parse, so nothing else moves.

## Risks / Trade-offs

- [A wrong port draws a formula wrongly, silently] → Android's `MathLayoutTest` and `MathTest` are ported as they are, and a hand-written EPUB covers each element; screenshots beside Android's output of the same book.
- [Slow chapters: a textbook chapter may hold a thousand formulas] → a shaping cache per token, measured with the `bench` example against the budgets.
- [The iOS shell misses brackets until it draws paths] → additive in the ABI, and told when it lands.
- [Package size] → measured before the font goes in.
- [No real textbook to test with] → the hand-written EPUB is not a publisher's MathML. If the maintainer has such a book, it goes through by hand at the end (task 7.5).

## Open Questions

- Whether formulas are in 1.0 or after it: this change does not depend on `release-1-0`, and a 0.20.0 can carry it. For the maintainer at release time.
