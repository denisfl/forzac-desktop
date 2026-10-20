## ADDED Requirements

### Requirement: Importing a folder
Adding a folder (`library`) SHALL copy every FB2, FB2.ZIP, FB3 and EPUB file in it and in its subfolders, at any depth,
into the Books folder as opening a book does: named by `File names in the folder`, a book whose content (id) is
already in the folder or earlier in the same import not copied again, each copy written in full before it shows.
Dotfiles and dot-folders, symbolic links to folders, the Books folder itself (when it is inside the added folder) and
files of other formats SHALL be skipped; a file that cannot be read or parsed SHALL be counted as failed and SHALL NOT
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
