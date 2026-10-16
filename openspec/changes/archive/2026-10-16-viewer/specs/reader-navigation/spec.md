## MODIFIED Requirements

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

### Requirement: Mouse and touchpad
A scroll wheel step SHALL turn one page (down: forward); one touchpad swipe SHALL turn at most one page. Clicks on
markers and links open them; clicks on images at least a third of the column wide and on tables open them in the view.

#### Scenario: Scrolling with a wheel
- **WHEN** the reader turns the wheel down three steps
- **THEN** three pages are turned forward

#### Scenario: Opening a picture with the mouse
- **WHEN** the reader clicks a photograph as wide as the column in the right third of the page
- **THEN** the photograph opens in the view instead of the page turning
