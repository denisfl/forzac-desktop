## Why

Finding a name or a quote in a long book is a common need, and on a keyboard-first reader search is expected on `/`.

## What Changes

- `/` opens a search field; `Enter` searches the whole book, insensitive to case and to ё/е; results list chapter and
  a snippet.
- `n` / `N` go to the next / previous match, laid out in the current settings, with the match highlighted on the page;
  `Esc` clears the highlight.
- The search runs in the core off the main thread and streams results.

## Capabilities

### New Capabilities
- `book-search`: matching rules, results, keys, highlight, a time budget for a whole book.

### Modified Capabilities
<!-- None. -->

## Impact

Core: a search module over the model (ported rules from Android's `Search`), match rectangles per page. Shells: the
field, the list and the highlight.
