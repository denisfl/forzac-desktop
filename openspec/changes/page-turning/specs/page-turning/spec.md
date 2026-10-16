## Purpose

How the reader moves through the text: page by page ("Slide") or as one continuous column ("Scroll"), with which keys,
how it animates and what it costs.

## ADDED Requirements

### Requirement: Slide
In the "Slide" mode the window SHALL show one page. `j`/`l`/`→`/`↓`/`Space`/`PgDn` (and `Enter` without a focused
marker) SHALL turn forward, `k`/`h`/`←`/`↑`/`Shift+Space`/`PgUp`/`Backspace` back, with a 200 ms slide (ease-out) on
the compositor's frame clock; a turn past the chapter's last page continues in the next chapter. A wheel step turns one
page, a touchpad swipe at most one. Nothing SHALL be drawn while no turn runs.

#### Scenario: Reading on
- **WHEN** the reader presses `Space` on the last page of a chapter
- **THEN** the first page of the next chapter slides in

### Requirement: Scroll
In the "Scroll" mode the book SHALL be one continuous column: the chapters' text without page breaks, the next chapter
following the previous one after a wide gap, no running head or foot. `j`/`↓` and `k`/`↑` SHALL scroll by a line,
`Space`/`PgDn`/`l`/`→`/`Enter` and `Shift+Space`/`PgUp`/`h`/`←` by a screen less a line, `Ctrl+D`/`Ctrl+U` by half a
screen, each as a smooth movement of at most 150 ms on the frame clock; a new key during a movement continues from where
it is. The wheel SHALL scroll smoothly by three lines a step; the touchpad SHALL move the column with the fingers and
glide on after a flick. Scrolling stops at the start and the end of the book. Nothing SHALL be drawn while the column
does not move.

#### Scenario: Reading across a chapter boundary
- **WHEN** the reader scrolls past the end of a chapter
- **THEN** the next chapter's start follows in the same column, without a jump or a wait

### Requirement: Switching modes
`m` SHALL switch between "Slide" and "Scroll", keeping the line at the top of the text where it was: the first line of
the page becomes the top line of the column, and the top line of the column becomes a line of the page shown. The mode
SHALL be remembered with the other settings. Contents, notes, links, `Tab` focus, the way back and the remembered
position SHALL work in both modes (in "Scroll" the `Tab` focus goes through the markers and links on screen).

#### Scenario: Switching to Scroll
- **WHEN** the reader is on a page starting with "Однажды во время лыжного кросса" and presses `m`
- **THEN** the column shows that line at the top, and pressing `m` again shows the page starting with it

### Requirement: Frame budget
Inside a page turn or a scroll movement no frame SHALL miss the display's refresh at 120 Hz on a Raspberry Pi 5,
crossing chapter boundaries included, and while idle the reader SHALL use no CPU.

#### Scenario: Scrolling a chapter
- **WHEN** the reader scrolls by a screen twenty times in a row on a Raspberry Pi 5 at 120 Hz
- **THEN** no frame of the movements is late
