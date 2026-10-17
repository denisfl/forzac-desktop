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
View (Contents, Bookmarks, Pages or Scroll, theme, text size, Enter Full Screen `⌃⌘F`), Go (next and previous chapter,
Back, Bookmark `⌘D`).

#### Scenario: Closing
- **WHEN** the reader presses `⌘W`
- **THEN** the window closes and the position is saved

#### Scenario: Bookmarking from the menu
- **WHEN** the reader chooses Go → Bookmark
- **THEN** the page gets a bookmark, as with `b`

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

### Requirement: Distribution
The Mac app SHALL be distributed as a disk image `Forzac-X.Y.Z-macos-arm64.dmg` downloaded from forzac.truenrth.app,
without a Developer ID, notarisation, the App Store or Homebrew. The image SHALL hold `Forzac.app` and a link to
`/Applications` for dragging it there, under the volume name "Forzac". The app SHALL be signed ad hoc as a whole
bundle (its resources sealed, so `codesign --verify --deep --strict` passes), with the bundle identifier
`app.truenrth.forzac`, the release's version in `CFBundleShortVersionString`, and the app icon. A file
`Forzac-X.Y.Z-macos-arm64.dmg.sha256` in `shasum -a 256` format SHALL be published next to the image.

#### Scenario: Checking a download
- **WHEN** the reader runs `shasum -a 256 -c Forzac-0.2.0-macos-arm64.dmg.sha256` next to the downloaded image
- **THEN** it reports `OK`

#### Scenario: Installing
- **WHEN** the reader opens the image and drags Forzac onto the Applications link
- **THEN** Forzac is in `/Applications` with its icon, and after the first-open step it starts and opens books

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
