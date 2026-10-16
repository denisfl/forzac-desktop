## Why

Every launch opens a book at its first page. A reader must reopen a book exactly where it was left, and give quick
access to the last books opened. Settings chosen in `reader-settings` need the same storage.

## What Changes

- The reading position (Locator) is saved per book whenever it changes (debounced) and on close, and restored when
  the book is opened again, after any change of settings or window size.
- Books are identified by content (a hash), so a renamed or moved file keeps its position.
- Recent books: opening Forzac without a file shows the last books with progress; `Enter` opens one.
- Storage in the core, in the platform's data directory (`$XDG_DATA_HOME/forzac`, `~/Library/Application Support/
  Forzac`); the format is chosen in design (SQLite through `rusqlite` or a small file).

## Capabilities

### New Capabilities
- `reading-state`: what is remembered per book and globally, where, and when it is written.

### Modified Capabilities
<!-- None. -->

## Impact

New storage module in the core and C ABI calls; both shells save and restore. No import step: books stay where they are.

## Non-goals

A library screen with covers and import, syncing between devices.
