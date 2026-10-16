## Why

A full-screen window on a landscape monitor (the Pi 5's display, a MacBook) shows one column of at most the measure
with wide empty margins. The Android app shows two pages side by side on a tablet in landscape (`tablet-layout`), like
a printed book; the parity list ([`docs/android-parity.md`](../../../docs/android-parity.md), row M6) decided the
desktop does the same in its own change.

## What Changes

- When the window is wider than tall and holds two text columns of the current measure with their margins, "Slide"
  shows two consecutive pages side by side; turning moves by a spread, the slide animation moves the spread.
- Each page keeps its own running head and foot; notes, links, `Tab` focus, search highlights and the pill work on
  both pages.
- Resizing into or out of a spread keeps the reading position: the spread (or page) shown holds the line that started
  the previous one. A chapter's first page may be the right page of a spread.
- "Scroll" stays one column. No new settings.

## Capabilities

### Modified Capabilities
- `page-turning`: spreads in "Slide".
- `reader-navigation`: focus, notes and links across the two pages.

## Impact

Shells only, as far as possible: the core already typesets a page for a width; the shells lay out two pages and turn
by two. Measured on the Pi 5 at 120 Hz like a single page.

## Non-goals

Spreads in "Scroll", a setting to force one or two pages.
