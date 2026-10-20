# Design

## Context

`library::import(src, folder, store)` copies one book into the Books folder: it reads the file, computes the id,
scans the folder (`scan`) to find a copy with that id, else parses, names (`file_name`), writes `.<id>.part` and
renames it, and adds a `file` line to the state. Both shells call it off the UI thread (GTK `gio::spawn_blocking`,
macOS the app's `importQueue` with its own state handle `worker`) and then open the copy. The library window drops
files into the shell's open function.

## Decisions

**1. One walk, one scan, one write.** `import_folder` lists the files first (`book_files`: a stack-based walk,
`DirEntry::file_type` so symbolic links to folders are not followed, dot entries and the Books folder skipped, sorted),
reads the Books folder's ids once (`folder_files`, the scan's cached part), and keeps the ids of this import in the same
set, so a book met twice is copied once. The copy itself is `import`'s, moved into `copy_in`. The state gets the new
`file` lines in one write at the end; a crash midway leaves complete copies that the next scan reads as books added by
hand. *Alternative:* calling `import` per file — a scan and a state write per book, quadratic for 300 books.

**2. Progress through a callback, counts as a value.** `progress(done, found)` before each file and at the end; the
shells turn it into the head's "Adding books… N of M". GTK stores the numbers in two atomics read by a 100 ms timer on
the main loop while the import runs (no drawing when idle: the timer is removed at the end); macOS passes a C callback
that dispatches to the main queue. `Added { added, known, failed }` and `Added::message(name)` keep the wording in the
core for both shells.

**3. Say nothing when it went well.** The new books appear in the library (copies are the newest files, so first among
the never opened). A message is shown only when something failed, when nothing was new, or when the folder held no
books — the cases where the reader would otherwise wonder.

**4. Where folders come in.** Dropping a folder on the library (both shells) and `Ctrl+Shift+O` (GTK
`FileDialog::select_folder`; on macOS File › Add a Folder… `⇧⌘O`, and folders allowed in the `⌘O` panel). A second
folder waits: GTK keeps a queue in the library; macOS runs imports on the serial `importQueue`, so they line up there.
Folders from the command line or Finder's "Open With" are not handled (non-goal).

## Risks / Trade-offs

- A huge folder (the home folder) is walked before the first number shows; the head shows an empty line meanwhile.
- Files are read whole to hash them, so every book is read once even when it is already in the library.
