## Why

Readers keep the lines that struck them and copy text out of books. The Android app has selection, highlights ("quotes")
and their list; the desktop app cannot even copy a word. It was deferred in `android-parity` as mouse-first; the
maintainer has now asked for it, with a keyboard way to select that suits a keyboard-first reader.

## What Changes

- Selecting text with the mouse or touchpad: a drag snaps to words, a double click takes a word, a triple click a
  paragraph, `Shift`+click extends, the ends can be dragged; holding at the page's edge turns the page (in "Scroll"
  the column scrolls) and the selection goes on.
- Selecting from the keyboard like Vim's visual mode: `v` puts a word cursor on the page, `h`/`l` move by words,
  `j`/`k` by lines (turning pages past the edge), `v` anchors the selection, `o` swaps its ends, `Esc` steps back.
- A bar over the selection as on Android: "Highlight · Copy · Share" on macOS (the system share menu), "Highlight ·
  Copy" on Linux (no system share there). `Enter` highlights, `c` or `Ctrl+C`/`⌘C` copies, `s` shares on macOS.
- Highlights (Android's quotes): an ochre marker under the text, drawn in with a stroke; kept in the state file as a
  Locator range, so they survive re-layout, settings changes and restarts. A click on a marker (or `Tab` to it and
  `Enter`) offers "Delete · Copy · Share".
- A "Highlights" tab in the contents panel next to "Bookmarks": the text, the section and the position; `Enter` goes,
  `d` deletes, `c` copies one, `Ctrl+C`/`⌘C` copies all in the Android export format (macOS also shares them).
- `docs/roadmap.md` and `docs/android-parity.md`: selection and quotes leave "Not planned for 1.0"; Q2 becomes a change.

## Non-Goals

Notes or colours on highlights; highlights in notes, previews and the table view; selection in the image or table
view; dictionaries and translation; exporting to files; a library-wide list of highlights.

## Capabilities

### New Capabilities
- `quotes`: selecting text (mouse and keyboard), copying, the selection bar, highlights on the page, their list, export
  and storage.

### Modified Capabilities
- `reader-navigation`: the contents panel gains the "Highlights" tab; highlights are `Tab` stops.
- `page-turning`: a click while text is selected only clears the selection.
- `macos-app`: Edit → Copy (`⌘C`).

## Impact

Core: words of a page and of a stretch of the column with their text ranges and rectangles; word hit-testing and
motions; quotes in the state file; the text, share and export strings; C ABI for all of it. Both shells: selection
input and drawing, the bar, markers under the text, the tab, the clipboard; macOS: `NSSharingServicePicker`.
