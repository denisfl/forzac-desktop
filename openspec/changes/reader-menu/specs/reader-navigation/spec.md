## MODIFIED Requirements

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
