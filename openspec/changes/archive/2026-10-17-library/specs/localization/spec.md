## MODIFIED Requirements

### Requirement: What is translated
Every string Forzac shows SHALL be translated: the panels (contents, bookmarks, highlights, search, settings, notes),
their placeholders and empty hints, the selection and highlight bars, the "← Back" pill, the end page's "Finished",
the contents' "Chapter N" for untitled chapters, the library (its buttons, note line, card, search, question, empty
state and the empty state's sample page), the messages for files that cannot be opened, the
help with its keys left as they are, the macOS menus and the reason the screen is kept on. Book texts, titles, authors,
font names and the "Aa" sample are not. Counts SHALL take Russian plural forms ("1 совпадение", "3 совпадения",
"5 совпадений").

#### Scenario: Searching in Russian
- **WHEN** the interface is Russian and a search finds 21 matches
- **THEN** the panel says "21 совпадение"

#### Scenario: The library in Russian
- **WHEN** the interface is Russian and Forzac starts without a book
- **THEN** the head says "Форзац" and "Открыть книгу", and the card "ПРОДОЛЖИТЬ"
