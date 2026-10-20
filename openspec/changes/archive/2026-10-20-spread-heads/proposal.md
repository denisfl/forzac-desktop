## Why

A spread (`two-page-spread`, 0.11.0) repeats the chapter's title over both pages and hangs the ribbon of a bookmark on
the left page in the gutter. The maintainer saw both on the M1 (2026-10-20): the double head is noise, and a ribbon in
the middle of the spread looks wrong. A printed book puts the book's title over the left page and the chapter's over
the right one, and a ribbon hangs from the spread's outer edge.

## What Changes

- On a spread the left page's running head SHALL be the book's title, the right page's the chapter's title (none on a
  chapter's first page, as today). A single page keeps the chapter's title.
- On a spread the ribbon SHALL always hang beside the right page's column, in the outer margin, whichever of the two
  pages holds the bookmark; the corner that sets and removes it stays the right page's.
- No new settings.

## Capabilities

### Modified Capabilities
- `reader-navigation`: the running heads of a spread.
- `bookmarks`: the ribbon of a spread hangs over the right page.

## Impact

Core: the page style knows it is a spread's page, the chapter layout keeps the book's title for the left head
(`core/src/layout`, `core/src/settings.rs`); the C ABI does not change. Shells: the ribbon's offset (`macos/PageView.swift`,
`gtk/src/page_view.rs`). Single-page windows and "Scroll" do not change.

## Non-goals

Authors in the head, heads that follow a part/chapter hierarchy, a ribbon per page.
