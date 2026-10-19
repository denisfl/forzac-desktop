## MODIFIED Requirements

### Requirement: The library window
Running Forzac without a file and closing the last book window SHALL show the library, in both shells: the books in the
Books folder (`books-folder`) together with the remembered books whose files still exist, laid out as the web reader's
library, in the reader's theme (paper, ink, secondary colour, lines) and the interface font (Golos Text; the brand, the
headings and the generated covers in the book serifs). Its head SHALL hold the brand (the app icon and "Forzac") on the
left and, while there are books, a search button, a "Show the Books folder" button and an "Open a book" button on the
right; under it, a note line in the secondary colour ("We don’t store or send any data to a server. Everything runs on
your computer. FB2, FB2.ZIP, FB3 and EPUB are supported."). Books SHALL be read from the Books folder: opening a book
from elsewhere copies it there first. Only one library window SHALL exist at a time.

#### Scenario: Starting without a book
- **WHEN** Forzac is started without arguments after reading three books
- **THEN** the library shows those three books in the reader's theme, the last opened first

#### Scenario: A file went away
- **WHEN** a book's file has been deleted from the Books folder since it was read
- **THEN** the library does not show it

### Requirement: Opening books
`Enter` or a click SHALL open the selected book where it was left, in its own window, and close the library. The
"Open a book" button and `Ctrl+O` (`⌘O` on the Mac) SHALL open the system's file dialog; dropping one or more files on
the library window SHALL open them, while a dragged file is over the window a dashed frame SHALL say "Release the file
to open it". A book opened from outside the Books folder SHALL be copied into it first, as `books-folder` says. A file
that cannot be opened SHALL be reported as `reader-window` says and leave the library as it was.

#### Scenario: Dropping a book
- **WHEN** the reader drags an EPUB from the file manager onto the library window and releases it
- **THEN** a copy goes into the Books folder, the book opens in its own window, and the next time the library shows,
  the book is first

#### Scenario: Dropping a photo
- **WHEN** the reader drops `photo.jpg` on the library
- **THEN** a message says "Couldn't open “photo.jpg”: unsupported format — FB2, FB3 or EPUB needed", nothing is copied
  and the library stays

### Requirement: Removing a book
`Delete` or `d` on the selected book, or the "×" shown next to a book's title under the pointer, SHALL ask "Remove
“title” from the library?", saying that the file goes to the Trash, with "Cancel" and "Remove". Removing SHALL move the
book's file in the Books folder to the system Trash, forget the book's entry (its place in the library and its reading
position) and drop its cached cover; its bookmarks and highlights SHALL stay, so putting the file back from the Trash
brings them back. A file that cannot be moved to the Trash SHALL be reported and the book SHALL stay. `Esc` or
"Cancel" SHALL keep the book; `Enter` or "Remove" removes it.

#### Scenario: Removing by key
- **WHEN** the reader selects a book, presses `d` and then `Enter`
- **THEN** the book leaves the library, its file is in the Trash, and the selection moves to the next book

#### Scenario: Putting it back
- **WHEN** a removed book with two highlights is put back from the Trash into the Books folder and the library opens
- **THEN** the book is on the shelf as not started, and opening it shows both highlights

### Requirement: Keys
In the library, the arrows and `h` `j` `k` `l` SHALL move the selection across the grid (left, down, up, right; from
the card down to the shelf and back), `Home` and `End` to the first and the last book, `Enter` SHALL open the selected
book, `Delete` and `d` ask to remove it, `/` searches, `F` shows the Books folder, `Ctrl+O` / `⌘O` opens a file, and
`q`, `Esc`, `Ctrl+W` / `⌘W` close the library (with no book open, Forzac quits). The keys SHALL also work in a Russian
keyboard layout. The selected book SHALL be marked with a ring around its cover and scrolled into view; the last
opened book is selected when the library opens.

#### Scenario: Down the shelf
- **WHEN** the library shows the card and a shelf of six columns, and the reader presses `j` twice
- **THEN** the selection moves from the card to the first book of the shelf, then to the first book of its second row

### Requirement: The empty library
With no books, the library SHALL show the web reader's empty state: "Read books right on your computer", "Open a book
file — a copy goes into your Books folder, the original stays where it is.", the "Open a book" button, "or drop a file
into this window", "Show the Books folder", the note line under a rule, and beside them an illustrated book page (a
page of "Pride and Prejudice" in English, of "Капитанская дочка" in Russian, over a second page turned slightly). On a
narrow window the page goes under the text.

#### Scenario: The first start
- **WHEN** Forzac starts for the first time without a book
- **THEN** the empty state invites to open a book, and `Ctrl+O` opens the file dialog
