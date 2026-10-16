## MODIFIED Requirements

### Requirement: Contents
`o` SHALL open the contents in a panel over the page, laid out as the Android design's contents: a "Contents" title
with a close button, a filter field, the book (cover, title, author and the share read), then the book's titled
sections with their nesting and their position in the book (in percent), the current one bold and marked. The entry of
the current place is selected; `j`/`k` or `↓`/`↑` move the selection, `Enter` jumps to the entry and closes the panel,
`/` filters the entries by the typed text (ignoring case and the difference between "ё" and "е"), `Esc` clears the
filter or closes the panel, `o` closes it. A book without contents shows its chapters. While the panel is open, keys do
not turn pages.

#### Scenario: Jumping to a chapter
- **WHEN** the reader presses `o`, types `/` and "наука", and presses `Enter`
- **THEN** the book opens at the first matching section and the panel closes

### Requirement: Notes
A note SHALL open in a bottom sheet as in the Android design: a grabber, the label "NOTE" with the note's title, then
the note typeset like the book (same fonts, hyphenation and justification) in the current theme, scrollable when long.
`Esc` or `Enter` closes it; the reading position does not change.

#### Scenario: Reading a footnote
- **WHEN** the reader opens a note marker
- **THEN** the note's label and text appear over the page and `Esc` returns to the unchanged page
