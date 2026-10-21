## MODIFIED Requirements

### Requirement: Opening books from Finder
Forzac SHALL open FB2 (`.fb2`, `.fb2.zip`), FB3, EPUB, TXT and Markdown (`.md`, `.markdown`) files from Finder (double-click, "Open With", dropping on the
Dock icon or on the library window), one window per book. For text and Markdown Forzac SHALL be offered in "Open With" without becoming their default
app. Started without a book, it SHALL show the library
(`library`) and open the chosen book.

#### Scenario: Double-click
- **WHEN** Forzac is the default app for EPUB and the reader double-clicks a book in Finder
- **THEN** the book opens where it was left

#### Scenario: Markdown from Finder
- **WHEN** the reader right-clicks `answer.md` in Finder and chooses Open With
- **THEN** Forzac is in the list; a double-click still opens the file in the app that opened it before
