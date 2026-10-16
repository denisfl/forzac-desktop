## Why

An image is scaled down to the text column and a table is squeezed into it (`typesetting`), so schematics, maps and
wide tables cannot be read. The Android app opens both full screen with a tap (`ImageViewer.kt`, `TableViewer.kt`);
the parity list ([`docs/android-parity.md`](../../../docs/android-parity.md), rows V1 and V2) decided the desktop gets
one viewer for both, before anything is published.

## What Changes

- `Tab` also stops at images at least a third of the column wide and at tables; `Enter` or a click opens the image or
  the table in a view filling the window, in the reader's theme.
- An image fits the window; a table is set at the text size with columns by content (at most 20 em, longer text wraps
  in the cell), as on Android.
- `+`/`−` zoom (images up to ×5, tables 50–300 %, a table re-typeset so it stays sharp), arrows and `h`/`j`/`k`/`l`
  pan, `0` fits again, the wheel and the touchpad pan, `Esc` or `q` closes and returns to the same page.
- Note markers and links inside a table keep working in the view.
- Works in "Slide" and "Scroll". No new settings.

## Capabilities

### New Capabilities
- `viewer`: opening, zooming, panning and closing images and tables.

### Modified Capabilities
- `reader-navigation`: `Tab` stops at images and tables.

## Impact

Core: image and table areas on pages and in the column, a table typeset unsqueezed at a given size; C ABI for both.
Both shells: the view, its keys and gestures.

## Non-goals

Formulas (not planned for 1.0), rotating images, saving or copying an image.
