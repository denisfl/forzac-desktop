## MODIFIED Requirements

### Requirement: The library window
Running Forzac without a file and closing the last book window SHALL show the library window, in both shells. It SHALL
hold two sections (`Sections`): "Books" — the books in the Books folder (`books-folder`) together with the remembered
books whose files still exist, laid out as the web reader's library — and "Reading stats" (`reading-stats`). The
content of both SHALL be in the reader's theme (paper, ink, secondary colour, lines) and the interface font (Golos
Text; the headings and the generated covers in the book serifs). The window's head SHALL be the platform's own:
`The head on macOS` and `The head on Linux`; the brand ("Forzac" with the app icon) SHALL NOT be shown in it. Books
SHALL be read from the Books folder: opening a book from elsewhere copies it there first. Only one library window SHALL
exist at a time; its first frame SHALL already show the theme's paper, never a white or system-coloured window.

#### Scenario: Starting without a book
- **WHEN** Forzac is started without arguments after reading three books
- **THEN** the library window opens on "Books" in the reader's theme, the last opened book first, with no white frame
  before it

#### Scenario: A file went away
- **WHEN** a book's file has been deleted from the Books folder since it was read
- **THEN** the library does not show it

### Requirement: Keys
In "Books", the arrows and `h` `j` `k` `l` SHALL move the selection across the grid (left, down, up, right; from the card
down to the shelf and back; from the last row of a group down to the first row of the next one and back up), `Home` and
`End` to the first and the last book, `Tab` and `Shift+Tab` SHALL switch the view, `Enter` SHALL open the selected book,
`i` SHALL show it in `About the book`, `Delete` and `d` ask to remove it, `/` searches (also `⌘F` on macOS and `Ctrl+F` on
Linux), `s` SHALL switch to "Reading stats", `F` shows the Books folder, `Ctrl+O` / `⌘O` opens a file, `Ctrl+Shift+O` /
`⇧⌘O` adds a folder. In both sections `⌘1` and `⌘2` on macOS, `Ctrl+1` and `Ctrl+2` on Linux SHALL choose "Books" and
"Reading stats"; on Linux `Ctrl+,` SHALL open the settings, `Ctrl+?` and `?` the keys, `F10` the main menu. `q`,
`Ctrl+W` / `⌘W` close the library window (with no book open, Forzac quits); `Esc` SHALL close an open search or menu,
return from "Reading stats" to "Books", and in "Books" with nothing open close the window. The keys SHALL also work in a
Russian keyboard layout. The selected book SHALL be marked with a ring around its cover and scrolled into view; the last
opened book is selected when the library opens in Recent, the first book in the other views.

#### Scenario: Down the shelf
- **WHEN** the library shows the card and a shelf of six columns, and the reader presses `j` twice
- **THEN** the selection moves from the card to the first book of the shelf, then to the first book of its second row

#### Scenario: Down into the next group
- **WHEN** the Author view shows an author with two books and below another with five, the first author's second book
  is selected, and the reader presses `j`
- **THEN** the selection moves to the next author's book in the same column, or its last book when that row is shorter

#### Scenario: To the stats and back
- **WHEN** the reader presses `s` in "Books", then `Esc`
- **THEN** "Reading stats" shows the current month, then "Books" shows again with the same book selected and the same
  scroll position

#### Scenario: Sections by number on Linux
- **WHEN** the reader presses `Ctrl+2` and then `Ctrl+1` in a Russian layout
- **THEN** "Reading stats" and then "Books" show

### Requirement: Search
`/`, `Ctrl+F` / `⌘F` or the search control SHALL start the search in "Books": on macOS the toolbar's search field takes
the focus; on Linux a search bar slides down under the header with a field ("Title, author or series"), and the
header's search button shows it is on. Typing SHALL show only the books whose title, authors or series have a word
starting with each word typed — in any order, regardless of case, "ё" and "е" and punctuation, also across the Latin
and Cyrillic alphabets (Dostoevsky finds Достоевский) — titles before authors, authors before series and the own
alphabet before transliteration, as one shelf without the "Continue reading" card and the tabs, updated as the reader
types. With no match the library SHALL say "No results". `Esc`, the field's clear button on macOS, or the search button
again on Linux SHALL close the search and show the chosen view again. Letters typed outside the field SHALL stay keys
(`Keys`), never start a search by themselves.

#### Scenario: Finding by author
- **WHEN** the library has "Война и мир" by Лев Толстой and the reader types `/tolst`
- **THEN** the shelf shows that book only

#### Scenario: Finding by series
- **WHEN** the library has three books of the series "Discworld" and the reader types `/disc`
- **THEN** the shelf shows those three books

#### Scenario: Ctrl+F on Linux
- **WHEN** the reader presses `Ctrl+F` in "Books" on Linux and types `pushk`
- **THEN** the search bar slides down under the header with "pushk" in it and the shelf shows Pushkin's books

## REMOVED Requirements

