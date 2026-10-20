## Why

A reader moving to Forzac usually has books already: a folder of downloads, an old Calibre library, a backup with
subfolders by author. Today they can only be opened one by one or dropped as files. Bulk import is the most asked-for
library feature elsewhere (Foliate #730), and the Calibre-like library (docs/roadmap.md, 16–19) needs real numbers of
books to group and sort.

## What Changes

- **Adding a folder:** dropping a folder on the library, choosing one with `Ctrl+Shift+O` (`⇧⌘O`, also File › Add a
  Folder… on macOS) or choosing a folder in the macOS "Open a book" panel copies every FB2, FB2.ZIP, FB3 and EPUB in
  it and its subfolders into the Books folder, named and de-duplicated as one opened book is. Nothing opens; the
  library stays and shows the new books.
- While it runs, the library's head says "Adding books… 120 of 300"; the window stays responsive.
- At the end the library shows the new books. Only when something was not added does a message say so: "Added 12
  books. 3 files couldn't be read." or "No new books in “Downloads”: all 40 are already in the library."
- The source folder and its files are only read.

## Non-goals

- Watching a folder, keeping the subfolder structure (shelves come in `library-shelves`), reading a Calibre library's
  `metadata.db`, importing from the command line or Finder's "Open With" with a folder.
- Other formats in the folder (PDF, MOBI): skipped silently, as in the Books folder.

## Capabilities

### Modified Capabilities
- `books-folder`: a new requirement — importing a folder.
- `library`: opening books accepts a folder (drop, the macOS panel); the `Ctrl+Shift+O` / `⇧⌘O` key; the progress line.

## Impact

- `core/`: `library::import_folder` (walk, one scan for duplicates, copies, one state write, progress callback, the
  counts) sharing the copy with `library::import`; the strings in `i18n`; tests.
- `gtk/`: the drop and `Ctrl+Shift+O` (`gtk::FileDialog::select_folder`), the progress line, the message.
- `macos/`: the drop, the open panel accepting folders, File › Add a Folder… (`⇧⌘O`), the progress line, the message.
- Docs: `docs/roadmap.md`.
