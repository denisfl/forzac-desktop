## Why

A book with MathML reads as a run of letters: `<msub><mi>a</mi><mn>11</mn></msub>` comes out as "a11", a matrix as a row of numbers, so a textbook is unreadable. Android has typeset formulas since 2026-10-16 (F7, A14 in `docs/android-parity.md`); "not planned for 1.0" predates the goal of 2026-10-30, the same features on all platforms.

## What Changes

- EPUB's MathML becomes a tree in the book's model, as on Android: tokens, rows, fractions, scripts, limits and accents, roots, fences, matrices, spaces. `semantics` takes its first child; annotations stay hidden.
- A formula in a paragraph is one character of the text (U+FFFC), so `Locator`s, selection and search go on working, and offsets agree with Android's. A `display="block"` formula, or one alone in its paragraph, is a block of its own, centred.
- The core typesets formulas (a port of Android's `MathLayout`) into what a page already holds, plus one new thing: polylines, for stretched brackets and root signs. Shells draw them in the theme's ink; a theme change is a redraw.
- A line with a tall formula is taller than the others; lines of one paragraph no longer all have one height.
- A formula wider than the column shrinks to fit, to 0.55 at the least, and a click or `Tab` and `Enter` opens it in the existing image and table view, at the book's size, with its zoom and panning. A formula that fits is not a click target.
- Quotes, copying, search results, a note's screen-reader text and the Kindle's MOBI use the formula's linear text (`a/b`, `x_i`, `[a b; c d]`).
- A math font joins the bundled fonts, used only inside formulas for what the book's face lacks, so a formula looks the same on every system.
- `docs/roadmap.md` and `docs/android-parity.md` no longer list formulas as not planned.

**BREAKING** for reading state, in books with MathML only: a formula used to be several characters, and a display formula part of a paragraph, so a saved place, bookmark or highlight after one moves by a few characters or a block. Nothing else changes, and the new offsets are Android's.

Non-goals, as on Android: TeX-quality rules (no display style inside a line, simplified spacing), line breaking inside a formula, MathML in FB2 and FB3 (neither format has it), LaTeX in Markdown, the elementary math of `mstack` and `mlongdiv`, a screen reader's reading of a formula beyond its linear text.

No setting is added.

## Capabilities

### New Capabilities
- `math`: formulas in the model, their typesetting, the view of a wide one, and their text in quotes, search and exports.

### Modified Capabilities
- `epub-parsing`: MathML is read as formulas, not as plain text.
- `typesetting`: lines of one paragraph can differ in height; a bundled math font inside formulas.
- `viewer`: the view also opens a shrunk formula.

## Impact

- Core: `core/src/math.rs` (new, the tree and linear text), `core/src/layout/math.rs` (new, typesetting), `core/src/model.rs` (`Style::Math`, `Block::Formula`, `Book::math`), `core/src/epub/xhtml.rs`, `core/src/layout/` (per-line heights, paths, the formula's area), `core/src/search.rs`, `core/src/selection.rs`, `core/src/kindle.rs`, `core/src/fonts.rs`, `assets/fonts/`.
- C ABI: pages gain `paths`; `forzac.h`. The iOS shell has to draw paths too, and its session has to know.
- Shells: drawing paths and opening the view (`gtk/src/page_view.rs`, `gtk/src/viewer.rs`, `macos/Core.swift`, `macos/PageView.swift` and the Mac's viewer).
- Packaging: the font and its licence in the tarballs, the `.deb`, the AUR packages and the app bundle.
