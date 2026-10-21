# books-folder Specification

## Purpose
TBD - created by archiving change books-folder. Update Purpose after archive.

## Requirements

### Requirement: The folder
Forzac SHALL keep its books in one folder, by default in the home folder: `~/Книги` when the interface language is
Russian at the time the folder is made, `~/Books` otherwise; the reader can choose another (`Changing the folder`).
The folder SHALL be made on the first start of a version with this capability, and its path SHALL be remembered in the
state, so a later change of the interface language does not move or rename it. When the remembered folder is missing
(deleted by hand, or a sync service's folder not there), Forzac SHALL make it again at the same path. A folder that
already exists SHALL be used as it is: its FB2, FB2.ZIP, FB3, EPUB, TXT and Markdown files at the top level join the library, and
Forzac SHALL NOT rename, move or change any file already there, nor touch its subfolders or files of other formats; it
only adds its own copies beside them. The only changes Forzac makes to a file it did not put there are moving it to
the Trash when the reader removes that book from the library and confirms, and moving it to a new Books folder when
the reader changes the folder and chooses "Move".

#### Scenario: The first start in Russian
- **WHEN** Forzac with a Russian interface starts for the first time on a Mac with no `~/Книги`
- **THEN** `~/Книги` exists afterwards, and the next starts use it even after the interface switches to English

#### Scenario: A Books folder that already exists
- **WHEN** Forzac with an English interface starts for the first time and `~/Books` already holds `dune.epub`,
  `notes.pdf` and a subfolder `Comics/`
- **THEN** "Dune" is in the library, `notes.pdf` and `Comics/` stay as they were and are not shown, and no file in
  `~/Books` is renamed or moved

### Requirement: Opening a book copies it into the folder
Opening a book that is not in the folder — from the file dialog, a drop on the library, Finder or the file manager,
`xdg-open` or the command line — SHALL copy the file into the folder and open the copy; the original file SHALL NOT be
moved, changed or kept open. A book whose content is already in the folder (the same book id) SHALL NOT be copied
again: the copy already there opens, where it was left. A book already in the folder SHALL open in place. Copying a
50 MB file SHALL NOT block the window: the book opens when the copy is complete, and a copy that fails (no space, no
permission) SHALL be reported as `reader-window` reports a file that cannot be opened, leaving the original untouched.

#### Scenario: A book from Downloads
- **WHEN** the reader opens `~/Downloads/oz.fb2.zip` from Finder
- **THEN** a copy appears in the Books folder, the book opens from the copy, and deleting `oz.fb2.zip` from Downloads
  does not remove the book from the library

#### Scenario: The same book twice
- **WHEN** the reader opens a file whose content is already in the folder under another name
- **THEN** no second copy is made and the book opens where it was left

### Requirement: File names in the folder
A copy SHALL be named `Author — Title.ext` from the book's metadata (the first author; the title alone when there is no
author; the original file name when there is neither), keeping the original extension (`.fb2`, `.fb2.zip`, `.fb3`,
`.epub`, `.txt`, `.md`, `.markdown`); a file without one gets the extension of its format by its content. Characters not allowed in file names on Linux or macOS (`/`, `:`, control characters) SHALL be replaced, the
name SHALL be at most 120 characters before the extension, and when a different book already has that name, ` (2)`,
` (3)`, … SHALL be added.

#### Scenario: Two books with one title
- **WHEN** two different editions of "Война и мир" by Лев Толстой are opened
- **THEN** the folder holds `Лев Толстой — Война и мир.fb2` and `Лев Толстой — Война и мир (2).fb2`

### Requirement: Books added by hand
Every FB2, FB2.ZIP, FB3, EPUB, TXT and Markdown file in the folder (not in its subfolders) SHALL be in the library, including files
put there by hand or by a sync service, the next time the library shows. Such a book SHALL appear as not started until
it is opened; a book without a title in it is titled by its file's name without the extension. Files of other formats and files that cannot be read SHALL be ignored. The folder SHALL be read only when
the library shows, never while a book is being read; reading a folder of 500 books whose files have not changed SHALL
take at most 50 ms on a Raspberry Pi 5, and a book's file SHALL be parsed for its title, authors and cover only once
until it changes.

#### Scenario: A book copied in by hand
- **WHEN** the reader copies `dune.epub` into the Books folder in Finder and opens the library
- **THEN** "Dune" is on the shelf, not started, with its cover

