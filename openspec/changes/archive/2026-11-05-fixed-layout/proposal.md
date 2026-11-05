## Why

The maintainer's goal since 2026-10-30 is the same features on every platform. A comic or manga as a fixed-layout EPUB reads on Android (`fixed-layout`) and fails on the desktop as "the book has no text". This is the next open row of `docs/android-parity.md` (F6) after PDF.

## What Changes

- **A picture book opens as a book.** A pre-paginated EPUB whose spine is pictures opens page by page, each page whole on the theme's paper, never recoloured. At most 10% of other pages are skipped; beyond that it reads as an ordinary book.
- **Turning.** Keys, clicks, wheel and "Slide" as in a book; a right-to-left book's next page is on the left.
- **Spreads.** A wide window shows spreads: the first page alone, then pairs, the first of a pair on the right in a right-to-left book.
- **Menu.** The slider by page; the contents when the book has them; the theme by `t` and `T`.
- **Place and sync.** `Locator(0, page, 0)`, kept on this device as on Android; the reading log counts time and pages.
- No new settings.

## Capabilities

### New Capabilities
- `fixed-layout`: EPUB of picture pages on the desktop.

### Modified Capabilities
<!-- `pdf-reading`'s window becomes the shared window for pages; its requirements do not change. -->

## Impact

- `core`: recognises such an EPUB, gives its pages and their sizes through the C ABI.
- `macos/`, `gtk/`: the PDF window draws from a source of pages, a PDF's or a picture book's.
- `docs/android-parity.md`: F6 and NP6.
