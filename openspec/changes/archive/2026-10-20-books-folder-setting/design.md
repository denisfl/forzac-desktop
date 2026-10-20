# Design

## Context

`library::books_folder` returns the remembered folder (setting `books.folder`) or makes the default one. `scan` lists
the remembered books whose files exist (a renamed file found in the folder by id) and the folder's files, with a cache
of the folder's files in `file` lines keyed by name, size and mtime. Per-book data is kept by id. Both shells run folder
work on a background thread with a state handle of its own.

## Decisions

**1. `library::change_folder(new, store, move_books) -> Result<Moved>`.** Errors when `new` is not a writable folder.
With `move_books`, for each book file at the old folder's top level (`folder_files`): skip when the new folder already
has its id; else `rename` into the new folder (a free name by `file_name`'s ` (2)` rule), falling back to copy and
remove across volumes (Dropbox on another disk); remembered entries pointing at the old path are pointed at the new
one; their cache lines move with the new names. Then the setting is written. Without `move_books` only the setting
changes and the cache is cleared (the new folder is read on the next scan).

**2. The folder's copy wins.** `scan` takes a remembered book's path from the folder (by id) before its remembered
path, so a book left in the old folder or elsewhere shows once, from the Books folder. This also covers a copy synced
in from another device.

**3. The question is native.** GTK `gtk::AlertDialog` with "Cancel", "Leave them", "Move"; macOS `NSAlert` with the
same three buttons — a decision with three answers reads best in the system's own dialog.

**4. The head's "⋯".** A circle button like the search button; GTK `PopoverMenu` with `lib.*` actions, macOS `NSMenu`.
It gives Linux a visible way to add a folder.

## Risks / Trade-offs

- Moving hundreds of files to a slow network folder takes time; it runs off the UI thread and the library shows
  "Adding books…"-style feedback only as the reload at the end. Acceptable for a one-time action.