### Requirement: The library's menu
**Reason**: The "⋯" menu mixed a section ("Reading stats"), a setting ("Change the Books folder…"), an action ("Add a
Folder…") and a file-manager shortcut; each now has the place its platform expects (`Sections`, `The head on macOS`,
`The head on Linux`, `settings`).
**Migration**: "Reading stats" — the section switcher, `s`, `⌘2` / `Ctrl+2`. "Add a Folder…" — "+" and File › Add a
Folder… (`⇧⌘O` / `Ctrl+Shift+O`). "Show the Books folder" and "Change the Books folder…" — Settings (`settings`), `F`,
and the File menu on macOS. "Settings…" — `⌘,` and Forzac › Settings… on macOS, the main menu and `Ctrl+,` on Linux.

## ADDED Requirements

### Requirement: Sections
The library window SHALL show one of two sections, "Books" ("Книги") and "Reading stats" ("Статистика"), chosen with a
switcher of two text segments in that order — never icons, never a third segment. The switcher SHALL mark the shown
section. Switching SHALL keep each section's state: the view, the selection and the scroll of "Books", the period of
"Reading stats". The window SHALL open on "Books", except when opened from the end page's month line
(`reader-navigation`), which opens it on "Reading stats". The section's own action SHALL sit with it: search and "+"
in "Books", Share on macOS and Save Image on Linux in "Reading stats". While the library is empty "Reading stats"
SHALL still be reachable. Switching SHALL show the other section within 100 ms on a Raspberry Pi 5 with a year of
reading log, and the window SHALL draw nothing and use no CPU while nothing changes.

#### Scenario: The share action follows the section
- **WHEN** the reader switches from "Books" to "Reading stats" with the switcher
- **THEN** the search and "+" give way to Share (macOS) or Save Image (Linux), and the switcher marks "Reading stats"

#### Scenario: The stats of an empty library
- **WHEN** the library is empty and the reader chooses "Reading stats"
- **THEN** the section shows the current month with zeros and "What you finish will appear here"

### Requirement: The head on macOS
On macOS the library window SHALL have a unified toolbar over the content, its background the theme's paper under the
transparent title bar: the window buttons at the leading edge; in the centre the section switcher as a segmented
control; at the trailing edge, in "Books", a "+" button with a menu "Open a Book…" ("Открыть книгу…") and "Add a
Folder…" ("Добавить папку…") and a search field, and in "Reading stats" a Share button. The toolbar SHALL NOT hold a
settings button, and the window SHALL NOT have any bar or control at its bottom edge at any width. Every command of the
toolbar SHALL also be in the menu bar (`macos-app`). The window's title SHALL be the section's name ("Books", "Reading
stats"), shown by the Window menu and Mission Control, and not drawn in the toolbar. The toolbar's controls SHALL be the
system's own and follow the system's appearance matched to the theme (dark paper — dark appearance). When the window is
too narrow for the search field, it SHALL shrink to a search button that opens it; the switcher and the trailing items
SHALL stay visible at every width the window allows. The window SHALL NOT be smaller than 520 × 420 points.

#### Scenario: A narrow Mac window
- **WHEN** the reader narrows the library window to its 520-point minimum
- **THEN** the switcher stays in the centre, the search field becomes a search button, and nothing appears at the
  bottom of the window

#### Scenario: Adding a folder from the toolbar
- **WHEN** the reader clicks "+" in "Books" and chooses "Add a Folder…"
- **THEN** the folder dialog opens, as File › Add a Folder… opens it

### Requirement: The head on Linux
On Linux the library window's header SHALL hold, at its start, the section's actions — in "Books" "+" (a menu "Open a
Book…" `Ctrl+O`, "Add a Folder…" `Ctrl+Shift+O`) and a search toggle button, in "Reading stats" "Save Image…"
(`s`) — in the centre the section switcher, and at its end the main menu button ("Main Menu", the icon of three lines)
with "Settings" ("Настройки", `Ctrl+,`), "Keyboard Shortcuts" ("Клавиши", `Ctrl+?`) and "About Forzac" ("О Форзаце"), in
that order and nothing else (no Quit, no Close). On floating compositors the header SHALL be the window's title bar
with the window buttons; on Hyprland, which draws no title bars, it SHALL be the top row of the window without them.
The header, its buttons, menus and the search bar SHALL be drawn in the theme's colours. "About Forzac" SHALL show the
version, the licence (MPL-2.0), the source link and the support line (`help`).

When the window is narrower than 550 logical pixels, the switcher SHALL leave the header for a floating pill at the
bottom of the window, centred above the content in the theme's colours (as in the Android app), and the header's centre
SHALL show the section's name instead; the content SHALL scroll so that its last row can rise above the pill. Crossing
550 pixels SHALL move the switcher at once, keep the section, the selection and the scroll, and SHALL NOT miss frames
of a live resize on the Pi 5. The library SHALL lay out without clipping down to a 360 × 294 window.

#### Scenario: Half a laptop screen in Omarchy
- **WHEN** Hyprland tiles the library into a 470-pixel-wide window
- **THEN** the header shows "+", the search button, "Books" and the main menu button, and the pill "Books | Reading
  stats" floats at the bottom

#### Scenario: The main menu
- **WHEN** the reader presses `F10` and chooses "Keyboard Shortcuts"
- **THEN** the keys card (`help`) opens over the library

#### Scenario: A wide window
- **WHEN** the window is widened past 550 pixels
- **THEN** the pill disappears and the switcher is in the header's centre, "Books" still selected
