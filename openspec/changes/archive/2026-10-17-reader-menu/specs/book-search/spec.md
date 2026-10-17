## MODIFIED Requirements

### Requirement: The search panel
`/` (on macOS also `⌘F`) SHALL open a panel with the search field focused, laid out like the contents panel. Matches
appear as the reader types: their count, then for each a snippet (up to 40 characters before and 80 after, cut at
words, "…" where cut) with the match in bold, and the section and the book page ("section · p. N") once the book's
pages are counted (`reader-menu`), before that the section and the position in the book in percent. `↓` moves into the
list, `j`/`k` move in it, `Enter` or a click goes to a match and closes the panel; `Esc` clears the field or closes the
panel.

#### Scenario: Finding a name
- **WHEN** the reader presses `/`, types "Пепперберг" and presses `Enter`
- **THEN** the book opens at the first match, with the match highlighted

#### Scenario: Pages of matches
- **WHEN** the book's pages are counted and the reader searches a word
- **THEN** each match shows its section and "p. N", the page it is on
