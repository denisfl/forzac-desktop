## Why

The maintainer reads on Android, the Mac and the Pi 5 and wants the place, bookmarks and highlights to follow them,
through a Dropbox-like service and no server of our own (docs/roadmap.md, 22). With the Books folder choosable
(`books-folder-setting`) and both apps naming books by the same id (`book-identity`), a shared folder can carry the
reading data too.

## What Changes

- A sync folder inside the Books folder: `.forzac/`, synced with the books by whatever syncs the Books folder.
- Each device writes only its own file `<device>.sync` and reads all of them; per item the latest change wins;
  removals are kept as tombstones. No conflicts for the sync service to resolve.
- What syncs: the reading position, bookmarks, highlights, shelves and edits of book details. Places travel as
  chapter, block, offset in code points and a text fragment, so a place is found even when the Android and desktop
  parsers number blocks differently.
- When: every time the library shows, off the UI thread. Books whose files are not there yet get their data once they
  arrive.
- `docs/sync-format.md` documents the format for the Android implementation.

## Non-goals

- The Android side (a change in the Android repository, by `docs/sync-format.md`); syncing settings or the reading
  log; live sync while a book is open; compacting tombstones; choosing a separate sync folder.

## Capabilities

### New Capabilities
- `sync`: the sync folder, the files, merging and applying.

## Impact

- `core/`: `sync` module (places, files, local changes, merge, apply), a few `Store` accessors; C ABI; tests.
- `gtk/`, `macos/`: a call before the library's scan.
- Docs: `docs/sync-format.md`, `docs/roadmap.md`, README.
