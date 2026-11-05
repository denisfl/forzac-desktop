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
Comparing ignores case, "ё"/"е", leading punctuation (quotes, brackets, an ellipsis) and a leading English "The", "A" or "An"; titles and names starting with a digit come first, then those in the alphabet of the interface language (Cyrillic in a Russian interface, Latin in an English one), then the rest, as Android's shelf orders them (`library-sort`). Equal titles in the Title view go by their first author. `Tab` and `Shift+Tab` SHALL move to the next and previous
view, a click on a tab SHALL choose it; the view SHALL be remembered in the state. Under each cover, when the view is
not Series and the book is in a series, the series and its number SHALL be shown in the secondary colour on one line,
shortened with an ellipsis.

#### Scenario: Reading a series in order
- **WHEN** the library has the Strugatskys' "Полдень, XXII век" as number 2 and "Стажёры" as number 4 of one series and
  the reader presses `Tab` three times
- **THEN** the Series view shows the series' heading with "Полдень, XXII век" before "Стажёры", and after a restart the
  library opens in the Series view

#### Scenario: The interface's alphabet first
- **WHEN** the library holds "Оно", "Анна Каренина", "The Shining" and "„Крутой маршрут“", the interface is Russian and the Title view is chosen
- **THEN** the order is "Анна Каренина", "„Крутой маршрут“", "Оно", "The Shining"; in an English interface "The Shining" comes first
