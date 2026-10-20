## Why

On a Mac the book window carries a title bar that never goes away: 28 points of chrome above the page, the only
thing on screen that is not the book. Apple Books, the reader every Mac user has, puts the page under a transparent
title bar and shows a toolbar — the window buttons, the book's title, contents, appearance, search, bookmark — only
while the pointer moves; left alone, the window is the page. The UX review ([`docs/ux-review.md`](../../../docs/ux-review.md),
W1, W2, O1, B1) found the same idea in Foliate and Readest, and the maintainer chose it as the first change from the
review (2026-10-20).

## What Changes

- On macOS the page fills the book window up to its top edge, under a transparent title bar with no title drawn.
- A reading bar over the page's top margin, without a background: the window buttons and "Back to library" at the
  left, Contents, Text settings ("Aa"), Search and Bookmark at the right — the actions of `o`, `s`, `/` and `b`. The
  page's own running head and ribbon stay in sight between them, so the bar repeats nothing the page shows (the
  maintainer, 2026-10-20, after a first version with the titles over a paper strip).
- The bar appears when the pointer moves in the window and fades 2 s after it stops (not while it rests on the bar);
  a page turned from the keyboard or a click on the page hides it at once. The pointer hides with it. A keyboard
  reader never sees the bar unless asked (`Return` opens the reading menu, which shows it).
- One top bar: on macOS the reading menu (`reader-menu`) shows this bar, held while the menu is open, instead of its
  own top bar; the bottom bar with the slider stays as it is.
- The same in full screen. Linux does not change.
- No new settings, keys or core API.

## Capabilities

### Modified Capabilities
- `macos-app`: the book window's top edge and the reading bar.
- `reader-menu`: on macOS the menu's top bar is the reading bar.

## Impact

macOS shell only (`macos/`): the book window's style, a toolbar, a paper strip under it, pointer tracking, the menu's
top bar. The page's 56 px top margin already holds only the running head, so the 52-point bar covers no text and the
page is not re-typeset when the bar comes or goes. The GTK shell and the core are untouched.

## Non-goals

The same bar on Linux (GTK's header bar is a separate decision); moving the panels (contents, search, settings) under
the bar; customising the toolbar; Apple Books' edge arrows (review W9).
