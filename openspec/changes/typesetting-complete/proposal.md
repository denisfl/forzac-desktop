## Why

The spike typesets text paragraphs only. Real books have images, footnote markers, links and tables, and contain
characters PT Serif does not have (Greek, math symbols, emoji), which now come out as empty boxes. Chapters are typeset
on the main thread when they are opened. The Android app solves all of these; the desktop core has to reach the same
quality before the reader is usable.

## What Changes

- Images: decoded and scaled in the core (one logical pixel per image pixel, never wider than the column or taller
  than the page), drawn by the shells as textures; never split across pages.
- Footnote markers and links: superscript markers, link ranges; the core reports their positions on the page so shells
  can activate them (used by `reader-navigation`).
- Tables: column widths by content, smaller sizes when they do not fit, rows kept together (port of `TableColumns`).
- Glyph fallback: characters missing from the book face are shaped with a fallback font; the core picks it and the
  shell draws exactly that file (bundled fallback faces first, system fonts through a narrow platform interface).
- Pagination rules completed to Android parity (epigraphs, verse, headings kept with text, chapter start lowered).
- Layout off the main thread: the current chapter first, then its neighbours in the background; a cache keyed by
  book × settings × window size; re-layout on resize keeps the Locator.

## Capabilities

### New Capabilities
- `typesetting`: line breaking, hyphenation, justification, pagination rules, images, tables, notes and links on the
  page, glyph fallback, layout performance budgets.

### Modified Capabilities
<!-- None: the spike has no specs. -->

## Impact

`core/src/layout.rs` grows into a module; new image decoding (pure Rust) and fallback-font selection in the core; the
glyph-run output gains image runs and active areas; both shells draw images. The C ABI grows accordingly.

## Non-goals

Formulas (MathML), fixed-layout books, hanging punctuation (a later polish change).
