# Proposal

## Why

`?` shows the keys as one block of plain text on a grey rectangle: lines of different lengths, keys and their meaning
run together, the text over the page shows through, and in the GTK shell it is the Adwaita OSD style, not Forzac's. The
maintainer asked on 2026-10-17 for the help to be made properly once the reading menu and the library were done, in
the web reader's style, so that the same help can later go to the web reader if it gets keys.

## What Changes

- The help becomes a card in the middle of the window over a dimmed page, in the reader's theme (the web reader's
  sheet colour, line and muted text, 22 px corners): a "Keys" title with a close button, the keys grouped in sections
  (Reading, Moving, Book, Selection, View, Mouse, Picture or table), each key drawn as a key cap, the meaning beside it;
  two columns when the window is wide enough, one otherwise; the support line at the foot, its address a link.
- `?`, `Esc`, a click outside the card and the close button hide it; it fades in and out.
- The core keeps the help as data (sections of rows of keys and a meaning) instead of two text blocks, translated row
  by row; both shells draw from it. The keys themselves are not translated.

## Capabilities

### New Capabilities
- `help`: what `?` shows in both shells, how it is laid out, how it closes, and the support line.

### Modified Capabilities
- `linux-integration`: its "Help and the support line" requirement moves to `help` (the support line's rule is kept
  there unchanged).

## Non-goals

- The web reader's help. The card is described so that it can be built there one to one (`design.md`), but the web
  reader has no keys yet and is another repository.
- Changing any key. The help lists the keys as they are.
- A searchable or customisable key list.

## Impact

- `core/src/i18n.rs` (the help as sections, the Russian rows), `core/src/ffi.rs` and `macos/forzac.h` (the help as a
  flat text the Mac parses), `gtk/src/help.rs` (new), `gtk/src/main.rs`, `gtk/src/panels.rs` (theme CSS),
  `macos/Help.swift` (new), `macos/main.swift`; README if it shows the help.
