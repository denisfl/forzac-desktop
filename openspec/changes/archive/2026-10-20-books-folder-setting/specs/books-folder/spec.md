## ADDED Requirements

### Requirement: Changing the folder
"Change the Books folder…" SHALL show the system's folder dialog; choosing a folder other than the current one SHALL
ask "Move the books to “name”?" ("Перенести книги в «name»?") with "The books in “old” go to the new folder; where
you stopped in each stays.", and "Move", "Leave them", "Cancel". "Cancel" SHALL change nothing. Otherwise the chosen
folder SHALL become the Books folder, remembered in the state. "Move" SHALL move every FB2, FB2.ZIP, FB3 and EPUB file
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

## MODIFIED Requirements

### Requirement: The folder
Forzac SHALL keep its books in one folder, by default in the home folder: `~/Книги` when the interface language is
Russian at the time the folder is made, `~/Books` otherwise; the reader can choose another (`Changing the folder`).
The folder SHALL be made on the first start of a version with this capability, and its path SHALL be remembered in the
state, so a later change of the interface language does not move or rename it. When the remembered folder is missing
(deleted by hand, or a sync service's folder not there), Forzac SHALL make it again at the same path. A folder that
already exists SHALL be used as it is: its FB2, FB2.ZIP, FB3 and EPUB files at the top level join the library, and
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

### Requirement: Showing the folder
`F` in the library, "Show the Books folder" ("Показать папку «Книги»") in the library head's "⋯" menu and in the empty
library, and, on macOS, File › Show the Books folder SHALL open the folder in Finder on macOS and in the default file
manager on Linux.

#### Scenario: Where are my books
- **WHEN** the reader presses `F` in the library
- **THEN** the Books folder opens in the file manager
