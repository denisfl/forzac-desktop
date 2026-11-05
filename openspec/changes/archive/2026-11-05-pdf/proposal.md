## Why

The maintainer's goal since 2026-10-30 is the same features on every platform, allowing for what each platform is. PDF is the most visible gap: the Android app reads PDF, the desktop refuses it ("Not planned for 1.0" since `android-parity`). The maintainer reversed that on 2026-10-30: simple PDF support, without themes, as on Android.

## What Changes

- **PDF opens as a book.** From the library, the file dialog, the command line, Finder and file managers; it is copied into the books folder and shown on the shelf like any book.
- **Pages as on paper.** Original colours on white, whatever the theme. Menus and panels over a PDF use the Light theme; the chosen theme is untouched and returns with the next book.
- **Turning.** "Slide" and the same keys, clicks and wheel as a book; in "Scroll" mode a PDF still slides, as on Android. The foot shows "N / M".
- **A page fits the window**, two pages side by side when the window is wider than tall and holds them, by the existing `spread` rule.
- **Zoom.** `+` `−` `0`, a pinch or `Ctrl`+wheel, 1× to 4×, sharp after the gesture; arrows, wheel or drag pan.
- **Menu and contents.** The slider by page, the PDF's outline when it has one; "Aa" offers no text settings.
- **Search and links.** Search in the page text, as in a book; links inside the file and external ones, with the "← Back" pill.
- **The place in a PDF** is remembered on this device. As on Android it does not sync; the day's reading log does.
- No new settings.

Not in this change, each its own later change if wanted: Android's margin cropping, splitting scanned spreads and tall pages into screens; selection and highlights in a PDF.

## Capabilities

### New Capabilities
- `pdf-reading`: PDF pages, turning, zoom, menu, search and links on the desktop.

### Modified Capabilities
<!-- `library` and `books-folder` take PDF as a book type; their requirements change by the file types they list. -->

## Impact

- `core`: recognises PDF by content and reports it as a type of its own instead of `NotABook`; no PDF parsing in the core. Book id, state, books folder and reading log take a PDF as they take any book.
- `macos/`: a PDF window on PDFKit.
- `gtk/`: a PDF window on PDFium (see design); a new runtime library in the Linux packages.
- `forzac-ios` gets the same behaviour on PDFKit in its own `pdf` change; its roadmap said "in theme colours" and is corrected.
- `docs/roadmap.md`, `CLAUDE.md`, `docs/android-parity.md`: PDF is no longer "not planned".
