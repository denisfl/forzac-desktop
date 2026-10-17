# reader-navigation Specification

## Purpose
How a reader moves through a book beyond turning pages: the contents, footnotes, links and the way back, and how the
page shows where the reader is. Everything is reachable from the keyboard; the mouse is a second way.

## Requirements

### Requirement: Contents
`o` SHALL open the contents in a panel over the page, laid out as the Android design's contents: the tabs "Contents"
and "Bookmarks" (`bookmarks`) with a close button, a filter field (on the "Contents" tab), the book (cover, title,
author and the share read), then the tab's list. `o` opens the panel on the "Contents" tab, `B` on the "Bookmarks" tab;
`←`/`→`, `h`/`l` or `Tab`/`Shift+Tab` on the list, or a click on a tab, switch tabs. The "Contents" tab lists the
book's titled sections with their nesting and their position in the book (in percent), the current one bold and
marked. The entry of the current place is selected; `j`/`k` or `↓`/`↑` move the selection, `Enter` jumps to the entry
and closes the panel, `/` filters the entries by the typed text (ignoring case and the difference between "ё" and "е"),
`Esc` clears the filter or closes the panel, `o` closes it. A book without contents shows every chapter, an untitled
one as "Chapter N" (N counted from 1 in reading order), as in the Android app. While the panel is open, keys do not
turn pages.

#### Scenario: Jumping to a chapter
- **WHEN** the reader presses `o`, types `/` and "наука", and presses `Enter`
- **THEN** the book opens at the first matching section and the panel closes

#### Scenario: A book without titles
- **WHEN** a book has no contents and three chapters without titles
- **THEN** the contents list "Chapter 1", "Chapter 2" and "Chapter 3", and each opens its chapter

#### Scenario: From the contents to the bookmarks
- **WHEN** the reader presses `o` and then `→`
- **THEN** the panel shows the "Bookmarks" tab and the filter field is gone

### Requirement: Active markers and links
The note markers and links of the current page, its images at least a third of the text column wide and its tables
SHALL be reachable from the keyboard: `Tab` and `Shift+Tab` cycle through them in reading order (a table before the
markers and links in its cells, a table spanning pages once per page), highlighting the focused one — a marker or link
with a soft box in the link ink, an image or a table with a rounded outline in the link ink around it; `Enter` opens it
(an image or a table in the view, `viewer`); `Esc` or turning the page removes the focus. Without focus, `Enter` turns
the page as before. A click on a marker or link opens it; a click on such an image or table, away from the markers and
links in it, opens it in the view.

#### Scenario: A footnote from the keyboard
- **WHEN** a page has two note markers and a link and the reader presses `Tab` twice and `Enter`
- **THEN** the second marker's note opens

#### Scenario: A table between paragraphs
- **WHEN** a page has a link, then a table with a note marker in a cell, and the reader presses `Tab` twice and `Enter`
- **THEN** the table opens in the view; a third `Tab` instead would have focused the marker in the cell

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
shortened with "…" when it does not fit. The foot of the page SHALL show the page within the chapter and the position
in the book: "4 / 22 · 37%".

#### Scenario: Middle of a book
- **WHEN** the reader is on page 4 of the 22 pages of a chapter that ends at 40% of the book
- **THEN** the head shows the chapter title and the foot "4 / 22 · 37%" (or the actual share)

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

#### Scenario: Finishing a book
- **WHEN** the reader presses `Space` on the last page of the book
- **THEN** the end page with the title, the author and "Finished" slides in, and the recent books show the book at
  100 %

#### Scenario: Back to the text
- **WHEN** the reader presses `k` on the end page
- **THEN** the last page of the text is shown
