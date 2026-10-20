## ADDED Requirements

### Requirement: The book window's top edge
On macOS a book window SHALL show no title bar of its own: the page SHALL fill the window up to its top edge, under a
transparent title bar with no title drawn and no line under it. The window SHALL keep its title (the book's title) for
the Window menu, Mission Control and the Dock. The page's layout SHALL not depend on whether the reading bar is shown:
the bar lies over the page's top margin, never over its text. The library window SHALL not change.

#### Scenario: A page and nothing else
- **WHEN** a book opens and the pointer does not move
- **THEN** the window shows the page from its top edge to its bottom edge, the running head in the top margin, and no
  title bar, window buttons or toolbar

#### Scenario: The title stays in the Window menu
- **WHEN** two books are open and the reader opens the Window menu
- **THEN** both books are listed by their titles

### Requirement: The reading bar
On macOS a book window SHALL have a reading bar 52 points high across its top, drawn over the page without a background
of its own: at the left the window buttons and a "Back to library" button (a chevron), at the right a Contents button,
a Text settings button ("Aa"), a Search button and a Bookmark button. The page's running head and its ribbon SHALL stay
in sight between the two groups: the bar adds no titles. Under each group the theme's paper SHALL fade out over 24
points towards the middle, so a long running head goes out under the buttons instead of running into them. The
Bookmark button SHALL be drawn outlined, and filled only on a bookmarked page whose ribbon lies under the right group of
buttons (a narrow window), where the ribbon cannot be seen. The buttons SHALL be the system's toolbar buttons with its
symbols, with tooltips naming their keys ("Contents (o)", "Text settings (s)", "Search (/)", "Bookmark (b)", "Back to
library"). Contents, Text settings, Search and Bookmark SHALL do what `o`, `s`, `/` and `b` do; "Back to library"
SHALL do what the reading menu's back button does (`reader-menu`). Dragging the bar's empty part SHALL move the window
and a double click on it SHALL do what a double click on a title bar does in the system's settings.

#### Scenario: Opening the contents from the bar
- **WHEN** the reader moves the pointer and clicks Contents in the bar
- **THEN** the contents panel opens on the "Contents" tab, as with `o`

#### Scenario: No second title
- **WHEN** the reader is on page 7 of "Глава 1. «Нормандия»" and moves the pointer
- **THEN** the bar appears around the page's running head "… Глава 1. «Нормандия»", which stays as it was, and the
  ribbon of a bookmarked page stays in sight

#### Scenario: A narrow window
- **WHEN** the window is 760 points wide, the page has a bookmark and the reader moves the pointer
- **THEN** the running head fades out before the Contents button, the ribbon is under the right group, and the Bookmark
  button is filled; in a 1100-point window the ribbon shows beside the column and the button stays outlined

### Requirement: When the reading bar shows
The reading bar SHALL appear, fading in over 150 ms, when the pointer moves in the book window while it is the key
window. It SHALL fade out over 300 ms 2 s after the pointer last moved, unless the pointer is over the bar or the
reading menu is open; it SHALL fade out at once when a key is pressed, when the reader clicks the page, and when a
panel, the help, a note or the image and table view opens. While a
panel, the help, a note or the view is open, the bar SHALL not appear. When the bar fades out after the pointer stopped
over the page, the pointer SHALL hide too, until it moves again. Keys SHALL never show the bar, except `Return` opening
the reading menu (`reader-menu`). The window buttons SHALL come and go with the bar.

#### Scenario: Reading with the keyboard
- **WHEN** the reader turns pages with `l`, `Space` and `j` without touching the mouse
- **THEN** no bar and no pointer appear over the page

#### Scenario: The pointer stops
- **WHEN** the reader moves the pointer over the page and leaves it there
- **THEN** the bar fades in, and 2 s after the pointer stopped the bar and the pointer fade away

#### Scenario: Resting on the bar
- **WHEN** the reader moves the pointer onto the bar and leaves it there
- **THEN** the bar stays until the pointer leaves it, then fades 2 s later

#### Scenario: A page turned while the bar shows
- **WHEN** the bar is shown and the reader presses `l`
- **THEN** the bar fades out as the next page slides in

### Requirement: The reading bar in full screen
In full screen the reading bar SHALL come and go with the pointer as in a window, on the page's top margin, without the
window buttons (the system shows them with the menu bar when the pointer reaches the screen's top edge). Entering and
leaving full screen SHALL keep the reading position (`page-turning`).

#### Scenario: Full screen
- **WHEN** the reader presses `f` and then moves the pointer
- **THEN** the page fills the screen and the bar appears over its top margin with the back button and the four
  buttons, and fades 2 s after the pointer stops

### Requirement: Cost of the reading bar
Showing and hiding the reading bar SHALL not redraw or re-typeset the page: the fades SHALL run on Core Animation and
SHALL miss no frames on the M1 MacBook Pro's 60 Hz display, also while a page slides. Once the bar has faded out and
the pointer rests, Forzac SHALL use no CPU (no timers or frame callbacks left running).

#### Scenario: A resting window
- **WHEN** the bar has faded out and the reader reads one page for 10 s without input
- **THEN** Forzac's CPU time does not grow over those 10 s
