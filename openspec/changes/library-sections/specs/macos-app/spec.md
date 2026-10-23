## MODIFIED Requirements

### Requirement: Mac conventions
The shell SHALL have the standard menus: Forzac (Settings `⌘,`, Quit `⌘Q`), File (Open `⌘O`, Open Recent, Add a Folder…
`⇧⌘O`, Show the Books Folder, Change the Books Folder…, Close `⌘W`), Edit (Copy `⌘C`, Find `⌘F`), View (in a book window:
Contents, Bookmarks, Highlights, Pages or Scroll, theme, text size; in the library: Books `⌘1` and Reading stats `⌘2`,
the shown one ticked; Enter Full Screen `⌃⌘F`), Go (next and previous chapter, Back, Bookmark `⌘D`). Every command of the
library's toolbar (`library`, `The head on macOS`) SHALL be in these menus too. Forzac › Settings… and `⌘,` SHALL open
the reader's text settings ("Aa") when a book window is the key window, and the settings window (`settings`) otherwise.
Edit › Find and `⌘F` SHALL search the book in a book window and focus the library's search field in "Books".

#### Scenario: Closing
- **WHEN** the reader presses `⌘W`
- **THEN** the window closes and the position is saved

#### Scenario: Bookmarking from the menu
- **WHEN** the reader chooses Go → Bookmark
- **THEN** the page gets a bookmark, as with `b`

#### Scenario: Copying from the menu
- **WHEN** text is selected and the reader chooses Edit → Copy
- **THEN** the text is on the clipboard and the selection is gone

#### Scenario: ⌘, in a book and in the library
- **WHEN** the reader presses `⌘,` in a book window, closes the panel, switches to the library and presses `⌘,` again
- **THEN** the first opens the "Aa" panel and the second the settings window

#### Scenario: The sections from the menu
- **WHEN** the library is the key window and the reader chooses View › Reading stats, then presses `⌘1`
- **THEN** the library shows "Reading stats", ticked in the View menu, and then "Books"
