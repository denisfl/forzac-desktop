# bookmarks Specification

## Purpose
Bookmarks: places in a book the reader marks to return to, set and removed with one key, shown on the page by a
ribbon and listed in the contents panel, kept by text position so they survive re-layout and restarts.

## Requirements

### Requirement: A bookmark is a place in the text
A bookmark SHALL be the Locator where the page (in "Scroll", the top line of the screen) started when it was set, never
a page number. A page SHALL count as marked when the Locator of at least one bookmark lies on it: from the page's start
up to the next page's start (the last page of a chapter up to the next chapter's start). In "Scroll" the screen counts
as marked when a bookmark lies on one of its lines below the top margin. Bookmarks SHALL belong to the book by its
content (as the reading position does, `reading-state`) and be kept in the state file with the same rules: written
atomically at once, other processes' entries kept, an unreadable line skipped.

#### Scenario: A larger text size
- **WHEN** the reader sets a bookmark on page 12 of a chapter and makes the text larger
- **THEN** the page that now holds the same text is marked, and no other

#### Scenario: A restart
- **WHEN** the reader sets a bookmark, closes the book and opens it again the next day
- **THEN** the page is marked and the bookmark is in the list

### Requirement: Setting and removing
`b` SHALL set a bookmark on an unmarked page (or screen) and remove every bookmark on a marked one, without a menu or a
question. A click in the top margin of the window from 44 px left of the text column's right edge to the window's right
edge (the ribbon's corner) SHALL do the same, unless it falls on a note marker or link, before the click zones that
turn pages. On the end page of the book `b` does nothing. On macOS, Go → Bookmark (`⌘D`) does what `b`
does.

#### Scenario: Setting a bookmark
- **WHEN** the reader presses `b` on a page without a bookmark
- **THEN** the ribbon drops onto the page and the page does not turn

#### Scenario: Removing it
- **WHEN** the reader presses `b` again on the same page
- **THEN** the ribbon goes up and the bookmark is gone from the list

### Requirement: The ribbon
A marked page SHALL show a ribbon as in the Android app: 16 px wide, hanging 44 px from the top edge of the window,
with a swallowtail end 9 px deep, in the right margin next to the text column (its centre at most 40 px from the text,
in the middle of a narrower margin), bordeaux `#8E2F2A` on light themes and `#B5483F` on dark ones (Dark, Wave,
Forest), with a soft shadow. Setting a bookmark SHALL drop the ribbon from above with a small bounce (a spring with
damping ratio 0.5 and stiffness 280 per second squared, Android's); removing it SHALL take it up in 200 ms. A ribbon
on a page turned to SHALL be there already and slide with its page. In "Scroll" the ribbon hangs at the top of the
window while the screen is marked. Nothing SHALL be drawn while the ribbon does not move.

#### Scenario: Turning to a marked page
- **WHEN** the reader turns to a page with a bookmark
- **THEN** the ribbon slides in with the page, without dropping

#### Scenario: Idle
- **WHEN** a marked page is shown and nothing moves
- **THEN** no frames are drawn

### Requirement: The bookmarks list
The contents panel (`reader-navigation`) SHALL have a "Bookmarks" tab: the book's bookmarks in reading order, each with
the section it is in (the contents entry at or before it), the book page it is on once the book's pages are counted
(`reader-menu`), before that its position in the book in percent, and the first words of the text from it (two lines at
most). The selected bookmark moves with `j`/`k` or `↓`/`↑`; `Enter` (or a double click) SHALL go to its page,
remembered for `Ctrl+O` with the "← Back" pill like a contents jump, and close the panel; `d` or `Delete` SHALL remove
it at once (the ribbon goes if its page is shown) and select the next one. `B` (`Shift+b`) SHALL open the panel on this
tab, or close it when it shows this tab. Without bookmarks the tab SHALL say "No bookmarks yet" and how to set one:
"Press b on a page — a ribbon marks the place."

#### Scenario: Going to a bookmark
- **WHEN** the reader presses `B`, `j` and `Enter`
- **THEN** the second bookmark's page is shown with the ribbon, and `Ctrl+O` returns to where the reader was

#### Scenario: Removing from the list
- **WHEN** the reader presses `d` on a selected bookmark
- **THEN** it is gone from the list and from the page, and the bookmark after it is selected

#### Scenario: The page of a bookmark
- **WHEN** the book's pages are counted and a bookmark is on page 37
- **THEN** its row shows "37" where it showed the percent

### Requirement: Cost
Setting or removing a bookmark SHALL write the state within 10 ms on a Raspberry Pi 5; the ribbon's drop and rise SHALL
miss no frame at 120 Hz there; page turns and scrolling SHALL miss no more frames than without bookmarks; opening the
"Bookmarks" tab with 200 bookmarks SHALL take at most 50 ms on a Raspberry Pi 5.

#### Scenario: Many bookmarks
- **WHEN** a book has 200 bookmarks and the reader presses `B` on a Raspberry Pi 5
- **THEN** the list appears within 50 ms

### Requirement: Bookmarks on a spread
A spread (`page-turning`) SHALL count as marked when a bookmark lies on either of its pages. `b` (on macOS also `⌘D`)
and a click in the ribbon's corner of the right page (its top margin from 44 px left of its text column's right edge to
the window's right edge) SHALL set a bookmark at the start of the left page on an unmarked spread and remove every
bookmark on both pages of a marked one. The ribbon SHALL hang in the right margin of the page holding the spread's first
bookmark, drop in and go up as on a single page, and slide with its spread.

#### Scenario: Setting a bookmark on a spread
- **WHEN** the reader presses `b` on an unmarked spread
- **THEN** the ribbon drops onto the left page, beside its column, and the bookmark is the left page's start

#### Scenario: A bookmark set on a single page
- **WHEN** a bookmark lies on what is now the right page of a spread and the reader presses `b`
- **THEN** the ribbon over the right page goes up and the bookmark is gone from the list
