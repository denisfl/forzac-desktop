## Why

Readers mark places to return to. On the keyboard a single key is the natural way.

## What Changes

- `b` toggles a bookmark on the current page (at its first line's Locator); a small mark on the page shows it.
- The contents panel (`o`) gets a bookmarks list; `Enter` jumps, `d` deletes.
- Bookmarks survive re-layout, settings changes and restarts (stored through `reading-state`).

## Capabilities

### New Capabilities
- `bookmarks`: adding, removing, listing, persistence.

### Modified Capabilities
- `reader-navigation`: the contents panel gains the bookmarks list.

## Impact

Storage and C ABI calls in the core; the mark and the list in both shells.
