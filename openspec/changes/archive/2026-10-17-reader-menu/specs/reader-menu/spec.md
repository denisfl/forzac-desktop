## Purpose

The reading menu of the web reader and the Android app over the page: where the reader is in the book, a slider to go
anywhere in it, and the way to the library, the contents and the text settings; and the count of the book's pages it
and the lists read.

## ADDED Requirements

### Requirement: Opening and closing the menu
A click in the middle third of the page (by the window's width, away from markers, links, images, tables, highlights,
the ribbon's corner and the "← Back" pill, with no text selected and no panel open) or `Enter` with no `Tab` focus
SHALL open the menu, in "Slide" and in "Scroll". A click on the page between the bars, `Esc`, or `Enter` with the slider
where it was SHALL close it. Any other key closes the menu and then does what it does without it (`Space` turns the
page, `o` opens the contents, `s` the settings). The bars SHALL fade and slide in from the window's top and bottom edges
by half their height in 200 ms and out in 150 ms, over the page, which does not move. Opening a panel from the menu
closes the menu. Nothing SHALL be drawn while the menu is open and nothing moves.

#### Scenario: Opening with a click
- **WHEN** the reader clicks in the middle of a page with no marker or link there
- **THEN** the top and bottom bars appear over the page; a second click in the middle closes them

#### Scenario: Opening from the keyboard
- **WHEN** no marker is focused and the reader presses `Enter`, then `Esc`
- **THEN** the menu opens, then closes, and the page is the same

#### Scenario: A key closes the menu
- **WHEN** the menu is open and the reader presses `Space`
- **THEN** the menu closes and the next page slides in

### Requirement: The top bar
The top bar SHALL be 56 px high in the theme's paper with a 1 px line in the theme's line colour below it: at the left a
back button (a chevron, tooltip "Back to library"), in the middle the title of the section the reader is in (the
contents entry at or before the reading position, else the book's title; medium 15 px, ink) above the book's title
(12 px, secondary), each on one line shortened with "…", at the right a contents button (a list icon, tooltip
"Contents"). The back button SHALL close the book's window (saving the position) and show the recent books (the
library); the contents button SHALL open the contents panel as `o` does.

#### Scenario: Back to the books
- **WHEN** the reader opens the menu and clicks the back button
- **THEN** the book's window closes and the recent books show, with this book first

#### Scenario: To the contents
- **WHEN** the reader opens the menu and clicks the contents button
- **THEN** the menu closes and the contents panel opens on the "Contents" tab

### Requirement: The bottom bar
The bottom bar SHALL be in the theme's paper with a 1 px line above it, its content at most 640 px wide and centred,
20 px from the sides: once the book's pages are counted, a slider 28 px high over the whole book — a 2 px track in the
line colour, a 1 × 8 px tick at the first page of every chapter but the first, the part read in the ink, a 16 px round
thumb in the ink at the current page; below it at the left "Page N of M" (medium 14 px, tabular figures) above a hint
(12 px, secondary), at the right an "Aa" button (52 × 44 px, rounded, a 1 px line border, serif 18 px) that opens the
settings sheet as `s` does. The hint SHALL be "N min left in chapter" — the characters of text from the reading
position to the chapter's end at 1000 a minute, rounded — or "Less than a minute left in chapter" when that rounds to
0. Until the pages are counted the slider and "Page N of M" SHALL not show; the hint does.

#### Scenario: In the middle of a chapter
- **WHEN** the reader is on page 37 of a counted 212-page book with 4300 characters left in the chapter and opens the menu
- **THEN** the bottom bar shows the thumb at 37 of 212, "Page 37 of 212" and "4 min left in chapter"

#### Scenario: Before the count
- **WHEN** the reader opens the menu before the book's pages are counted
- **THEN** the bottom bar shows the hint and the "Aa" button, without the slider and the page line

### Requirement: Going anywhere with the slider
Pressing on the slider or dragging its thumb SHALL move the thumb to the book page under the pointer and show that
page's "Page N of M" and its section's title in place of the hint, without moving the reading position; releasing SHALL
go to that page (the start of its text), close the menu and be remembered for `Ctrl+O` with the "← Back" pill like a
contents jump. While the menu is open `←`/`h` and `→`/`l` SHALL move the thumb one page back and forward (repeating
while held) with the same labels, and `Enter` SHALL go to the page shown, as releasing does. `Esc` closes the menu and
leaves the reading position where it was.

#### Scenario: Dragging to another part
- **WHEN** the reader drags the thumb of a counted 300-page book to the middle and releases it
- **THEN** while dragging the bar shows "Page 150 of 300" and that page's section, and after release the page shows
  with the "← Back" pill, and `Ctrl+O` returns to where the reader was

#### Scenario: From the keyboard
- **WHEN** the menu is open on page 10 and the reader presses `→` three times and `Enter`
- **THEN** the bar shows "Page 13 of M" and the page's section, and page 13 shows with the menu closed

### Requirement: The book's pages
The reader SHALL count the pages of the whole book for the window's size and the settings in the background, after
the first page is shown and again after any re-typesetting change (size, text size, font, spacing, alignment, margins),
without delaying a page, a turn or a scroll: page turns and scrolls at 120 Hz on a Raspberry Pi 5 SHALL miss no more
frames while the count runs than without it, and once it is done the reader SHALL use no CPU while idle. The count SHALL
paginate every chapter exactly as reading does, so a book page is the page the reader sees. Once counted, the foot of
every page SHALL show the page of the book and their number, "N of M"; before, it shows what it showed. A count for a
size or settings no longer shown SHALL stop and be discarded. The time to count the corpus's longest book on a
Raspberry Pi 5 SHALL be recorded.

#### Scenario: Reading while counting
- **WHEN** a long book opens on a Raspberry Pi 5 and the reader turns twenty pages at once
- **THEN** no turn misses more frames than without the count, and after it the foot shows "N of M"

#### Scenario: Resizing the window
- **WHEN** the reader makes the window narrower
- **THEN** the foot shows the chapter's pages until the book is counted again for the new width, then "N of M" with the
  new number
