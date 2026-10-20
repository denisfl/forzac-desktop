# library Specification

## Purpose
The library window, as the web reader's: the books read, with their covers, the last one to continue first; opening,
dropping and removing books, search, the empty state, its keys and the cover cache. Books open in place.

## Requirements

### Requirement: The library window
Running Forzac without a file and closing the last book window SHALL show the library, in both shells: the books in the
Books folder (`books-folder`) together with the remembered books whose files still exist, laid out as the web reader's
library, in the reader's theme (paper, ink, secondary colour, lines) and the interface font (Golos Text; the brand, the
headings and the generated covers in the book serifs). Its head SHALL hold the brand (the app icon and "Forzac") on the
left and, while there are books, a search button and an "Open a book" button on the right; there is no note line under it (removed 2026-10-19: the app now asks GitHub for updates, so "we don’t send any
data" is no longer the whole truth). Books SHALL be read from the Books folder: opening a book
from elsewhere copies it there first. Only one library window SHALL exist at a time.

#### Scenario: Starting without a book
- **WHEN** Forzac is started without arguments after reading three books
- **THEN** the library shows those three books in the reader's theme, the last opened first

#### Scenario: A file went away
- **WHEN** a book's file has been deleted from the Books folder since it was read
- **THEN** the library does not show it

### Requirement: Continue reading and the shelf
The last opened book that is not finished (read less than 99.9 %) SHALL be shown above the shelf as a "Continue
reading" card: its cover, the caption "CONTINUE READING", the title, the authors, a thin progress bar and the share
read. A rule SHALL separate it from the shelf when the shelf is not empty. The shelf SHALL hold the other books, the
last opened first, as a grid of covers in the proportion 2 : 3 with columns from 130 to 176 logical pixels wide as the
window allows; under each cover its title, its authors in the secondary colour and the share read ("N%", at least 1 %;
"Finished" for a finished book).

#### Scenario: A finished book
- **WHEN** the last opened book was read to its end page and an older one is half read
- **THEN** the older one is on the "Continue reading" card, and the finished one is first on the shelf with "Finished"
  under it

#### Scenario: A narrow window
- **WHEN** the window is made narrower
- **THEN** the shelf has fewer columns of covers, each between 130 and 176 pixels wide

### Requirement: Covers
A book with a cover image SHALL show it, filling the cover's box (cropped, not stretched). A book without one SHALL
show a generated cover as the Android app draws it (`Covers.kt`): cloth of one of six colours or, for about 40 % of
books, marbled paper, chosen by the book's id; a darker spine 9 % of the width; a paper label with an ochre frame
holding the title in a bold serif, sized by the title's length and the longest word, and the authors in capitals. A
cover SHALL have a near-square corner at the spine and a rounded one at the edge, and a soft shadow. Cover images SHALL
be cached as thumbnails of at most 352 × 528 pixels in the user's cache directory (`$XDG_CACHE_HOME/forzac/covers` on
Linux, `~/Library/Caches/Forzac/covers` on macOS), made when a book is opened; a book in the library with no thumbnail
yet SHALL get one in the background while the library is shown, without delaying it. The reading state file SHALL NOT
hold covers.

#### Scenario: A book read before covers were cached
- **WHEN** the library shows a book whose cover was never cached
- **THEN** the library appears at once with a placeholder for it, and its cover replaces the placeholder when it is
  ready

#### Scenario: Two books without covers
- **WHEN** two books without covers are in the library
- **THEN** each has its own generated cover with its title, and each looks the same every time the library opens

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

### Requirement: Removing a book
`Delete` or `d` on the selected book, or "Remove from library" in the book's menu, SHALL ask "Remove “title” from the
library?", saying that the file goes to the Trash, with "Cancel" and "Remove". Removing SHALL move the book's file in
the Books folder to the system Trash, forget the book's entry (its place in the library and its reading position) and
drop its cached cover; its bookmarks and highlights SHALL stay, so putting the file back from the Trash brings them
back. A file that cannot be moved to the Trash SHALL be reported and the book SHALL stay. `Esc` or "Cancel" SHALL keep
the book; `Enter` or "Remove" removes it.

#### Scenario: Removing by key
- **WHEN** the reader selects a book, presses `d` and then `Enter`
- **THEN** the book leaves the library, its file is in the Trash, and the selection moves to the next book

#### Scenario: Putting it back
- **WHEN** a removed book with two highlights is put back from the Trash into the Books folder and the library opens
- **THEN** the book is on the shelf as not started, and opening it shows both highlights

### Requirement: Search
`/` or the search button SHALL replace the brand with a search field ("Title, author or series"); typing SHALL show
only the books whose title, authors or series have a word starting with each word typed — in any order, regardless of
case, "ё" and "е" and punctuation, also across the Latin and Cyrillic alphabets (Dostoevsky finds Достоевский) —
titles before authors, authors before series and the own alphabet before transliteration, as one shelf without the
"Continue reading" card and the tabs. With no match the library SHALL say "No results". `Esc` or "×" SHALL close the
search and show the chosen view again.

#### Scenario: Finding by author
- **WHEN** the library has "Война и мир" by Лев Толстой and the reader types `/tolst`
- **THEN** the shelf shows that book only

