## Why

Finding a name or a quote in a long book is a common need, and on a keyboard-first reader search is expected on `/`.

## What Changes

- `/` (on macOS also `⌘F`) opens the search panel with its field; the book is searched as the reader types, with the
  Android app's rules: case and ё/е do not matter, soft hyphens and non-breaking spaces do not get in the way, at least
  two characters. The panel lists the matches: a snippet with the match in bold, the section and the position.
- `Enter` (or a click) goes to a match, remembered for the way back; the match is highlighted on the page. `n` / `N` go
  to the next / previous match; `Esc` clears the highlight.
- The search runs in the core on a worker thread; the highlight comes from the core as rectangles on the page or the
  column.

## Capabilities

### New Capabilities
- `book-search`: matching rules, the panel, keys, the highlight, a time budget for a whole book.

### Modified Capabilities
<!-- None. -->

## Impact

Core: `search` (ported from Android's `Search.kt`), match rectangles from the layout (lines remember where each glyph's
text is), C ABI for macOS. Shells: the panel after the contents panel's look, the highlight, `n`/`N`.
