## MODIFIED Requirements

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
