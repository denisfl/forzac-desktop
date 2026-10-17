## MODIFIED Requirements

### Requirement: A file that cannot be opened
When a file cannot be opened as a book — from the command line, a file manager, `xdg-open`, Finder, the Dock, `⌘O`,
the library's "Open a book" button or a file dropped on the library — the reader SHALL say so in a message in its
interface, never only on the terminal: "Couldn't open “name”: "
followed by the reason, worded as in the Android app: "unsupported format — FB2, FB3 or EPUB needed" (not a book),
"the file is damaged" (an archive that cannot be unpacked), "the book is DRM-protected and can't be opened",
"the book has no text", or "the file is unavailable" (it cannot be read: missing, no permission). When no book window
is open, the library SHALL follow the message (or stay, when it is open).

#### Scenario: A photo archive from a file manager
- **WHEN** the reader opens `photos.zip` with Forzac from the file manager and no book is open
- **THEN** a message says "Couldn't open “photos.zip”: unsupported format — FB2, FB3 or EPUB needed", and after it the
  library is shown

#### Scenario: A file that went away
- **WHEN** a book's file is not readable any more and the reader opens it
- **THEN** the message ends with "the file is unavailable", not "the file is damaged"

### Requirement: After the last book
Closing the last book window SHALL show the library (`library`), in both shells; closing the library (`q`, `Esc`,
`Ctrl+W`, `⌘W`) quits.

#### Scenario: Done with a book on Linux
- **WHEN** the reader presses `q` in the only book window
- **THEN** the library appears with that book on the "Continue reading" card, or first on the shelf when it is finished

### Requirement: The screen stays on while reading
While a book window is in front, the reader SHALL keep the display from blanking or sleeping for 10 minutes after
the last key press, page turn, scroll or click in it; after 10 minutes without them the system's own idle rules apply
again. The library and the other applications do not keep the screen on.

#### Scenario: A slow page
- **WHEN** the system blanks the screen after 1 minute and the reader reads one page for 3 minutes without a key
- **THEN** the screen stays on

#### Scenario: A forgotten book
- **WHEN** the reader leaves an open book for more than 10 minutes
- **THEN** the screen blanks by the system's setting
