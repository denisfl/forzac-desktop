## Context

Android keeps a picture book as one chapter with a block per page (`EpubReader.kt` `pictureBook`, `FixedLayoutScreen.kt`). The desktop has just gained a window for books made of pages: `pdf`. Its zoom, slide, menu, place and log are what a picture book needs.

## Goals / Non-Goals

**Goals:** a pre-paginated EPUB of pictures reads as on Android, with the desktop's keys and window.

**Non-Goals:** fixed-layout pages with text or vector drawings (they read as an ordinary book, as on Android); recolouring; syncing the place; a settings panel in this window.

## Decisions

1. **`Book.pictures`, not chapters.** Pages are a list of image ids (`None`: a blank page), with the direction. Chapters stay empty, as for a PDF, so everything keyed on `Book::page_count` (clamp, progress, log, sync, Kindle) treats both alike. Android's progress rule holds: (page + 1) / pages. Rejected: Android's one chapter of image blocks, which would let the typesetter's paths run on a book they cannot lay out.
2. **One window, two sources of pages.** A source says how many pages, their sizes, whether spreads start at the second page and the direction, and draws a page. `macos/Paged.swift` and `gtk/src/pdf.rs` hold the rest. Rejected: a second window, a copy of about a thousand lines.
3. **Spreads by the rule for books.** The window holds two pages by `Settings::spread`, as in `pdf`; Android's rule is about phones and tablets (the short side from 600 dp), which a resizable window has no use for.
4. **No foot on a picture book.** The whole window is the page, as on Android; "Page N of M" is in the menu.
5. **The theme by `t` and `T`; no "Aa".** Android's "Aa" holds the theme and the turning. The desktop's settings panel belongs to the typesetting view, and a PDF slides in either mode, so a picture book does too. A smaller menu here is a desktop difference, not a gap: the keys are those of a book.
6. **Pictures decode off the main thread**, scaled to what the page needs, neighbours ahead; a zoomed page is decoded at up to its own size.

## Risks / Trade-offs

- [The whole book is in memory: `Book.images` holds every picture's bytes, as for any EPUB] → A 200 MB manga takes 200 MB. Accepted for now: parsing already reads the whole archive, and Android's limit on unpacked size applies.
- [A picture in an unusual format does not decode] → The page shows blank paper; the rest of the book reads.

## Migration Plan

New behaviour for books that failed or read as text before; no data changes. A picture book read as an ordinary book before keeps its id; a saved place is clamped to its pages.
