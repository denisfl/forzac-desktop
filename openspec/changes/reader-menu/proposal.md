## Why

On 2026-10-17 the maintainer looked at the Mac build next to the Android app and the web reader
(forzac.truenrth.app/read/) and found the desktop reader unlike both: no reading menu, no visible chapter navigation,
no page numbers of the book. They decided to make the desktop reader like the web reader, one to one, keeping the
desktop's keys. This change brings the web reader's menu; the library screen is a parallel change (`library`).

It reverses two `android-parity` decisions (docs/android-parity.md): R1 ("keep chapter pagination and `4 / 22 · 37%`")
and R3 ("no menu and no time estimate on desktop").

## What Changes

- **A reading menu** over the page, as the web reader's (`#bar-top`, `#bar-bottom`) and Android's `ReaderMenu`: a click
  in the middle third of the page or `Enter` (no `Tab` focus) opens it; a click on the page or `Esc` closes it. Top bar:
  back to the library, the chapter title and the book title, the contents button. Bottom bar: a slider over the whole
  book with chapter ticks (dragging shows the page's number and chapter, releasing jumps there, remembered for `Ctrl+O`
  like a contents jump), "Page N of M", "N min left in chapter" (1000 characters a minute, as on Android), the "Aa"
  button (the settings sheet). From the keyboard: `←`/`→`/`h`/`l` move the slider, `Enter` jumps, `Esc` closes.
- **BREAKING (keys):** `Enter` without a focused marker opens the menu instead of turning the page (in "Scroll":
  instead of scrolling a screen). Every other key stays.
- **A click in the middle third** of the page opens the menu (it did nothing); in "Scroll" too.
- **Whole-book pages:** the book is typeset in the background, chapter by chapter, for the current window and settings
  (as Android and the web do). Once counted, the foot shows "N of M" for the whole book, the menu its slider and page,
  the contents and bookmarks lists page numbers, the highlights and search lists "section · p. N". Until then (and
  again after a resize or a settings change, until recounted) they show what they show now.
- **"Back to library"** closes the book's window and shows the recent books (the library once `library` lands).
- Docs: the reversal of R1 and R3 in docs/android-parity.md and docs/roadmap.md; the help texts.

## Non-Goals

Restyling the help overlay (after this change); the running head (R2 stays); a page-number setting; the "← Back"
pill's wording; Android's opening animation; caching counted pages across sessions (the web keeps them in IndexedDB;
here a count is cheap enough to redo per window); the library itself.

## Capabilities

### New Capabilities
- `reader-menu`: the reading menu (bars, slider, keys, mouse) and the background count of the book's pages it reads.

### Modified Capabilities
- `page-turning`: `Enter` no longer turns or scrolls; a click in the middle third opens the menu.
- `reader-navigation`: `Enter` without focus opens the menu; the foot shows the book's page once counted; the contents
  show page numbers once counted.
- `bookmarks`: the list shows page numbers once counted.
- `quotes`: the highlights list shows "section · p. N" once counted.
- `book-search`: matches show "section · p. N" once counted.

## Impact

Core: a whole-book page count (`core/src/layout/`), the foot with book pages, the time left in a chapter, the section
of a place; their C ABI (`core/src/ffi.rs`, `macos/forzac.h`); new strings in `core/src/i18n.rs` and the help texts.
GTK: a menu module, the page view's background count, the window's keys and clicks, the lists in `panels.rs`.
macOS: a menu view, `PageView.swift` count and keys, `Panels.swift` lists, `main.swift` (back to the recent books).
Shared with `library` (keep small): `gtk/src/main.rs` and `macos/main.swift` around the recent books.
