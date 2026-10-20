## ADDED Requirements

### Requirement: Shelves
The book's menu SHALL hold "Add to shelf" ("На полку") with every shelf name A to Z, checked when the book is on it,
and "New shelf…" ("Новая полка…"). Choosing a shelf SHALL put the book on it, or take it off when checked. "New
shelf…" SHALL ask "New shelf" with a name field, "Cancel" and "Add" (`Enter` adds, `Esc` cancels); a name, trimmed and
not empty, SHALL put the book on that shelf (an existing name, compared without case, is that shelf). A shelf SHALL
exist while at least one book is on it. The Shelves view SHALL show, each as a heading with its books by title, the
automatic shelves "Reading" (opened and not finished), "Not started" (never opened) and "Finished", each only when it
has books, then the reader's shelves A to Z; a book SHALL show under every shelf it is on.

#### Scenario: A shelf for the summer
- **WHEN** the reader chooses "New shelf…" in a book's menu, types "Летом" and presses `Enter`, then chooses "Летом" in
  another book's "Add to shelf"
- **THEN** the Shelves view shows "Летом" with both books after the automatic shelves, and after a restart it still does

#### Scenario: The last book taken off
- **WHEN** the only book on "Работа" is taken off it
- **THEN** "Работа" is no longer offered in "Add to shelf" and not shown in the Shelves view

## MODIFIED Requirements

### Requirement: Views
Under the head, the library SHALL show tabs "Recent", "Title", "Author", "Series", "Shelves" ("Недавние", "Название",
"Автор", "Серия", "Полки") while it has books and no search is open, the chosen one marked as the contents panel marks
its tab. Recent SHALL be the shelf of `Continue reading and the shelf`. The other views SHALL show no "Continue
reading" card:
- Title: one shelf, A to Z by title.
- Author: a heading per first author with that author's books under it; authors A to Z by surname (the last word of
  the name that is not an initial), books by series and number, then title; books without an author last, under
  "Unknown author".
- Series: a heading per series with its books in their number order (unnumbered last, by title); series A to Z; books
  outside a series last, under "Not in a series".
- Shelves: as `Shelves` says.
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
right click on the book, SHALL open a menu with "About the book", "Add to shelf" (`Shelves`) and "Remove from
library". `Delete` and `d` SHALL still ask to remove the selected book.

#### Scenario: Removing from the menu
- **WHEN** the reader clicks "⋯" under a book and chooses "Remove from library"
- **THEN** the library asks "Remove “title” from the library?" as `Removing a book` says
