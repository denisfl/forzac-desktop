## Context

The GTK shell has one mode: a page per window, a 200 ms slide between cached page nodes on the frame clock. The core
paginates a chapter (`ChapterLayout`: block layouts and pages) and renders a page into `PageContent`; the neighbouring
chapters are laid out in the background. Navigation (contents, notes, links, `Tab`, back) and the remembered position
work with pages.

## Goals / Non-Goals

**Goals:** "Slide" as specified; "Scroll" as a continuous column; `m` keeps the top line; the mode remembered; the
frame budget in both.

**Non-Goals:** the macOS shell (`macos-app`); other turning effects; a progress display in "Scroll".

## Decisions

### Core: a column next to the pages
`paginate` also places the chapter's blocks as one column (as Android's `flowOf`): the same gaps (groups, spacing,
empty lines), the chapter start lowered by two lines, no page breaks and no widow rules. `ChapterLayout` gains
`flow_height()`, `render_flow(y0, y1, style)` (the column between two heights, positioned with `y0` at the top),
`flow_y(Locator)` (where the line holding a place is) and `flow_locator(y)` (the first line at or below a height). The
column uses the page style's column, so both modes share the block layouts.

### GTK: tiles of the column
In "Scroll" the view keeps a scroll offset inside the current chapter's column and draws tiles: slices of the column one
view high, rendered by the core and cached as render nodes per chapter and tile. A frame draws at most three tiles,
translated; building a tile happens when it first comes into view and costs as much as building a page. Chapters stack:
the previous and next chapters (the background layouts) are drawn above and below with a gap of four lines; when the
top of the window passes into a neighbour, it becomes the current chapter. A missing neighbour is laid out on the spot
(as a page turn into an unprepared chapter does).

### Movement
Keys and wheel steps set a target offset; a tick callback moves towards it with an ease-out over at most 150 ms and
stops when it arrives, so nothing is drawn while still. Touchpad deltas move the offset directly; GTK's kinetic
deceleration (`decelerate` with a velocity) starts a glide that slows by friction. The reading position follows the top
line (`flow_locator`) and is reported like a turn, so it is saved as before.

### Mode switch and navigation
`m` converts the position: to "Scroll" the offset puts the page's first line where the page showed it (below the top
margin); to "Slide" the page holding the top line. Jumps (`go_to`) set the offset from the target. `Tab` and clicks use
the areas of the tiles on screen. The mode is the setting `turning` (`slide`, `scroll`).

### What smooth scrolling on the Pi took
Measured with `--scrolls 25` across a chapter boundary at 120 Hz; each step removed late frames:
- tiles a screen above and below are built in idle time once the column stops (and after it is placed), so a movement
  only draws;
- each new tile is drawn once off screen with the window's renderer in that idle time, so its glyphs (a new chapter's
  heading in a new size) are rasterised before the frame that shows it;
- the column is drawn on whole pixels: text moved by fractions of a pixel has its glyphs rasterised again;
- a movement's first frame already moves (its clock starts a refresh earlier): a frame that draws what is on screen
  presents nothing, and the next one comes a refresh late;
- while the column moves, a chapter not laid out yet is not laid out in a frame; "Scroll" lays out two chapters ahead
  in the background; layouts left behind are freed on a worker and tiles in idle time; the next background job starts
  in idle time.
Result: 1 late frame (15 ms) in about 1800 frames of scrolling; page turns as before.

### Measurement
`--scrolls N` scrolls by a screen N times like `--turns N`, reporting late frames the same way. Both start 1.2 s after
the first frame, when the neighbouring chapters are laid out, as a reader would.

## Risks / Trade-offs

- A tile built during a movement costs a frame's share on the Pi → tiles are one view high, so at most one new tile
  enters per frame of a fast scroll; measured with `--scrolls`.
- Kinetic gliding depends on the compositor sending deceleration → without it, the touchpad still scrolls directly.
