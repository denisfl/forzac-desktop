## Why

The desktop app is meant to be the same reader as the Android app, with the keyboard in place of touch. Today it
differs in features and behaviour in ways nobody has listed, so it is not ready to publish, and testing on the Pi 5
and the Mac has nothing to test against.

## What Changes

- A parity list ([`docs/android-parity.md`](../../../docs/android-parity.md)): every reader feature and behaviour of
  the Android app — opening books, typesetting, page turning, contents, notes and links, settings and their presets,
  themes, reading position, search, progress, what happens on errors — with how the desktop app does it.
- Each difference decided with the maintainer (2026-10-16): **intended** (keyboard instead of touch, no PDF, no
  "Curl" and "Cover", the items in "Not planned for 1.0", all confirmed) or a **gap**.
- The small gaps closed here:
  - the language of a book without one detected from its text (moved here from `localization`);
  - code in a monospace face (PT Mono, bundled) and strikethrough drawn;
  - a click in the left or right third of the page turns it;
  - an end page after the last page that marks the book finished;
  - untitled chapters listed in the contents as "Chapter N";
  - a link to a short target (a figure, a table, a paragraph) opens in the note sheet;
  - after a jump, a "← Back" pill as on Android, over the existing `Ctrl+O` stack;
  - search in table cells;
  - a 200 ms cross-fade when the theme changes;
  - the recent-books window in the reader's theme, with the same keys in both shells;
  - a message with the reason when a file cannot be opened;
  - GTK shows the recent books after the last book closes and brings an open book forward, as macOS does;
  - the screen stays on while reading, for 10 minutes after the last key.
- The larger gaps split into their own changes: `viewer` (images and tables in a full-window view, milestone 0.4),
  `two-page-spread` (1.0), `accessibility` (after 1.0).
- A by-hand pass on the Pi 5 and the M1 Mac against the list, with the results recorded in it.

## Capabilities

### New Capabilities
- `reader-window`: opening a file and saying why it cannot be read, one window per book, what follows closing the
  last book, keeping the screen on while reading.

### Modified Capabilities
- `typesetting`: the book's language when it names none; code and strikethrough.
- `page-turning`: clicks in the side thirds turn pages.
- `reader-navigation`: untitled chapters in the contents; short link targets in the note sheet; the "← Back" pill;
  the end of the book.
- `book-search`: table cells are searched.
- `reader-settings`: the theme cross-fades.
- `reading-state`: the recent books in the reader's theme, the same keys on both shells.

## Impact

- Core: language detection, a bundled monospace font, strikethrough strokes, `Book::contents`, link previews, the end
  page, search over table rows, open errors with reasons, a finished position; C ABI additions for the Mac.
- Shells (GTK and AppKit): clicks, the pill, the preview sheet, the end page, the cross-fade, the recent-books window,
  error messages, the idle inhibitor, the window per book.
- `assets/fonts/pt-mono` (OFL) and the packaging scripts that list the font directories.
- `docs/roadmap.md` and `openspec/changes/localization/proposal.md`; new proposals for `viewer`, `two-page-spread`
  and `accessibility`.

## Non-goals

Everything the parity list marks intended or decides to keep: no PDF, no library screen, no selection or highlights,
no formulas, no fixed layout, no menu or progress slider, no whole-book page numbers, no opening animation, no
generated covers, no new settings, hyphenation for Russian and English only.
