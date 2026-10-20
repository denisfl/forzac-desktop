## ADDED Requirements

### Requirement: Navigation on a spread
On a spread (`page-turning`) both pages SHALL work as one page does: a click on a note marker, link, wide image, table
or highlight of either page opens it; `Tab` and `Shift+Tab` SHALL cycle through the stops of the left page and then
those of the right page, in reading order; a search match and the selection SHALL show on whichever page holds them,
and a drag SHALL select text across the two pages. Each page SHALL show its own running head (none on a chapter's first
page) and its own foot, "37 of 212" on the left and "38 of 212" on the right once the book's pages are counted (before,
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
