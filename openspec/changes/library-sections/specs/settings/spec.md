## MODIFIED Requirements

### Requirement: The settings window
The settings SHALL open from Forzac › Settings… and `⌘,` on macOS (when no book window is the key window,
`macos-app`), and from the main menu's "Settings" and `Ctrl+,` on Linux (`library`). On macOS they SHALL be a window
of their own in the system's appearance and with the system's controls: not resizable, its minimise and zoom buttons
dimmed, brought forward when already open, with a toolbar of two panes that cannot be customised — "General"
("Основные": "Theme", then "Language") and "Books" ("Книги": "Sync", which shows the Books folder) — the last viewed
pane shown on opening and its name the window's title. On Linux they SHALL be a sheet over the dimmed library in the
library's theme, modal, not resizable, with the sections "Theme", "Language" and "Sync" in that order; in a window
narrower than 550 logical pixels the sheet SHALL fill the window's width from its bottom edge. On macOS `⌘W` and `Esc`
SHALL close the window; on Linux `Esc`, "Close" ("Закрыть") and a click beside the sheet SHALL close it. Opening the
settings SHALL NOT sync or read any book; they SHALL show within 100 ms with ten device files in `.forzac/`, and SHALL
draw nothing while idle.

#### Scenario: Opening the settings on Linux
- **WHEN** the reader presses `Ctrl+,` in the library
- **THEN** a sheet with "Theme", "Language" and "Sync" covers the dimmed library, and `Esc` closes it

#### Scenario: Opening the settings on the Mac
- **WHEN** the reader presses `⌘,` with the library in front, clicks "Books", closes the window with `⌘W` and presses
  `⌘,` again
- **THEN** the window titled "General" opens, then shows the "Books" pane titled "Books", closes, and opens again on
  "Books"

#### Scenario: A narrow window on Linux
- **WHEN** the library window is 470 pixels wide and the reader opens the settings
- **THEN** the sheet rises from the bottom edge across the whole width

## ADDED Requirements

### Requirement: Theme
The settings SHALL offer the six reading themes — "Light", "Sepia", "Dark", "Dawn", "Wave", "Forest" ("Светлая",
"Сепия", "Тёмная", "Рассвет", "Волна", "Лес") — as the "Aa" panel shows them: each a swatch of its paper with "Aa" in its
ink and its name under it, the current theme marked. It is the reading theme itself (`reader-settings`), not a new
setting: choosing one SHALL save it as the reading theme and SHALL repaint at once the library, the settings sheet on
Linux and every open book window, keeping each book's place (a redraw only). Until a theme is chosen the current one
SHALL be the one that follows the system's light or dark appearance, as in a book. Arrows or `h` `l` SHALL move between
the swatches and `Enter` or `Space` choose one on Linux; on macOS the swatches SHALL be reachable with full keyboard
access.

#### Scenario: Changing the theme without a book
- **WHEN** no book is open and the reader chooses "Forest" in the settings
- **THEN** the library turns Forest's paper and ink at once, and the next book opens in Forest

#### Scenario: A book is open
- **WHEN** a book is open on page 37 and the reader chooses "Dark" in the settings
- **THEN** the book window turns dark on page 37 without re-typesetting

### Requirement: The Books folder in the settings
The Books folder's line in "Sync" SHALL have "Show" ("Показать"), which opens the folder in Finder or the file manager
as `F` does (`books-folder`), beside "Change…".

#### Scenario: Showing the folder from the settings
- **WHEN** the reader presses "Show" beside the Books folder's path
- **THEN** the Books folder opens in the file manager
