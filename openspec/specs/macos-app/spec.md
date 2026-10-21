# macos-app Specification

## Purpose
The macOS shell: the shared reader behaviour on a Mac, Mac conventions, opening books from Finder, and how the app
is distributed.

## Requirements

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
Edit (Copy `⌘C`, Find), View (Contents, Bookmarks, Highlights, Pages or Scroll, theme, text size, Enter Full Screen
`⌃⌘F`), Go (next and previous chapter, Back, Bookmark `⌘D`). Forzac › Settings… and `⌘,` SHALL open the reader's text
settings ("Aa") when a book window is the key window, and the settings window (`settings`) otherwise.

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

### Requirement: Trackpad
A horizontal two-finger swipe SHALL turn one page in its direction; in "Scroll" two-finger scrolling moves the column.

#### Scenario: Swiping
- **WHEN** the reader swipes left with two fingers
- **THEN** the next page slides in, once per swipe

### Requirement: Opening books from Finder
Forzac SHALL open FB2 (`.fb2`, `.fb2.zip`), FB3 and EPUB files from Finder (double-click, "Open With", dropping on the
Dock icon or on the library window), one window per book. Started without a book, it SHALL show the library
(`library`) and open the chosen book.

#### Scenario: Double-click
- **WHEN** Forzac is the default app for EPUB and the reader double-clicks a book in Finder
- **THEN** the book opens where it was left

### Requirement: Distribution
The Mac app SHALL be distributed as a disk image `Forzac-X.Y.Z-macos-arm64.dmg` downloaded from forzac.truenrth.app,
without a Developer ID, notarisation, the App Store or Homebrew. The image SHALL hold `Forzac.app` and a link to
`/Applications` for dragging it there, under the volume name "Forzac". The app SHALL be signed ad hoc as a whole
bundle (its resources sealed, so `codesign --verify --deep --strict` passes), with the bundle identifier
`app.truenrth.forzac`, the release's version in `CFBundleShortVersionString` and `CFBundleVersion`, and the app icon.
The bundle SHALL embed the update framework (`Contents/Frameworks/Sparkle.framework`, signed ad hoc as its release
comes) with its licence in `Contents/Resources/licenses`, and SHALL carry the EdDSA public key that update images are
verified with; the matching private key SHALL NOT be in the repository. A file `Forzac-X.Y.Z-macos-arm64.dmg.sha256` in
`shasum -a 256` format SHALL be published next to the image.

#### Scenario: Checking a download
- **WHEN** the reader runs `shasum -a 256 -c Forzac-0.2.0-macos-arm64.dmg.sha256` next to the downloaded image
- **THEN** it reports `OK`

#### Scenario: Installing
- **WHEN** the reader opens the image and drags Forzac onto the Applications link
- **THEN** Forzac is in `/Applications` with its icon, and after the first-open step it starts and opens books

#### Scenario: The bundle is sealed with the framework inside
- **WHEN** `codesign --verify --deep --strict Forzac.app` runs on the app from the image
- **THEN** it passes, and `Contents/Frameworks/Sparkle.framework` is there

### Requirement: App icon
The app icon SHALL be the Android app's launcher icon (the marbled endpaper and the page edge) on the macOS icon grid:
a rounded square with continuous corners, 824 pixels on a 1024 pixel canvas, with a soft shadow, in every size an
`.icns` holds from 16 to 1024 pixels. It SHALL be generated from the Android app's vector drawables by the script
that makes the Linux icons.

#### Scenario: The Dock
- **WHEN** Forzac runs
- **THEN** its Dock icon has the same size and shape as the system apps' icons next to it

### Requirement: First-open instructions
The repository SHALL hold the text for the download page that explains, for macOS 14 and for macOS 15 and later, how
to open Forzac the first time despite Gatekeeper: on macOS 14 Control-click › Open › Open; on macOS 15 and later
System Settings › Privacy & Security › "Open Anyway" after the first attempt, then "Open Anyway" and the password.
It SHALL say why (the app is free and not notarised by Apple), that the step is needed once, how to check the
checksum, and the Terminal alternative (`xattr -dr com.apple.quarantine /Applications/Forzac.app`).

#### Scenario: A reader on macOS 15
- **WHEN** a reader on macOS 15 opens Forzac from the image and macOS says it was not opened
- **THEN** following the page's macOS 15 steps opens Forzac, and later opens need no step

