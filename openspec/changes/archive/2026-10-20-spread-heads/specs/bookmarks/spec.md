## MODIFIED Requirements

### Requirement: Bookmarks on a spread
A spread (`page-turning`) SHALL count as marked when a bookmark lies on either of its pages. `b` (on macOS also `⌘D`)
and a click in the ribbon's corner of the right page (its top margin from 44 px left of its text column's right edge to
the window's right edge) SHALL set a bookmark at the start of the left page on an unmarked spread and remove every
bookmark on both pages of a marked one. The ribbon SHALL hang in the right page's outer margin, beside its column, whichever page holds the
bookmark — never in the gutter — drop in and go up as on a single page, and slide with its spread.

#### Scenario: Setting a bookmark on a spread
- **WHEN** the reader presses `b` on an unmarked spread
- **THEN** the ribbon drops beside the right page's column, and the bookmark is the left page's start

#### Scenario: A bookmark set on a single page
- **WHEN** a bookmark lies on what is now the right page of a spread and the reader presses `b`
- **THEN** the ribbon over the right page goes up and the bookmark is gone from the list
