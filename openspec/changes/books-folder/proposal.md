## Why

Forzac reads every book where its file lies and remembers that path. A book opened from Downloads disappears from the
library when Downloads is cleaned up, and on macOS reopening it from the library asks for access to the Downloads
folder. The Android app copies every book into its own storage; the desktop app should keep its books the same way, in
one visible folder the reader owns — the base for the Calibre-like library, for sending books to a Kindle and, later,
for syncing through Dropbox or a similar service.

## What Changes

- A **"Books" folder** in the home folder (`~/Books`, or `~/Книги` when the interface is Russian), created on the
  first start of this version. Its place is fixed and remembered in the state. A folder of that name that already
  exists is used as it is: its books join the library, nothing in it is renamed or moved, subfolders and other formats
  are left alone.
- **Opening a book from anywhere** (the file dialog, a drop on the library, Finder or the file manager, `xdg-open`)
  **copies it into the folder** as `Author — Title.ext` and opens the copy; a book already in the folder (the same
  file content) is not copied again, it opens. The original file is never moved or changed.
- **Books put into the folder by hand** (copied in Finder, the file manager, a sync service) appear in the library the
  next time it shows; their files are read once for title, author and cover.
- **Removing a book from the library moves its file to the system Trash** and forgets its entry; bookmarks and
  highlights stay, so restoring the file from the Trash brings them back. **BREAKING** for 0.9.0 behaviour, where the
  file stayed.
- **Moving the existing library in:** on the first start of this version, every remembered book whose file still
  exists is copied into the folder and remembered there; the originals stay where they are.
- The library gets **"Show the Books folder"**, which opens it in Finder or the file manager.

## Non-goals

- Choosing or moving the folder (a setting) — a fixed, predictable place is the default that needs no explanation;
  revisit when syncing through a service needs it.
- Watching the folder while the library is open, subfolders as shelves, editing metadata, converting formats,
  sending to a Kindle, syncing — later changes.
- Books in the folder in other formats (PDF, MOBI): ignored as today.

## Capabilities

### New Capabilities
- `books-folder`: where the folder is, how books get into it (copy on open, by hand), naming and duplicates, moving the
  existing library in, showing the folder.

### Modified Capabilities
- `library`: opening books copies them into the folder; the library lists the folder's books; removing a book moves
  its file to the Trash; the "Show the Books folder" action.
- `reading-state`: a remembered book's path is its copy in the folder; forgetting a book goes with its file.

## Impact

- `core/`: the folder's path and naming, import (copy with the content id, duplicate check), the folder scan with a
  cache in the state, the one-time move of the library; tests.
- `gtk/`, `macos/`: opening routes through the import; the library's scan, remove-to-Trash (GIO `g_file_trash`,
  `NSFileManager.trashItem`), "Show the Books folder"; the macOS file access no longer needs Downloads.
- Docs: README, the help card if a key is added, `docs/roadmap.md`.