#### Scenario: Finding by series
- **WHEN** the library has three books of the series "Discworld" and the reader types `/disc`
- **THEN** the shelf shows those three books

### Requirement: Keys
In the library, the arrows and `h` `j` `k` `l` SHALL move the selection across the grid (left, down, up, right; from
the card down to the shelf and back; from the last row of a group down to the first row of the next one and back up),
`Home` and `End` to the first and the last book, `Tab` and `Shift+Tab` SHALL switch the view, `Enter` SHALL open the
selected book, `i` SHALL show it in `About the book`, `Delete` and `d` ask to remove it, `/` searches, `F` shows the
Books folder, `Ctrl+O` / `⌘O` opens a file, `Ctrl+Shift+O` / `⇧⌘O` adds a folder, and `q`, `Esc`, `Ctrl+W` / `⌘W` close
the library (with no book open, Forzac quits). The keys SHALL also work in a Russian keyboard layout. The selected book
SHALL be marked with a ring around its cover and scrolled into view; the last opened book is selected when the library
opens in Recent, the first book in the other views.

#### Scenario: Down the shelf
- **WHEN** the library shows the card and a shelf of six columns, and the reader presses `j` twice
- **THEN** the selection moves from the card to the first book of the shelf, then to the first book of its second row

#### Scenario: Down into the next group
- **WHEN** the Author view shows an author with two books and below another with five, the first author's second book
  is selected, and the reader presses `j`
- **THEN** the selection moves to the next author's book in the same column, or its last book when that row is shorter

### Requirement: The empty library
With no books, the library SHALL show the web reader's empty state: "Read books right on your computer", "Open a book
file — a copy goes into your Books folder, the original stays where it is.", the "Open a book" button, "or drop a file
into this window", "Show the Books folder" and, under a rule, "FB2, FB2.ZIP, FB3 and EPUB are supported.", and beside
them an illustrated book page (a
page of "Pride and Prejudice" in English, of "Капитанская дочка" in Russian, over a second page turned slightly). On a
narrow window the page goes under the text.

#### Scenario: The first start
- **WHEN** Forzac starts for the first time without a book
- **THEN** the empty state invites to open a book, and `Ctrl+O` opens the file dialog

### Requirement: Speed and idle
The library with 50 books, their covers cached, SHALL present its first frame at most 200 ms after the start of
`forzac` on a Raspberry Pi 5, and SHALL draw nothing and use no CPU while nothing changes.

#### Scenario: Measuring on the Pi
- **WHEN** `forzac --measure` runs on the Pi 5 with 50 books in the reading state
- **THEN** it prints a first frame at 200 ms or less

### Requirement: Views
Under the head, the library SHALL show tabs "Recent", "Title", "Author", "Series" ("Недавние", "Название", "Автор",
"Серия") while it has books and no search is open, the chosen one marked as the contents panel marks its tab. Recent
SHALL be the shelf of `Continue reading and the shelf`. The other views SHALL show no "Continue reading" card:
- Title: one shelf, A to Z by title.
- Author: a heading per first author with that author's books under it; authors A to Z by surname (the last word of
  the name that is not an initial), books by series and number, then title; books without an author last, under
  "Unknown author".
- Series: a heading per series with its books in their number order (unnumbered last, by title); series A to Z; books
  outside a series last, under "Not in a series".
Comparing ignores case, "ё"/"е" and leading punctuation. `Tab` and `Shift+Tab` SHALL move to the next and previous
view, a click on a tab SHALL choose it; the view SHALL be remembered in the state. Under each cover, when the view is
not Series and the book is in a series, the series and its number SHALL be shown in the secondary colour on one line,
shortened with an ellipsis.

#### Scenario: Reading a series in order
- **WHEN** the library has the Strugatskys' "Полдень, XXII век" as number 2 and "Стажёры" as number 4 of one series and
  the reader presses `Tab` three times
- **THEN** the Series view shows the series' heading with "Полдень, XXII век" before "Стажёры", and after a restart the
  library opens in the Series view

### Requirement: The book's menu
A "⋯" button SHALL appear next to a book's title under the pointer and on the selected book; a click on it, or a
right click on the book, SHALL open a menu with "About the book" and "Remove from library". `Delete` and `d` SHALL still
ask to remove the selected book.

#### Scenario: Removing from the menu
- **WHEN** the reader clicks "⋯" under a book and chooses "Remove from library"
- **THEN** the library asks "Remove “title” from the library?" as `Removing a book` says

### Requirement: About the book
"About the book" or `i` on the selected book SHALL show a sheet over the dimmed library with the cover, the title, the
authors, the series and number, the genres, the year, the language's name and the annotation (scrolling when long),
with "Read" and "Close"; empty values SHALL be left out. `Enter` or "Read" SHALL open the book, `Esc`, "Close" or a
click beside the sheet SHALL close it. The book's file SHALL be read off the UI thread; the sheet appears when it is
read.

#### Scenario: A book without an annotation
- **WHEN** the reader presses `i` on a book with a title, one author and nothing else
- **THEN** the sheet shows the cover, the title and the author, and "Read"
