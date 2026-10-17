## MODIFIED Requirements

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