#### Scenario: An answer saved by hand
- **WHEN** the reader saves `ответ.md` without a first-level heading into the Books folder and opens the library
- **THEN** "ответ" is on the shelf

### Requirement: Moving the existing library in
On the first start of a version with this capability, every remembered book whose file exists outside the folder
SHALL be copied into the folder as an opened book would be, keeping its reading position, bookmarks and highlights;
the original files SHALL stay where they are. Books whose files are gone SHALL be left as they are (not shown, as
before). This SHALL happen once, and a failure for one book SHALL NOT stop the others.

#### Scenario: Updating from 0.9.0
- **WHEN** a reader with three books read from Downloads in 0.9.0 starts this version
- **THEN** the three books are in the Books folder, each opens where it was left, and the files in Downloads are
  still there

### Requirement: Showing the folder
`F` in the library, "Show the Books folder" ("Показать папку «Книги»") in the library head's "⋯" menu and in the empty
library, and, on macOS, File › Show the Books folder SHALL open the folder in Finder on macOS and in the default file
manager on Linux.

#### Scenario: Where are my books
- **WHEN** the reader presses `F` in the library
- **THEN** the Books folder opens in the file manager

### Requirement: Importing a folder
Adding a folder (`library`) SHALL copy every FB2, FB2.ZIP, FB3 and EPUB file in it and in its subfolders, at any depth,
into the Books folder as opening a book does: named by `File names in the folder`, a book whose content (id) is
already in the folder or earlier in the same import not copied again, each copy written in full before it shows.
Dotfiles and dot-folders, symbolic links to folders, the Books folder itself (when it is inside the added folder) and
files of other formats — TXT and Markdown included, as a folder holds READMEs, logs and licences — SHALL be skipped; a file that cannot be read or parsed SHALL be counted as failed and SHALL NOT
stop the rest. The added folder and its files SHALL only be read. The import SHALL run off the UI thread, report its
progress as (files done, files found), write the state once at its end, and return how many books were added, how
many were already there and how many failed.

#### Scenario: An old library with subfolders
- **WHEN** the reader adds `~/Old books` with 300 books in subfolders by author, 20 of them already in the library and
  5 damaged
- **THEN** 275 copies appear in the Books folder, the 20 and the 5 are not copied, and `~/Old books` is unchanged

#### Scenario: Adding the same folder twice
- **WHEN** the reader adds a folder that was added before
- **THEN** nothing is copied and the counts say every book is already in the library

#### Scenario: Adding the home folder
- **WHEN** the reader adds the home folder, which holds the Books folder
- **THEN** the books outside the Books folder are added and the Books folder's own files are not copied again

#### Scenario: A folder of projects
- **WHEN** the reader adds a folder that holds two FB2 books and forty `README.md` files
- **THEN** the two books are added and no README is

### Requirement: Changing the folder
"Change the Books folder…" SHALL show the system's folder dialog; choosing a folder other than the current one SHALL
ask "Move the books to “name”?" ("Перенести книги в «name»?") with "The books in “old” go to the new folder; where
you stopped in each stays.", and "Move", "Leave them", "Cancel". "Cancel" SHALL change nothing. Otherwise the chosen
folder SHALL become the Books folder, remembered in the state. "Move" SHALL move every FB2, FB2.ZIP, FB3, EPUB, TXT and Markdown file
at the old folder's top level into the new one off the UI thread — a book whose content (id) is already in the new
folder SHALL stay where it is, a name taken by another file SHALL get ` (2)`, ` (3)`, … — and point the remembered
books at their new paths; positions, bookmarks, highlights, shelves and edits SHALL stay, as they are kept by book id.
A file that cannot be moved SHALL stay and be counted; when any could not, a message SHALL say "Couldn't move N
books.". "Leave them" SHALL move nothing. The library SHALL then show the new folder's books.

#### Scenario: Into Dropbox
- **WHEN** the reader with 40 books in `~/Книги` chooses `~/Dropbox/Книги` and "Move"
- **THEN** the 40 files are in `~/Dropbox/Книги`, `~/Книги` holds none of them, each book opens where it was left, and
  after a restart the library still reads `~/Dropbox/Книги`

#### Scenario: A folder that already has the books
- **WHEN** the reader chooses a Syncthing folder that already holds 30 of the 40 books (synced from another computer)
  and "Move"
- **THEN** the 10 others are moved, the 30 stay in `~/Книги`, and the library shows each of the 40 once, from the new
  folder
