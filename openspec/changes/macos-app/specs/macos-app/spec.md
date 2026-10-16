## ADDED Requirements

### Requirement: The same reader as on Linux
The macOS shell SHALL implement the shared specs — `reading-state`, `reader-navigation`, `page-turning`,
`reader-settings`, `panels-design` — with the same keys, recognised by key code so they work in any keyboard layout.

#### Scenario: A Russian layout
- **WHEN** the keyboard layout is Russian and the reader presses the key labelled `о`/`j`
- **THEN** the page turns as with `j`

#### Scenario: Settings on a Mac
- **WHEN** the reader presses `s` and picks Literata
- **THEN** the panel looks as in the Android design and the page is set in Literata at the same place

### Requirement: Mac conventions
The shell SHALL have the standard menus: Forzac (Settings `⌘,`, Quit `⌘Q`), File (Open `⌘O`, Open Recent, Close `⌘W`),
View (Contents, Pages or Scroll, theme, text size, Enter Full Screen `⌃⌘F`), Go (next and previous chapter, Back).

#### Scenario: Closing
- **WHEN** the reader presses `⌘W`
- **THEN** the window closes and the position is saved

### Requirement: Trackpad
A horizontal two-finger swipe SHALL turn one page in its direction; in "Scroll" two-finger scrolling moves the column.

#### Scenario: Swiping
- **WHEN** the reader swipes left with two fingers
- **THEN** the next page slides in, once per swipe

### Requirement: Opening books from Finder
Forzac SHALL open FB2 (`.fb2`, `.fb2.zip`), FB3 and EPUB files from Finder (double-click, "Open With", dropping on the
Dock icon), one window per book. Started without a book, it SHALL list the recent books and open the chosen one.

#### Scenario: Double-click
- **WHEN** Forzac is the default app for EPUB and the reader double-clicks a book in Finder
- **THEN** the book opens where it was left
