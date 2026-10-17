## MODIFIED Requirements

### Requirement: Opening books from Finder
Forzac SHALL open FB2 (`.fb2`, `.fb2.zip`), FB3 and EPUB files from Finder (double-click, "Open With", dropping on the
Dock icon or on the library window), one window per book. Started without a book, it SHALL show the library
(`library`) and open the chosen book.

#### Scenario: Double-click
- **WHEN** Forzac is the default app for EPUB and the reader double-clicks a book in Finder
- **THEN** the book opens where it was left
