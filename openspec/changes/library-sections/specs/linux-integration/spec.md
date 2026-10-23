## MODIFIED Requirements

### Requirement: Window size
On Hyprland, Forzac windows SHALL have no title bar drawn by the system or by the toolkit, since the compositor draws
none and arranges the windows; the library's header (`library`, `The head on Linux`) SHALL be the top row of its window
there. On floating compositors the library's header SHALL be its window's title bar with the window buttons, and a book
window keeps the toolkit's title bar. On floating compositors a book window SHALL open at 760 × 980 logical pixels and
the library at 1040 × 760, each no larger than 85% of the monitor's width and height, so that a window fits a small
screen whole. The library SHALL work in windows down to 360 × 294 logical pixels (GNOME's smallest adaptive size),
which a tiling compositor may give it.

#### Scenario: Hyprland
- **WHEN** Forzac starts in a Hyprland session
- **THEN** its windows have no title bar, fill the tile Hyprland gives them, and the library's header is its top row

#### Scenario: A small screen
- **WHEN** a book opens on the Pi 5's 1280 × 673 Raspberry Pi Connect desktop
- **THEN** the whole window, title bar included, is on the screen

#### Scenario: A floating library
- **WHEN** the library opens on GNOME
- **THEN** its title bar is the library's header — "+", search, the switcher, the main menu and the window buttons in
  one row
