## Why

The spike has the "Slide" mode without a specification. The agreed modes for desktop are "Slide" (paged) and
"Scroll" (a continuous column, natural for wide screens and long reading with the wheel or `j`/`k`). Both have to keep
the reading position when switched or re-laid out, and stay within the animation budgets measured in the spike.

## What Changes

- "Slide" specified: one page per window, `j`/`l`/`→`/`↓`/`Space`/`PgDn` forward, `k`/`h`/`←`/`↑`/`Shift+Space`/
  `PgUp` back, a 200 ms slide on the frame clock, nothing drawn while idle.
- "Scroll": the chapter as one column with chapters separated by space; `j`/`k` scroll by a line, `Space`/`Shift+Space`
  and `PgDn`/`PgUp` by a screen, `Ctrl+D`/`Ctrl+U` by half a screen; smooth scrolling with the wheel and touchpad.
- `m` switches between the two modes and keeps the line at the top of the window.

## Capabilities

### New Capabilities
- `page-turning`: both modes, their keys, animation and frame budgets (0 missed frames inside an animation at 120 Hz
  on the reference hardware).

### Modified Capabilities
<!-- None. -->

## Impact

Core: a scroll layout from the same block layouts (as `flowOf` on Android). Shells: a scrolling view.
