# reader-navigation Specification

## Purpose
How a reader moves through a book beyond turning pages: the contents, footnotes, links and the way back, and how the
page shows where the reader is. Everything is reachable from the keyboard; the mouse is a second way.

## Requirements

### Requirement: Contents
`o` SHALL open the contents in a panel over the page, laid out as the Android design's contents: the tabs "Contents",
"Bookmarks" (`bookmarks`) and "Highlights" (`quotes`) with a close button, a filter field (on the "Contents" tab), the
book (cover, title, author and the share read), then the tab's list. `o` opens the panel on the "Contents" tab, `B` on
the "Bookmarks" tab; `←`/`→`, `h`/`l` or `Tab`/`Shift+Tab` on the list, or a click on a tab, switch tabs. The
"Contents" tab lists the book's titled sections with their nesting and, at the right, the book page each starts on
once the book's pages are counted (`reader-menu`), before that their position in the book in percent; the current one
bold and marked. The entry of the current place is selected; `j`/`k` or `↓`/`↑` move the selection, `Enter`
jumps to the entry and closes the panel, `/` filters the entries by the typed text (ignoring case and the difference
between "ё" and "е"), `Esc` clears the filter or closes the panel, `o` closes it. A book without contents shows every
chapter, an untitled one as "Chapter N" (N counted from 1 in reading order), as in the Android app. While the panel is
open, keys do not turn pages.

#### Scenario: Jumping to a chapter
- **WHEN** the reader presses `o`, types `/` and "наука", and presses `Enter`
- **THEN** the book opens at the first matching section and the panel closes

#### Scenario: A book without titles
- **WHEN** a book has no contents and three chapters without titles
- **THEN** the contents list "Chapter 1", "Chapter 2" and "Chapter 3", and each opens its chapter

#### Scenario: From the contents to the bookmarks
- **WHEN** the reader presses `o` and then `→`
- **THEN** the panel shows the "Bookmarks" tab and the filter field is gone

#### Scenario: To the highlights
- **WHEN** the reader presses `o` and then `←`
- **THEN** the panel shows the "Highlights" tab

#### Scenario: Page numbers
- **WHEN** the book's pages are counted and the reader presses `o`
- **THEN** each entry shows the page it starts on, the first chapter's "1"

