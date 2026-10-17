## MODIFIED Requirements

### Requirement: Window size
On Hyprland, Forzac windows SHALL have no client-side title bar, since the compositor draws none and arranges the
windows. On floating compositors a book window SHALL open at 760 × 980 logical pixels and the library at 1040 × 760,
each no larger than 85% of the monitor's width and height, so that a window fits a small screen whole.

#### Scenario: Hyprland
- **WHEN** Forzac starts in a Hyprland session
- **THEN** its windows have no title bar and fill the tile Hyprland gives them

#### Scenario: A small screen
- **WHEN** a book opens on the Pi 5's 1280 × 673 Raspberry Pi Connect desktop
- **THEN** the whole window, title bar included, is on the screen
