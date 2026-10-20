## ADDED Requirements

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

## MODIFIED Requirements

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
