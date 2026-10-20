## Why

Syncing with the Android app (`sync-format`, docs/roadmap.md) needs both apps to call the same file the same book.
Android names a book by the first 16 bytes of the file's SHA-256 (`BookImport.idOf`); the desktop by FNV-1a 64 over
the file's length and its first and last 64 KiB (`state::book_id`). Shelves and edited metadata, which come next, are
kept by book id too, so the id has to change now, before more data hangs on it.

## What Changes

- A book's id becomes the Android app's: the first 16 bytes of the SHA-256 of the whole file, as 32 lowercase hex
  digits. The same file has the same id on Android and the desktop.
- On the first start of this version the state is moved to the new ids once: every remembered book, bookmark list,
  highlight list and cached file of the Books folder whose file is at hand is hashed again and renamed. A book whose
  file is gone (in the Trash) keeps its old id; when its file comes back into the Books folder, its entry, bookmarks
  and highlights are renamed then.
- Generated covers are chosen by the id, as on Android, so a book without a cover now gets the same cloth or marbled
  paper as in the Android app (it may change once from what 0.11.0 showed). Cached cover thumbnails are made again
  under the new id in the background.

## Non-goals

- Syncing itself, a sync file format, the Locator's offset units (`sync-format`).
- Removing the old cover thumbnails from the cache: they are small and the cache directory is the system's to clean.

## Capabilities

### Modified Capabilities
- `reading-state`: a book is recognised by the SHA-256 of its content; the one-time move of the state to the new ids.

## Impact

- `core/`: `state::book_id` (SHA-256 through the `sha2` crate), the old FNV id kept for the move, `Store::rename_ids`,
  the move in `library::books_folder` and the late rename in the folder scan; tests.
- `gtk/`, `macos/`: no code change; both call `books_folder` on a background thread already.
- Docs: `docs/roadmap.md`.
