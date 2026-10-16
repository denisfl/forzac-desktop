## MODIFIED Requirements

### Requirement: Slide
In the "Slide" mode the window SHALL show one page. `j`/`l`/`→`/`↓`/`Space`/`PgDn` (and `Enter` without a focused
marker) SHALL turn forward, `k`/`h`/`←`/`↑`/`Shift+Space`/`PgUp`/`Backspace` back, with a 200 ms slide (ease-out) on
the compositor's frame clock; a turn past the chapter's last page continues in the next chapter. A wheel step turns one
page, a touchpad swipe at most one. A click in the left third of the page SHALL turn back and in the right third
forward, unless it falls on a note marker or link (which it opens) or a panel is open (which it closes); a click in
the middle third does nothing. Nothing SHALL be drawn while no turn runs.

#### Scenario: Reading on
- **WHEN** the reader presses `Space` on the last page of a chapter
- **THEN** the first page of the next chapter slides in

#### Scenario: Turning with the mouse
- **WHEN** the reader clicks in the right third of a page, away from markers and links
- **THEN** the next page slides in; a click in the middle third leaves the page as it is
