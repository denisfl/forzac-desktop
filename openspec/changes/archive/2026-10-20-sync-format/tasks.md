# Tasks

## 1. Core

- [x] 1.1 Places: a local `Locator` to a portable place (code points, fragment) and back (exact, search, clamp); tests
- [x] 1.2 The file: read, write (escaping, atomic), the device id; tests
- [x] 1.3 Local changes against the own file; merge; applying remote items to the state (positions, bookmarks,
      highlights, shelves, edits), books missing kept for later; tests with two devices in one folder
- [x] 1.4 C ABI `forzac_sync`; `docs/sync-format.md`

## 2. Shells and checks

- [x] 2.1 GTK and macOS: sync before the library's scan
- [x] 2.2 Both builds and clippy green; by hand: two states on the Mac sharing one Books folder (two "devices"): a
      book read to page 5 and bookmarked on A; on B the library shows it continued, and it opens on the same pages with
      the bookmark; two `.sync` files. Not tried: two real computers through Dropbox, Android (not implemented yet)
- [x] 2.3 `docs/roadmap.md`, README
