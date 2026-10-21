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
- **THEN** a message says "Couldn't open “photo.jpg”: unsupported format — FB2, FB3, EPUB, TXT or Markdown needed", nothing is copied
  and the library stays

#### Scenario: Dropping a folder
- **WHEN** the reader drops a folder with 40 books in subfolders on the library
- **THEN** the head counts "Adding books… 1 of 40" up to 40, no book window opens, and the library then shows the 40
  books first, with no message

### Requirement: The empty library
With no books, the library SHALL show the web reader's empty state: "Read books right on your computer", "Open a book
file — a copy goes into your Books folder, the original stays where it is.", the "Open a book" button, "or drop a file
into this window", "Show the Books folder" and, under a rule, "FB2, FB2.ZIP, FB3, EPUB, TXT and Markdown are supported.", and beside
them an illustrated book page (a
page of "Pride and Prejudice" in English, of "Капитанская дочка" in Russian, over a second page turned slightly). On a
narrow window the page goes under the text.

#### Scenario: The first start
- **WHEN** Forzac starts for the first time without a book
- **THEN** the empty state invites to open a book, and `Ctrl+O` opens the file dialog
