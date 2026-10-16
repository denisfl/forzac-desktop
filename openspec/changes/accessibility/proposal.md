## Why

The page is drawn from glyph runs, so screen readers (Orca on Linux, VoiceOver on macOS) see an empty widget. On
Android TalkBack reads the page and offers next and previous page (`ReaderScreen.kt`, `semantics`). The parity list
([`docs/android-parity.md`](../../../docs/android-parity.md), row X4) decided to do it after 1.0.

## What Changes

- The page (in "Scroll", the text on screen) exposes its text to the platform's accessibility API, notes markers and
  links as named, activatable elements.
- Next page, previous page and contents are reachable as accessibility actions; the panels' controls have names.

## Capabilities

### New Capabilities
- `accessibility`: what the reader exposes to screen readers.

## Impact

Core: the plain text of a page or a column range. Shells: GTK accessible roles and properties, AppKit accessibility
elements.

## Non-goals

Reading aloud by the app itself.
