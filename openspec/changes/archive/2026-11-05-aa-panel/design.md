## Context

The Android change (`SettingsSheet.kt`, `ReaderSettings.kt` `Margins.horizontal`, `ReaderScreen.kt`) is the reference;
its design notes the rejected tries (elements sized by their content gave a ragged right edge). The desktop sheet is
560 px wide at most, like Android's tablet sheet, so the layout is copied in logical pixels for dp.

## Decisions

- **One grid.** Each row is a caption column filling the sheet between its 20 px paddings; halves are two equal
  columns 16 px apart; segments within a group are equal. macOS: `NSStackView`s with `.fillEqually` and width
  constraints to the stack; GTK: homogeneous boxes.
- **Selection colour.** ink × 0.12 + sheet × 0.88, opaque (Android's `compositeOver`). GTK: `mix(sheet, ink, 0.12)`;
  macOS: computed per channel. A segment's content is muted unless selected (`FlatButton.option` on macOS, the
  `option` class on GTK); the size buttons and font chips stay in ink.
- **Dividers.** macOS: 1 px views after every segment but the last; GTK: a left border on every segment but the first,
  so a homogeneous box keeps the segments equal.
- **Font row.** macOS: an `NSScrollView` whose document view places the chips by hand (natural width + 32 px padding,
  plus an equal share of the spare width when they fit). GTK: chips with `hexpand` in a box inside a `Viewport` in a
  `ScrolledWindow` (vertical policy Never): the viewport gives the box the larger of its width and the row's, and GTK
  spreads the spare width equally over expanding children. When the keys pick a chip beyond the edge, the row scrolls
  to it. Unlike Android, the row does not bleed under the sheet's paddings: at desktop widths the chips nearly always
  fit, and GTK has no negative margins.
- **Margins.** `Margins::column` returns the measure in em and the share; `PageStyle::column` takes the share and
  computes `max(share × width, (width − measure × size) / 2)` for the page's width (half the window in a spread, as
  Android's `pageWidth`). `spread` and `spread_inset` use the share of the half for the least margin.
  `PageStyle::for_window` (tests, bench, corpus) uses the medium preset's 7 %.
- **Keys.** The seven controls and their order stay; the keys outline is 4 px around the control everywhere (the theme
  row no longer needs room for a ring).

## Risks / Trade-offs

- [The margins change at the same settings] → in a 760 px window at 20 px the medium margin is 53 px instead of 48;
  in a 1440 px spread the column is 30.96 em instead of 31.2. Positions are `Locator`s; pages are re-counted as on
  any re-typeset.
- [Narrow windows: the wide preset is 13 % of the page] → 65 px at 500 px instead of 72; the reader's choice.
