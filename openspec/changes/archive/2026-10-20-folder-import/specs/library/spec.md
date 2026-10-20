## MODIFIED Requirements

### Requirement: Opening books
`Enter` or a click SHALL open the selected book where it was left, in its own window, and close the library. The
"Open a book" button and `Ctrl+O` (`⌘O` on the Mac) SHALL open the system's file dialog; dropping one or more files on
the library window SHALL open them, while a dragged file is over the window a dashed frame SHALL say "Release the file
to open it". A book opened from outside the Books folder SHALL be copied into it first, as `books-folder` says. A file
that cannot be opened SHALL be reported as `reader-window` says and leave the library as it was.

A folder SHALL be added instead of opened (`books-folder`, importing a folder) when it is dropped on the library,
chosen in the folder dialog of `Ctrl+Shift+O` (`⇧⌘O`, File › Add a Folder… on the Mac), or chosen in the Mac's open
panel. Adding SHALL leave the library open: while it runs, the head SHALL say "Adding books… N of M" in the secondary
colour in place of the search button; when it ends the library SHALL show the books again with the new ones. Only
when a file failed or nothing was added SHALL a message say so: "Added N books. K files couldn't be read." or "No new
books in “folder”: all N are already in the library." (with the plural forms of the interface language). A second
folder added while one is being added SHALL wait for it.

#### Scenario: Dropping a book
- **WHEN** the reader drags an EPUB from the file manager onto the library window and releases it
- **THEN** a copy goes into the Books folder, the book opens in its own window, and the next time the library shows,
  the book is first

#### Scenario: Dropping a photo
- **WHEN** the reader drops `photo.jpg` on the library
- **THEN** a message says "Couldn't open “photo.jpg”: unsupported format — FB2, FB3 or EPUB needed", nothing is copied
  and the library stays

#### Scenario: Dropping a folder
- **WHEN** the reader drops a folder with 40 books in subfolders on the library
- **THEN** the head counts "Adding books… 1 of 40" up to 40, no book window opens, and the library then shows the 40
  books first, with no message

### Requirement: Keys
In the library, the arrows and `h` `j` `k` `l` SHALL move the selection across the grid (left, down, up, right; from
the card down to the shelf and back), `Home` and `End` to the first and the last book, `Enter` SHALL open the selected
book, `Delete` and `d` ask to remove it, `/` searches, `F` shows the Books folder, `Ctrl+O` / `⌘O` opens a file,
`Ctrl+Shift+O` / `⇧⌘O` adds a folder, and `q`, `Esc`, `Ctrl+W` / `⌘W` close the library (with no book open, Forzac
quits). The keys SHALL also work in a Russian keyboard layout. The selected book SHALL be marked with a ring around
its cover and scrolled into view; the last opened book is selected when the library opens.

#### Scenario: Down the shelf
- **WHEN** the library shows the card and a shelf of six columns, and the reader presses `j` twice
- **THEN** the selection moves from the card to the first book of the shelf, then to the first book of its second row
