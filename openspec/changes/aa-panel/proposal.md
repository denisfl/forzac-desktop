## Why

The Android app's "Aa" panel was redesigned (`aa-panel` in the Android repository, branch `aa-panel`, commit b0c0a37):
one grid, two spacings, one way to show the selection, and margins whose difference shows. The maintainer approved it
on the phone (2026-10-24) and asked for the same on the desktop. Today the desktop sheet is the old Android layout:
theme circles with names under them, a size segment with the number in it, a 96 px alignment pair, round font chips,
the selection as a solid ink fill ("ink on paper"), and each row ending at its own right edge. Its margins (24 / 48 /
72 px at least, 38 / 34 / 30 em at most) differ from Android's.

## What Changes

- **Rows from edge to edge in equal cells**, a 12 px caption above each control, 8 px from caption to control and 20 px
  between rows: Theme; Size | Alignment (halves, 16 px apart); Font; Line spacing | Margins (halves); Pages.
- **Theme**: six equal cards (56 px tall, 12 px corners) filling the row, each the theme's paper with "Aa" in its ink;
  the shown one has a 2 px frame in the current ink. No names under the cards: the caption reads "Theme Light".
- **Size**: two equal "A" buttons (15 and 21 px) filling the half; no font-size number; dimmed at the scale's ends.
- **Segments** (alignment, line spacing, margins, pages) equal, 44 px tall, 12 px corners, 1 px line-coloured frame and
  dividers.
- **Selection**: a soft fill, the theme's ink at 12 % over the sheet, with the content in ink; unselected content
  muted. The solid ink fill leaves the panel.
- **Font**: chips 44 px tall, 12 px corners, set in their own font, always in ink; when they all fit they stretch
  evenly to the row's right edge, when not they keep their widths and the row scrolls sideways.
- **Margins**: a share of the page's width — narrow 4 %, medium 7 %, wide 13 % — and the widest column — 38, 34, 30 em:
  a page's margin is the larger of the share and what keeps the line within the measure, per page in a spread. The
  presets' names and the stored setting stay.
- New strings: "Size", "Alignment", "Font".
- Not included: the macOS Settings window's theme swatches, the sheet's own width (560 px), new settings.

## Capabilities

### New Capabilities

### Modified Capabilities
- `reader-settings`: "The set" — margins as a share of the page's width and a measure; "The settings panel" — the
  grid, the cards, the soft selection, the size buttons without a number, the font row.

## Impact

- `core/src/settings.rs` (`Margins::column`, `spread`, `spread_inset`), `core/src/layout/mod.rs` (`PageStyle::column`,
  `for_window`), `core/src/i18n.rs`; the core's tests.
- `macos/Panels.swift` (`FlatButton`, `Segmented`, `captioned`, `SettingsSheet`, `ChipsRow`).
- `gtk/src/settings_sheet.rs`, `gtk/src/panels.rs` (CSS).
- The keys of the panel do not change: `j`/`k` move through the same seven controls, `h`/`l` change them.
- Pages re-typeset differently at the same settings (the margins): positions are `Locator`s and stay.
