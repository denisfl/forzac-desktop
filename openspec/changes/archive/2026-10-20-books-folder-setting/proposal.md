## Why

The Books folder is fixed at `~/Books` / `~/Книги`. To keep the books in Dropbox, iCloud Drive or a Syncthing folder —
and later to sync with the Android app through such a folder (`sync-format`) — the reader has to choose where it is.
Row 20 of docs/roadmap.md; agreed with the maintainer on 2026-10-20: the choice lives in a "⋯" menu in the library's
head, and Forzac asks whether to move the books.

## What Changes

- **A "⋯" button in the library's head**, before the search button, with "Show the Books folder", "Change the Books
  folder…" and "Add a Folder…" (until now only `Ctrl+Shift+O` and a drop on Linux). On macOS File gets "Change the
  Books folder…" too.
- **Changing the folder:** the system's folder dialog, then a question "Move the books to “Dropbox/Книги”?" with
  "Move", "Leave them" and "Cancel". "Move" moves the book files of the old folder (its top level) into the new one,
  keeping positions, bookmarks, highlights, shelves and edits (they are kept by book id), skipping a book whose content
  is already there and renaming on a name clash; "Leave them" uses the new folder as it is and leaves the old one on
  disk. The library then shows the new folder's books.
- A book whose content is in the Books folder opens from there, even if a remembered copy elsewhere still exists.

## Non-goals

- Watching the folder for changes while the library is open; several Books folders; moving subfolders or other files.

## Capabilities

### Modified Capabilities
- `books-folder`: the folder can be changed; the books moved on request.
- `library`: the head's "⋯" menu.

## Impact

- `core/`: `library::change_folder`, `scan` preferring the folder's copy, strings, C ABI, tests.
- `gtk/`, `macos/`: the head's menu, the folder dialog, the question, the move in the background, the library reloaded.
- Docs: `docs/roadmap.md`, the README's line on the Books folder if it names it fixed.