### Requirement: Active markers and links
The note markers and links of the current page, its highlights (`quotes`), its images at least a third of the text
column wide and its tables SHALL be reachable from the keyboard: `Tab` and `Shift+Tab` cycle through them in reading
order (a table before the markers and links in its cells, a table spanning pages once per page, a highlight at its
first line on the page), highlighting the focused one — a marker, link or highlight with a soft box in the link ink
over each of its lines on the page, an image or a table with a rounded outline in the link ink around it; `Enter` opens
it (an image or a table in the view, `viewer`; a highlight's bar, `quotes`); `Esc` or turning the page removes the
focus. Without focus, `Enter` opens the menu (`reader-menu`). A click on a marker or link opens it; a click on such an
image or table, away from the markers and links in it, opens it in the view.

#### Scenario: A footnote from the keyboard
- **WHEN** a page has two note markers and a link and the reader presses `Tab` twice and `Enter`
- **THEN** the second marker's note opens

#### Scenario: A table between paragraphs
- **WHEN** a page has a link, then a table with a note marker in a cell, and the reader presses `Tab` twice and `Enter`
- **THEN** the table opens in the view; a third `Tab` instead would have focused the marker in the cell

#### Scenario: A highlight from the keyboard
- **WHEN** a page has a link and then a highlight, and the reader presses `Tab` twice and `Enter`
- **THEN** the bar with "Delete" and "Copy" shows over the highlight

### Requirement: Notes
A note SHALL open in a bottom sheet as in the Android design: a grabber, the label "NOTE" with the note's title, then
the note typeset like the book (same fonts, hyphenation and justification) in the current theme, scrollable when long.
`Esc` or `Enter` closes it; the reading position does not change.

#### Scenario: Reading a footnote
- **WHEN** the reader opens a note marker
- **THEN** the note's label and text appear over the page and `Esc` returns to the unchanged page

### Requirement: Links and the way back
A link to a note SHALL open the note. A link to a short place in the book — a target within one chapter of at most
3 blocks and 600 characters of text, such as a figure with its caption, a table or a paragraph — SHALL open those
blocks in the note sheet (`Notes`), typeset like the book, and the page stays. A link to any other place SHALL go to
the page where the target starts. A link outside the book opens in the default browser. Every jump (a link, a
contents entry, a search match) SHALL remember where it started: `Ctrl+O` returns to the place before the last jump,
repeatedly, up to 50 places. After a jump a pill "← Back · Ctrl+O" (on macOS "← Back · ⌃O") SHALL show over the foot
of the page, in the ink with the paper's colour for its text, as the Android pill; a click on it does what `Ctrl+O`
does. The pill SHALL go when the reader returns, or turns or scrolls two pages (in "Scroll" two screens) away from
where the jump arrived; the remembered places stay for `Ctrl+O`.

#### Scenario: Following a cross-reference
- **WHEN** the reader follows "see Chapter 10" from page 5 of chapter 1 and then presses `Ctrl+O`
- **THEN** the book shows chapter 10 with the pill, then page 5 of chapter 1 again, without the pill

#### Scenario: A link to a figure
- **WHEN** the reader opens a link "Figure 2-1" whose target is an image with a one-line caption
- **THEN** the note sheet shows the image and the caption, and `Esc` returns to the unchanged page

#### Scenario: Staying to read
- **WHEN** after following a link the reader turns two pages forward
- **THEN** the pill is gone, and `Ctrl+O` still returns to where the link was

### Requirement: Where the reader is
Every page except the first page of a chapter SHALL show the chapter's title as a running head in the secondary ink,
shortened with "…" when it does not fit. Once the book's pages are counted (`reader-menu`) the foot of the page SHALL
show the page of the book and the book's pages, "37 of 212"; before, the page within the chapter and the position in
the book: "4 / 22 · 37%".

#### Scenario: Middle of a book
- **WHEN** the reader is on page 4 of the 22 pages of a chapter that ends at 40% of the book, before the count
- **THEN** the head shows the chapter title and the foot "4 / 22 · 37%" (or the actual share)

#### Scenario: A counted book
- **WHEN** the book's 212 pages are counted and the reader is on its 37th
- **THEN** the foot shows "37 of 212"

### Requirement: Mouse and touchpad
A scroll wheel step SHALL turn one page (down: forward); one touchpad swipe SHALL turn at most one page. Clicks on
markers and links open them; clicks on images at least a third of the column wide and on tables open them in the view.

#### Scenario: Scrolling with a wheel
- **WHEN** the reader turns the wheel down three steps
- **THEN** three pages are turned forward

#### Scenario: Opening a picture with the mouse
- **WHEN** the reader clicks a photograph as wide as the column in the right third of the page
- **THEN** the photograph opens in the view instead of the page turning

### Requirement: No cost while reading
Opening the contents SHALL take at most 50 ms for a book of 2000 sections on a Raspberry Pi 5; a note SHALL open
within 30 ms; the running head and foot SHALL not change the time to typeset a chapter by more than 5%, and page turns
at 120 Hz SHALL miss no frames.

#### Scenario: A long book
- **WHEN** the contents of a 2000-chapter book are opened on a Raspberry Pi 5
- **THEN** the panel appears within 50 ms

### Requirement: The end of the book
In "Slide", turning forward from the last page of the book SHALL show an end page: in the middle of the window, the
book's title, its authors and "Finished", in the reader's theme, without running head or foot. In "Scroll", the same
end screen, one window high, SHALL follow the last paragraph. Showing it SHALL remember the book as finished: its
position stays at the start of the last page of text and its share read becomes 100 % (the contents and the recent
books show 100 %). Turning back or scrolling up returns to the text; forward keys on the end page do nothing.

Under "Finished" the end page SHALL show the book's totals, the parts that are not zero: its time by the log of every
device (`reading-log`, in the stats' form: "35 min", "6 h"), the days since it was first opened on this computer up to
today inclusive, its highlights and its bookmarks, with plural forms ("6 h · 12 days · 7 highlights"). The time SHALL
show only when the log's first day (any device) is not later than the day the book was first opened here; without a
known first opening neither time nor days show. Under the totals, in the link colour, the current month SHALL show its
finished books and time by the stats (`reading-stats`): "October: 3 books · 14 h" ("Октябрь: 3 книги · 14 ч"). A
click on it, or `Tab` to it and `Enter`, SHALL open the stats window on the current month; closing the window returns
to the end page.

#### Scenario: Finishing a book
- **WHEN** the reader presses `Space` on the last page of the book
- **THEN** the end page with the title, the author and "Finished" slides in, and the recent books show the book at
  100 %

#### Scenario: Back to the text
- **WHEN** the reader presses `k` on the end page
- **THEN** the last page of the text is shown

#### Scenario: The totals
- **WHEN** the reader, who first opened the book here 12 days ago, logged 6 h 10 min on it and made 7 highlights,
  turns past its last page
- **THEN** under "Finished" the end page shows "6 h · 12 days · 7 highlights" and "October: 3 books · 14 h"

#### Scenario: A book begun before the log
- **WHEN** the reader finishes a book opened before the version with the log
- **THEN** the totals show no time and no days, only the highlights and bookmarks; the month shows

#### Scenario: Read on the phone too
- **WHEN** the reader read the book 4 h on the phone and 2 h on the Mac, the devices synced, and the Mac shows its end
  page
- **THEN** the totals start with "6 h"

#### Scenario: The stats from the end page
- **WHEN** the reader presses `Tab` on the end page, then `Enter`
- **THEN** the stats window opens on the current month with the book first under "Finished"; `Esc` returns to the end
  page

### Requirement: Navigation on a spread
On a spread (`page-turning`) both pages SHALL work as one page does: a click on a note marker, link, wide image, table
or highlight of either page opens it; `Tab` and `Shift+Tab` SHALL cycle through the stops of the left page and then
those of the right page, in reading order; a search match and the selection SHALL show on whichever page holds them,
and a drag SHALL select text across the two pages. The left page's running head SHALL be the book's title and the right page's the chapter's title
(as on a single page, none on a chapter's first page), and each page SHALL show its own foot, "37 of 212" on the left and "38 of 212" on the right once the book's pages are counted (before,
each its page within the chapter and the position in the book); an empty right half shows neither. The pill after a
jump SHALL stay centred on the window's foot and go after two spreads turned away. A jump (a link, a contents entry, a
search match, the menu's slider) SHALL show the spread holding its target.

#### Scenario: A footnote on the right page
- **WHEN** the left page has one note marker, the right page has two, and the reader presses `Tab` three times and
  `Enter`
- **THEN** the right page's second marker's note opens

#### Scenario: Page numbers of a spread
- **WHEN** the book's 212 pages are counted and the spread shows its 37th and 38th
- **THEN** the left foot reads "37 of 212" and the right foot "38 of 212"

#### Scenario: Selecting across the gutter
- **WHEN** the reader drags from the last line of the left page to the first line of the right page
- **THEN** the text of both lines up to the pointer is selected, as one selection

#### Scenario: The heads of a spread
- **WHEN** the spread shows pages 5 and 6 of "Глава 1. «Нормандия»" in "Одноэтажная Америка"
- **THEN** the left page's head reads "Одноэтажная Америка" and the right page's "Глава 1. «Нормандия»"
