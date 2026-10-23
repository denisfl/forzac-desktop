## MODIFIED Requirements

### Requirement: The library's menu
While no search is open — also when the library is empty — its head SHALL show a "⋯" button before the search button; a click
SHALL open a menu with "Show the Books folder", "Change the Books folder…" (`books-folder`), "Add a Folder…"
(`Opening books`) and, after them, "Reading stats" (`reading-stats`) and "Settings…" (`settings`). On macOS the File
menu SHALL also hold "Change the Books folder…".

#### Scenario: Adding a folder with the mouse on Linux
- **WHEN** the reader clicks "⋯" in the library's head and chooses "Add a Folder…"
- **THEN** the folder dialog opens, and the chosen folder's books are added as `Ctrl+Shift+O` adds them

#### Scenario: Back from an empty folder
- **WHEN** the reader changed the Books folder to an empty one and chose "Leave them"
- **THEN** the empty library still shows "⋯", and "Change the Books folder…" there leads back

#### Scenario: The settings from an empty library
- **WHEN** the library is empty and the reader chooses "Settings…" in its "⋯" menu
- **THEN** the settings open

#### Scenario: The stats from the library
- **WHEN** the reader clicks "⋯" in the library's head and chooses "Reading stats"
- **THEN** the stats window opens on the current month; `Esc` closes it and the library is as it was

### Requirement: Keys
In the library, the arrows and `h` `j` `k` `l` SHALL move the selection across the grid (left, down, up, right; from
the card down to the shelf and back; from the last row of a group down to the first row of the next one and back up),
`Home` and `End` to the first and the last book, `Tab` and `Shift+Tab` SHALL switch the view, `Enter` SHALL open the
selected book, `i` SHALL show it in `About the book`, `Delete` and `d` ask to remove it, `/` searches, `s` opens the
reading stats, `F` shows the Books folder, `Ctrl+O` / `⌘O` opens a file, `Ctrl+Shift+O` / `⇧⌘O` adds a folder, and
`q`, `Esc`, `Ctrl+W` / `⌘W` close the library (with no book open, Forzac quits). The keys SHALL also work in a Russian
keyboard layout. The selected book SHALL be marked with a ring around its cover and scrolled into view; the last opened
book is selected when the library opens in Recent, the first book in the other views.

#### Scenario: Down the shelf
- **WHEN** the library shows the card and a shelf of six columns, and the reader presses `j` twice
- **THEN** the selection moves from the card to the first book of the shelf, then to the first book of its second row

#### Scenario: Down into the next group
- **WHEN** the Author view shows an author with two books and below another with five, the first author's second book
  is selected, and the reader presses `j`
- **THEN** the selection moves to the next author's book in the same column, or its last book when that row is shorter

#### Scenario: The stats by key
- **WHEN** the reader presses `s` in the library
- **THEN** the stats window opens on the current month