### Requirement: The book window's top edge
On macOS a book window SHALL show no title bar of its own: the page SHALL fill the window up to its top edge, under a
transparent title bar with no title drawn and no line under it. The window SHALL keep its title (the book's title) for
the Window menu, Mission Control and the Dock. The page's layout SHALL not depend on whether the reading bar is shown:
the bar lies over the page's top margin, never over its text. The library window SHALL not change.

#### Scenario: A page and nothing else
- **WHEN** a book opens and the pointer does not move
- **THEN** the window shows the page from its top edge to its bottom edge, the running head in the top margin, and no
  title bar, window buttons or toolbar

#### Scenario: The title stays in the Window menu
- **WHEN** two books are open and the reader opens the Window menu
- **THEN** both books are listed by their titles

### Requirement: The reading bar
On macOS a book window SHALL have a reading bar 52 points high across its top, drawn over the page without a background
of its own: at the left the window buttons and a "Back to library" button (a chevron), at the right a Contents button,
a Text settings button ("Aa"), a Search button and a Bookmark button. The page's running head and its ribbon SHALL stay
in sight between the two groups: the bar adds no titles. Under each group the theme's paper SHALL fade out over 24
points towards the middle, so a long running head goes out under the buttons instead of running into them. The
Bookmark button SHALL be drawn outlined, and filled only on a bookmarked page whose ribbon lies under the right group of
buttons (a narrow window), where the ribbon cannot be seen. The buttons SHALL be the system's toolbar buttons with its
symbols, with tooltips naming their keys ("Contents (o)", "Text settings (s)", "Search (/)", "Bookmark (b)", "Back to
library"). Contents, Text settings, Search and Bookmark SHALL do what `o`, `s`, `/` and `b` do; "Back to library"
SHALL do what the reading menu's back button does (`reader-menu`). Dragging the bar's empty part SHALL move the window
and a double click on it SHALL do what a double click on a title bar does in the system's settings.

#### Scenario: Opening the contents from the bar
- **WHEN** the reader moves the pointer and clicks Contents in the bar
- **THEN** the contents panel opens on the "Contents" tab, as with `o`

#### Scenario: No second title
- **WHEN** the reader is on page 7 of "Глава 1. «Нормандия»" and moves the pointer
- **THEN** the bar appears around the page's running head "… Глава 1. «Нормандия»", which stays as it was, and the
  ribbon of a bookmarked page stays in sight

#### Scenario: A narrow window
- **WHEN** the window is 760 points wide, the page has a bookmark and the reader moves the pointer
- **THEN** the running head fades out before the Contents button, the ribbon is under the right group, and the Bookmark
  button is filled; in a 1100-point window the ribbon shows beside the column and the button stays outlined

### Requirement: When the reading bar shows
The reading bar SHALL appear, fading in over 150 ms, when the pointer moves in the book window while it is the key
window. It SHALL fade out over 300 ms 2 s after the pointer last moved, unless the pointer is over the bar or the
reading menu is open; it SHALL fade out at once when a key is pressed, when the reader clicks the page, and when a
panel, the help, a note or the image and table view opens. While a
panel, the help, a note or the view is open, the bar SHALL not appear. When the bar fades out after the pointer stopped
over the page, the pointer SHALL hide too, until it moves again. Keys SHALL never show the bar, except `Return` opening
the reading menu (`reader-menu`). The window buttons SHALL come and go with the bar.

#### Scenario: Reading with the keyboard
- **WHEN** the reader turns pages with `l`, `Space` and `j` without touching the mouse
- **THEN** no bar and no pointer appear over the page

#### Scenario: The pointer stops
- **WHEN** the reader moves the pointer over the page and leaves it there
- **THEN** the bar fades in, and 2 s after the pointer stopped the bar and the pointer fade away

#### Scenario: Resting on the bar
- **WHEN** the reader moves the pointer onto the bar and leaves it there
- **THEN** the bar stays until the pointer leaves it, then fades 2 s later

#### Scenario: A page turned while the bar shows
- **WHEN** the bar is shown and the reader presses `l`
- **THEN** the bar fades out as the next page slides in

### Requirement: The reading bar in full screen
In full screen the reading bar SHALL come and go with the pointer as in a window, on the page's top margin, without the
window buttons (the system shows them with the menu bar when the pointer reaches the screen's top edge). Entering and
leaving full screen SHALL keep the reading position (`page-turning`).

#### Scenario: Full screen
- **WHEN** the reader presses `f` and then moves the pointer
- **THEN** the page fills the screen and the bar appears over its top margin with the back button and the four
  buttons, and fades 2 s after the pointer stops

### Requirement: Cost of the reading bar
Showing and hiding the reading bar SHALL not redraw or re-typeset the page: the fades SHALL run on Core Animation and
SHALL miss no frames on the M1 MacBook Pro's 60 Hz display, also while a page slides. Once the bar has faded out and
the pointer rests, Forzac SHALL use no CPU (no timers or frame callbacks left running).

#### Scenario: A resting window
- **WHEN** the bar has faded out and the reader reads one page for 10 s without input
- **THEN** Forzac's CPU time does not grow over those 10 s
