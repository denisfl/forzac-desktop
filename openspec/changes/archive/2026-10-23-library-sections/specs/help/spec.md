## ADDED Requirements

### Requirement: The keys in the library
`?` in the library window (and on Linux the main menu's "Keyboard Shortcuts" and `Ctrl+?`) SHALL show the help card
over the dimmed library, laid out and closed as in a book window (`The help's layout`, `Closing the help`), with the
title "Keys" and the library's keys in sections in this order: "Books" (moving the selection, views, opening, about,
removing, search, the Books folder, opening a file, adding a folder), "Reading stats" (periods, Month and Year, share or
save, scrolling), "Window" (choosing the section, settings, closing) — each with the shell's own keys (`library`,
`reading-stats`) — and, under the rule, the support line (`The support line`).

#### Scenario: The library's keys
- **WHEN** the reader presses `?` in the library
- **THEN** the card "Keys" shows "Books", "Reading stats" and "Window" with their key caps, and `?` hides it again
