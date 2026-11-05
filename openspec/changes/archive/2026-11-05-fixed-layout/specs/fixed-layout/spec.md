## ADDED Requirements

### Requirement: A book of picture pages
An EPUB SHALL count as a book of picture pages when its layout is fixed (`rendition:layout` is `pre-paginated` in the package's metadata or on every spine item) and its spine documents in reading order are picture pages: one image without text, an image directly in the spine, or an empty page. Pages that cannot be shown so SHALL be skipped when they are at most 10%; with more, or with no picture at all, the book SHALL read as an ordinary one. Its cover SHALL be the first page when the package names none.

#### Scenario: Manga of JPEGs in the spine
- **WHEN** the reader opens a pre-paginated EPUB whose spine is JPEG files
- **THEN** it is added to the library and opens page by page

#### Scenario: A note at the end
- **WHEN** a book has 12 picture pages and a last page of text
- **THEN** it opens with 12 pages, without the note

#### Scenario: Pictures with text between them
- **WHEN** more than 10% of a fixed-layout book's pages hold text
- **THEN** it reads as an ordinary book

### Requirement: Showing picture pages
Each page SHALL show whole, fitted to the window and centred, on the current theme's paper, and MUST NOT be recoloured. An empty page SHALL show the paper. Keys, clicks, the wheel, a swipe and "Slide" SHALL work as in a book. In a book with `page-progression-direction="rtl"` the next page SHALL be on the left: the left third, `←` and `h` go forward. When the window holds two pages by the existing spread rule, spreads SHALL show: the first page alone, then pairs; in a right-to-left book the first page of a pair on the right. Zoom SHALL work as for a PDF. The theme SHALL change with `t` and `T` without moving the page.

#### Scenario: Manga
- **WHEN** the reader opens a right-to-left book and clicks the left third of the window
- **THEN** the next page opens, sliding in from the left

#### Scenario: Spread
- **WHEN** a right-to-left book is on page 2 in a wide window
- **THEN** pages 2 and 3 show, page 2 on the right

### Requirement: The place in a book of picture pages
The place SHALL be `Locator(0, page, 0)`, restored on reopening, a spread showing with its page. Progress SHALL be (page + 1) / pages, the last page 100%. The place MUST NOT be written to the sync file; reading time and pages SHALL go into the reading log. Pictures SHALL decode off the main thread, the neighbouring pages ahead.

#### Scenario: Reopening
- **WHEN** the reader closes a manga on page 12 and opens it again
- **THEN** page 12 shows, and the library shows 12 / N
