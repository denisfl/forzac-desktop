## ADDED Requirements

### Requirement: Opening the view
An image at least a third of the text column wide, or a table, SHALL open in a view filling the window when the reader
presses `Enter` while it is focused (`Tab`) or clicks it. The view SHALL use the reader's theme: the page's paper as
its background, the text in the ink, links and note markers in the link ink, the table's rules as on the page. It SHALL
fade in within 200 ms and out within 150 ms, as on Android. While it is open, keys act on the view only; a round close
button in its top right corner (the sheet colour with an "×" in the ink) closes it. Closing it SHALL return to the same
page (in "Scroll" the same column offset) with the same `Tab` focus, without moving the reading position. It SHALL work
the same in "Slide" and "Scroll".

#### Scenario: A schematic from the keyboard
- **WHEN** a page holds a schematic as wide as the column and the reader presses `Tab` until it is focused and `Enter`
- **THEN** the schematic fills the window on the page's paper, and `Esc` returns to the unchanged page

#### Scenario: A vignette is not a stop
- **WHEN** a page holds an ornament a fifth of the column wide
- **THEN** `Tab` does not stop at it and a click on it does what a click on the text there does

### Requirement: Image view
An image SHALL open fitted to the window (scaled up or down, its proportions kept) and centred. `+` (also `=` and the
keypad `+`) SHALL zoom in by a factor of 1.25 and `−` (`-`, the keypad `−`) zoom out by it, around the window's centre,
from the fitted size up to 5 times it; `0` SHALL fit it again. Arrows and `h`/`j`/`k`/`l` SHALL pan by an eighth of the
window, `Space`/`PgDn` and `Shift+Space`/`PgUp` by a window height less 10%, `g`/`Home` and `G`/`End` to the top and
the bottom; every key movement and zoom is a smooth movement of 150 ms. Panning SHALL stop where the image's edge meets
the window's edge; an image smaller than the window in a direction stays centred in it. The wheel SHALL pan by three
lines a step (with `Shift` sideways), the touchpad SHALL pan with the fingers, `Ctrl` (on macOS `⌘`) with the wheel
and a touchpad pinch SHALL zoom around the pointer, and the mouse SHALL pan by dragging. A click on the image without dragging closes
the view, as a tap does on Android. `Esc` or `q` SHALL close it.

#### Scenario: Reading a detail of a schematic
- **WHEN** the reader opens a schematic, presses `+` four times and `l` twice
- **THEN** the schematic is shown 2.44 times its fitted size and moved left by a quarter of the window, its right edge
  never past the window's right edge

#### Scenario: Back to the whole picture
- **WHEN** the reader presses `0` in a zoomed image
- **THEN** the whole image fits the window again, centred

### Requirement: Table view
A table SHALL open set at the reader's text size, in the book's face, with the page's table rules, header cells bold and
no vertical rules, its columns as wide as their content (at least their longest word, at most 20 em; longer text wraps
inside the cell with hyphenation), as on Android, and the view starts at the table's top left corner. `+` and `−`
SHALL change the size by a factor of 1.25 between 50 % and 300 % of the text size in steps of 5 %, `0` SHALL return to
100 % and the top left corner; the table SHALL be typeset again at every size, so its text stays sharp, keeping the
point at the window's centre where it was. Panning (keys, wheel, touchpad, mouse drag) is as in the image view, with a
margin of 24 px around the table (56 px above it, under the close button); a table narrower or lower than the window is
centred in that direction. `Ctrl` (`⌘`) with the wheel and a touchpad pinch change the size like `+`/`−`, around the pointer.
`Esc` or `q` SHALL close the view.

#### Scenario: A wide table
- **WHEN** the reader opens a table of eight columns that is set at 0.8 of the text size on the page
- **THEN** the view shows it at the text size with every column as wide as its text up to 20 em, and `l` pans to its
  right columns

#### Scenario: A larger size
- **WHEN** the reader presses `+` three times in a table view
- **THEN** the table is set at 195 % of the text size and its text is as sharp as the page's

### Requirement: Markers and links in the table view
The note markers and links inside a table SHALL work in its view: a click opens them, `Tab` and `Shift+Tab` cycle through
them in reading order (panning so the focused one is in the window) and `Enter` opens the focused one. A note or a link's
short target SHALL open in the note sheet over the view, and closing the sheet returns to the view. A link to another
place in the book SHALL close the view and jump there, remembered for `Ctrl+O`. A link outside the book opens in the
default browser.

#### Scenario: A footnote in a table cell
- **WHEN** the reader clicks a note marker in a cell of the table view
- **THEN** the note opens in the sheet over the table, and `Esc` returns to the table view

### Requirement: The view's cost
The view SHALL draw nothing while it does not move: no CPU used by Forzac while it is open and idle. Panning and
zooming an image and panning a table SHALL only move what is already drawn, missing no frames at 120 Hz on a Raspberry
Pi 5 and at 60 Hz on the M1 MacBook Pro. Typesetting a table for the view SHALL take at most 50 ms on a Raspberry Pi 5
for a table of 100 rows; opening an image SHALL not decode it again when its page has shown it.

#### Scenario: An open view at rest
- **WHEN** a table view is open and the reader does nothing for 10 seconds
- **THEN** Forzac uses no CPU in that time
