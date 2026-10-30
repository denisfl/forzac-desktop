## ADDED Requirements

### Requirement: Opening a PDF
A file starting with the PDF signature SHALL open as a book from every place a book opens, whatever its extension, and SHALL be copied into the books folder and shown on the shelf with its first page as the cover and its metadata's title, or the file name without one. A damaged or password-protected PDF SHALL give the usual "Couldn't open" message with its reason.

#### Scenario: From the file dialog
- **WHEN** the reader opens `kleppmann.pdf`
- **THEN** its first page shows, and the library lists the book with that page as its cover

#### Scenario: Password
- **WHEN** the reader opens a PDF that needs a password
- **THEN** the message says the file is protected, and no window opens

### Requirement: Pages as on paper
A PDF page SHALL be shown in its original colours on white (#FFFFFF), without recolouring, in every theme. The menu, panels and page number over a PDF SHALL use the Light theme. The saved theme MUST NOT change.

#### Scenario: Dark theme
- **WHEN** the reader with the Dark theme opens a PDF with diagrams
- **THEN** the page is white, the diagrams keep their colours, and the menu is light

#### Scenario: Back to a book
- **WHEN** the reader closes the PDF and opens an FB2
- **THEN** the FB2 shows in the Dark theme

### Requirement: Pages in the window
A page SHALL fit the window whole. When the window is wider than tall and holds two pages by the existing spread rule, two neighbouring pages SHALL show side by side at one scale. Resizing SHALL keep the page.

#### Scenario: Wide window
- **WHEN** a book-format PDF is open in a full-screen landscape window
- **THEN** two pages show side by side and a turn moves by two

### Requirement: Turning and place
A PDF SHALL turn with the same keys, clicks, wheel and swipe as a book, with the "Slide" animation in either page mode. The foot SHALL show "N / M". The page SHALL be remembered as `Locator(0, page, 0)` and restored on reopening. It MUST NOT be written to the sync file; reading time and pages SHALL go into the reading log.

#### Scenario: Reopening
- **WHEN** the reader closes a PDF on page 300 and opens it again
- **THEN** page 300 shows

#### Scenario: Sync
- **WHEN** the reader spends 15 minutes in a PDF with a sync folder set
- **THEN** the day's line with the time and pages is in the device's file, and the place in the PDF is not

### Requirement: Zoom
A page SHALL zoom from 1× to 4× with `+` `−` `0`, a pinch or `Ctrl`+wheel, around the pointer when there is one, and SHALL be sharp once the gesture ends. While zoomed, arrows, the wheel, the touchpad and a drag SHALL pan, and a turn key SHALL go to the next page at 1×.

#### Scenario: Small print
- **WHEN** the reader pinches on a paragraph of an A4 page
- **THEN** the page grows around the pointer and the text is sharp after the gesture

### Requirement: Menu and contents
The menu SHALL have the title, a slider by page and "Page N of M". "Aa" MUST NOT offer text settings on a PDF. The contents SHALL list the PDF's outline, or say it has none. The "Bookmarks" and "Highlights" tabs MUST NOT show on a PDF.

#### Scenario: Slider
- **WHEN** the reader drags the slider to page 120 and releases
- **THEN** page 120 shows with the "← Back" pill

### Requirement: Search and links
Search SHALL follow the book search's rules: from two characters, ignoring case, ё equal to е, a hyphen at a line's end not interrupting a word. Results SHALL arrive as pages are read, in the background, without delaying turns; a page without text SHALL be skipped, and a PDF with no text at all SHALL say its pages are scanned. A link in the file SHALL go to its target with the "← Back" pill, an external one SHALL open in the default application, and a link SHALL win over the click zones.

#### Scenario: Scan
- **WHEN** the reader searches in a scan of 1904
- **THEN** instead of a list the panel says the pages are scanned

#### Scenario: Link
- **WHEN** the reader clicks "Chapter 5" in the text
- **THEN** the chapter's page shows with the pill, and the pill returns

### Requirement: Speed
Rendering SHALL run off the main thread, with the neighbouring pages prepared ahead. Turning between prepared pages SHALL miss no frames, and nothing SHALL be drawn at rest.

#### Scenario: Turning
- **WHEN** the reader turns twenty pages of a text PDF, one every second or two, on the M1 MacBook Pro
- **THEN** no frames are missed inside a turn, and CPU is idle between turns
