## MODIFIED Requirements

### Requirement: The top bar
The top bar SHALL be 56 px high in the theme's paper with a 1 px line in the theme's line colour below it: at the left a
back button (a chevron, tooltip "Back to library"), in the middle the title of the section the reader is in (the
contents entry at or before the reading position, else the book's title; medium 15 px, ink) above the book's title
(12 px, secondary), each on one line shortened with "…", at the right a contents button (a list icon, tooltip
"Contents"). The back button SHALL close the book's window (saving the position) and show the recent books (the
library); the contents button SHALL open the contents panel as `o` does. On macOS the menu SHALL draw no top bar of its
own: its top bar SHALL be the reading bar (`macos-app`), which opening the menu shows (fading in, without sliding) and
which stays while the menu is open; when the menu closes, the reading bar fades out as when the pointer stops (at once
when a key or a click closed the menu).

#### Scenario: Back to the books
- **WHEN** the reader opens the menu and clicks the back button
- **THEN** the book's window closes and the recent books show, with this book first

#### Scenario: To the contents
- **WHEN** the reader opens the menu and clicks the contents button
- **THEN** the menu closes and the contents panel opens on the "Contents" tab

#### Scenario: One top bar on macOS
- **WHEN** on macOS the reader presses `Return` on a page
- **THEN** the reading bar and the menu's bottom bar with the slider appear, and no second bar shows under the reading
  bar; `Esc` closes the menu and the reading bar fades
