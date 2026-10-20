## ADDED Requirements

### Requirement: Two-page spread
In "Slide", when the window is wider than tall and half its width holds a text column of at least 28 em at the current
text size with the margin setting's least side margins on both sides, the window SHALL show a spread: two consecutive
pages of a chapter side by side, each half of the window wide, each typeset as a page of that width (its own margins,
its column at most the margin setting's measure), each with its own running head and foot (`reader-navigation`). Every
chapter SHALL start on the left page of a spread; when a chapter ends on a left page, the right half of its last
spread stays empty paper. The keys, wheel steps, swipes and clicks that turn a page in "Slide" SHALL turn a spread: the
next (or previous) two pages slide in as one with the same 200 ms slide, and a turn past the chapter's last spread
continues with the next chapter's first spread (back: the previous chapter's last spread). The click zones stay the
window's thirds. Turning forward from the book's last spread SHALL show the end page across the whole window. The
reading position after a turn SHALL be the start of the left page. "Scroll" SHALL stay one column as wide as today.
No setting turns spreads on or off.

#### Scenario: A full-screen window on a laptop
- **WHEN** the reader reads at the default text size (20 px) and margins in a full-screen window 1440 × 900 px
- **THEN** two pages stand side by side, each column about 31 em wide, and `Space` slides in the next two pages

#### Scenario: A large text size
- **WHEN** the reader in the same window makes the text 24 px
- **THEN** one page is shown, as the halves would hold only 26 em

#### Scenario: A chapter with an odd number of pages
- **WHEN** a chapter has five pages and the reader turns forward from the spread of its pages 3 and 4
- **THEN** page 5 is shown on the left with empty paper on the right, and the next turn shows the next chapter's first
  two pages

#### Scenario: Back across a chapter boundary
- **WHEN** the reader turns back from the first spread of a chapter whose previous chapter has seven pages
- **THEN** the spread with that chapter's page 7 on the left and empty paper on the right slides in

### Requirement: The spread keeps the reading position
Resizing the window into or out of a spread, and changing a setting that re-typesets, SHALL keep the reading position:
the spread (or the page) shown SHALL hold the line that started the page or the left page shown before, which may be
the right page of the new spread. A resize alone SHALL NOT move the reading position, so resizing back shows the page
shown before. `m` from a spread SHALL put the left page's first line at the top of the column; from "Scroll" the
spread shown holds the column's top line.

#### Scenario: Leaving full screen
- **WHEN** the reader is on the spread of pages 6 and 7 of a chapter, starting with "Однажды во время лыжного кросса",
  and makes the window narrower than tall
- **THEN** the single page shown starts at or just before "Однажды во время лыжного кросса"

#### Scenario: Back to full screen
- **WHEN** the reader is on page 5 of a chapter in a narrow window and enters full screen
- **THEN** the spread shown holds the line that started page 5, on its left or right page, and leaving full screen
  again shows the same page 5

### Requirement: Frame budget of a spread
Inside a spread turn no frame SHALL miss the display's refresh at 120 Hz on a Raspberry Pi 5 in a 1920 × 1080 window,
crossing chapter boundaries included, and while a spread is shown and nothing moves the reader SHALL use no CPU.

#### Scenario: Turning spreads
- **WHEN** the reader turns twenty spreads forward on a Raspberry Pi 5 at 120 Hz in a full-screen 1920 × 1080 window
- **THEN** no frame of the turns is late
