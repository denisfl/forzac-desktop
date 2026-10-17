## MODIFIED Requirements

### Requirement: The highlights list and export
The contents panel SHALL have a "Highlights" tab after "Bookmarks": the book's highlights in reading order, each with a
3 px ochre rule on its left, its text (PT Serif 16/23, at most six lines) and below it "section · p. N" (the book page
it starts on) once the book's pages are counted (`reader-menu`), before that "section · N%". `j`/`k` move,
`Enter` (or a double click) SHALL go to the page where it starts, remembered for `Ctrl+O` like a contents jump, `d`
SHALL delete it, `c` copy its text. `Ctrl+C` (`⌘C`) SHALL copy all the book's highlights as Android exports them: the
title and authors joined by " — ", a blank line, then each highlight with every line prefixed "> ", its section on a
line "— Section" when it has one, highlights apart by a blank line; on macOS a share button in the panel's header
shares the same text. Without highlights the tab SHALL say "No highlights yet" and "Select text with the mouse, or press
v on a page, then choose Highlight."

#### Scenario: Copying all highlights
- **WHEN** a book has two highlights in chapters 3 and 7 and the reader presses `Ctrl+C` on the "Highlights" tab
- **THEN** the clipboard holds the title line, then "> " lines of each highlight with "— Глава 3" and "— Глава 7"

#### Scenario: The page of a highlight
- **WHEN** the book's pages are counted and a highlight in "Глава 3" starts on page 41
- **THEN** its row shows "Глава 3 · p. 41" (in Russian "Глава 3 · стр. 41")
