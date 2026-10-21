## MODIFIED Requirements

### Requirement: A file that cannot be opened
When a file cannot be opened as a book — from the command line, a file manager, `xdg-open`, Finder, the Dock, `⌘O`,
the library's "Open a book" button or a file dropped on the library — the reader SHALL say so in a message in its
interface, never only on the terminal: "Couldn't open “name”: "
followed by the reason, worded as in the Android app: "unsupported format — FB2, FB3, EPUB, TXT or Markdown needed" (not a book),
"the file is damaged" (an archive that cannot be unpacked), "the book is DRM-protected and can't be opened",
"the book has no text", or "the file is unavailable" (it cannot be read: missing, no permission). When no book window
is open, the library SHALL follow the message (or stay, when it is open).

#### Scenario: A photo archive from a file manager
- **WHEN** the reader opens `photos.zip` with Forzac from the file manager and no book is open
- **THEN** a message says "Couldn't open “photos.zip”: unsupported format — FB2, FB3, EPUB, TXT or Markdown needed", and after it the
  library is shown

#### Scenario: A file that went away
- **WHEN** a book's file is not readable any more and the reader opens it
- **THEN** the message ends with "the file is unavailable", not "the file is damaged"
