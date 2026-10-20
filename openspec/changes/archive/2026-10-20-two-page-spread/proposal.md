## Why

A full-screen window on a landscape monitor (the Pi 5's display, a MacBook) shows one column of at most the measure
with wide empty margins. The Android app shows two pages side by side on a tablet in landscape (`tablet-layout`), like
a printed book; the parity list ([`docs/android-parity.md`](../../../docs/android-parity.md), row M6) decided the
desktop does the same in its own change.

## What Changes

- When the window is wider than tall and each half holds a text column of at least 28 em with its margins, "Slide"
  shows two consecutive pages side by side; turning moves by a spread, the slide animation moves the spread. 28 em,
  not the full measure: a full-screen 13" MacBook (1440 px) gives 31 em halves at the default size.
- Each page keeps its own running head and foot ("37 of 212", "38 of 212"); notes, links, `Tab` focus, search
  highlights, the selection and the pill work on both pages. A bookmark marks the spread; its ribbon hangs over the page
  that holds it.
- Resizing into or out of a spread keeps the reading position: the spread (or page) shown holds the line that started
  the previous one. Every chapter starts on the left page; a chapter ending on a left page leaves the right half
  empty (as Apple Books does): pages are typeset chapter by chapter and the book's page numbers are known only after
  the background count, so Android's whole-book pairing would re-pair the spreads once the count ends.
- "Scroll" stays one column. No new settings.

## Capabilities

### Modified Capabilities
- `page-turning`: spreads in "Slide".
- `reader-navigation`: focus, notes, links, selection and page numbers across the two pages.
- `bookmarks`: a bookmark marks the spread.

## Impact

The core decides when the window shows a spread and the page style for it (one rule for both shells, also used by the
background page count); the shells draw two pages and turn by two. Measured on the Pi 5 at 120 Hz like a single page.

## Non-goals

Spreads in "Scroll", a setting to force one or two pages.
