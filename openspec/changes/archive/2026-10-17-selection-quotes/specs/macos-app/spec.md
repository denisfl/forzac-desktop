## MODIFIED Requirements

### Requirement: Mac conventions
The shell SHALL have the standard menus: Forzac (Settings `⌘,`, Quit `⌘Q`), File (Open `⌘O`, Open Recent, Close `⌘W`),
Edit (Copy `⌘C`, Find), View (Contents, Bookmarks, Highlights, Pages or Scroll, theme, text size, Enter Full Screen
`⌃⌘F`), Go (next and previous chapter, Back, Bookmark `⌘D`).

#### Scenario: Closing
- **WHEN** the reader presses `⌘W`
- **THEN** the window closes and the position is saved

#### Scenario: Bookmarking from the menu
- **WHEN** the reader chooses Go → Bookmark
- **THEN** the page gets a bookmark, as with `b`

#### Scenario: Copying from the menu
- **WHEN** text is selected and the reader chooses Edit → Copy
- **THEN** the text is on the clipboard and the selection is gone
