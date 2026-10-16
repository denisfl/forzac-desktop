## Why

The macOS shell from the spike draws pages and turns them; it remembers the position, the theme and the text size.
The Linux shell has since gained the settings, the contents, notes, links with a way back, the "Scroll" mode and the
panels of the Android design. The Mac version has to catch up so it can be read and tested every day.

## What Changes

- Parity with the Linux shell for `reading-state` (recent books when started without a book), `reader-navigation`
  (contents, notes, links, the way back, progress), `page-turning` ("Slide" and "Scroll"), `reader-settings` (the
  settings, the panel, light or dark from the system) and `panels-design` (the panels as in the Android design).
- Keys by key code (any layout, Russian included), the same keys as on Linux; standard menus with the main commands
  and the Mac's own shortcuts (`⌘W`, `⌘Q`, `⌃⌘F`, `⌘,` settings).
- Trackpad: a horizontal swipe turns a page, two-finger scrolling scrolls in "Scroll".
- Open from Finder: document types for FB2, FB3, EPUB; double-click, "Open With", the Dock icon; recent documents.
- The C ABI grows by the calls these need; no binding generator (a few dozen plain functions stay readable).

Search arrives with `book-search` in both shells. Signing, notarisation, a DMG and a Homebrew cask need an Apple
Developer ID and move to their own change, `macos-release`.

## Capabilities

### New Capabilities
- `macos-app`: what the Mac shell does beyond the shared specs: keys, menus, trackpad, Finder.

### Modified Capabilities
<!-- None: the shared specs already describe the behaviour; macOS implements them. -->

## Impact

`core/src/ffi.rs` and `macos/forzac.h` grow; `macos/` splits into several Swift files; the `.app` bundle carries the
fonts. The Linux shell takes the book's contents from the core (one implementation for both shells).
