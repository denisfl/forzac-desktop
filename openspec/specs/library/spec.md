# library Specification

## Purpose
The library window, as the web reader's: the books read, with their covers, the last one to continue first; opening,
dropping and removing books, search, the empty state, its keys and the cover cache. Books open in place.

## Requirements

### Requirement: The library window
Running Forzac without a file and closing the last book window SHALL show the library, in both shells: the books of
the reading state whose files still exist, laid out as the web reader's library, in the reader's theme (paper, ink,
secondary colour, lines) and the interface font (Golos Text; the brand, the headings and the generated covers in the
book serifs). Its head SHALL hold the brand (the app icon and "Forzac") on the left and, while there are books, a
search button and an "Open a book" button on the right; under it, a note line in the secondary colour ("We don’t store
or send any data to a server. Everything runs on your computer. FB2, FB2.ZIP, FB3 and EPUB are supported."). Books
SHALL open in place: the library never copies or imports a file. Only one library window SHALL exist at a time.

#### Scenario: Starting without a book
- **WHEN** Forzac is started without arguments after reading three books
- **THEN** the library shows those three books in the reader's theme, the last opened first

#### Scenario: A file went away
- **WHEN** a book's file has been deleted since it was read
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
to open it". A file that cannot be opened SHALL be reported as `reader-window` says and leave the library as it was.

#### Scenario: Dropping a book
- **WHEN** the reader drags an EPUB from the file manager onto the library window and releases it
- **THEN** the book opens in its own window, and the next time the library shows, the book is first

#### Scenario: Dropping a photo
- **WHEN** the reader drops `photo.jpg` on the library
- **THEN** a message says "Couldn't open “photo.jpg”: unsupported format — FB2, FB3 or EPUB needed" and the library
  stays

### Requirement: Removing a book
`Delete` or `d` on the selected book, or the "×" shown next to a book's title under the pointer, SHALL ask "Remove
“title” from the library?", saying that the file stays where it is, with "Cancel" and "Remove". Removing SHALL forget
the book's entry (its place in the library and its reading position) and its cached cover; the file, its bookmarks and
its highlights SHALL stay, and opening the file again brings the book back. `Esc` or "Cancel" SHALL keep the book;
`Enter` or "Remove" removes it.

#### Scenario: Removing by key
- **WHEN** the reader selects a book, presses `d` and then `Enter`
- **THEN** the book leaves the library, its file is untouched, and the selection moves to the next book

### Requirement: Search
`/` or the search button SHALL replace the brand with a search field ("Title or author"); typing SHALL show only the
books whose title or authors have a word starting with each word typed — in any order, regardless of case, "ё" and
"е" and punctuation, also across the Latin and Cyrillic alphabets (Dostoevsky finds Достоевский) — titles before
authors and the own alphabet before transliteration, without the "Continue reading" card. With no match the library
SHALL say "No results". `Esc` or "×" SHALL close the search and show the whole library.

#### Scenario: Finding by author
- **WHEN** the library has "Война и мир" by Лев Толстой and the reader types `/tolst`
- **THEN** the shelf shows that book only

### Requirement: Keys
In the library, the arrows and `h` `j` `k` `l` SHALL move the selection across the grid (left, down, up, right; from
the card down to the shelf and back), `Home` and `End` to the first and the last book, `Enter` SHALL open the selected
book, `Delete` and `d` ask to remove it, `/` searches, `Ctrl+O` / `⌘O` opens a file, and `q`, `Esc`, `Ctrl+W` / `⌘W`
close the library (with no book open, Forzac quits). The keys SHALL also work in a Russian keyboard layout. The
selected book SHALL be marked with a ring around its cover and scrolled into view; the last opened book is selected
when the library opens.

#### Scenario: Down the shelf
- **WHEN** the library shows the card and a shelf of six columns, and the reader presses `j` twice
- **THEN** the selection moves from the card to the first book of the shelf, then to the first book of its second row

### Requirement: The empty library
With no books, the library SHALL show the web reader's empty state: "Read books right on your computer", "Open a book
file — it stays where it is, the library only remembers it.", the "Open a book" button, "or drop a file into this
window", the note line under a rule, and beside them an illustrated book page (a page of "Pride and Prejudice" in
English, of "Капитанская дочка" in Russian, over a second page turned slightly). On a narrow window the page goes under
the text.

#### Scenario: The first start
- **WHEN** Forzac starts for the first time without a book
- **THEN** the empty state invites to open a book, and `Ctrl+O` opens the file dialog

### Requirement: Speed and idle
The library with 50 books, their covers cached, SHALL present its first frame at most 200 ms after the start of
`forzac` on a Raspberry Pi 5, and SHALL draw nothing and use no CPU while nothing changes.

#### Scenario: Measuring on the Pi
- **WHEN** `forzac --measure` runs on the Pi 5 with 50 books in the reading state
- **THEN** it prints a first frame at 200 ms or less
