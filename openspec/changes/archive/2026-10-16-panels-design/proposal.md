## Why

The GTK panels were built from the specs' words alone: the settings panel is a list of "name — value" lines, the
contents and the note use GTK's default look. The agreed design is the Android app's (the prototype linked from the
Android repository's `docs/design.md`: `Settings.dc.html`, `Toc.dc.html`, `Footnote.dc.html`, and `SettingsSheet.kt`,
`TocScreen.kt`, `NoteSheet.kt`); the desktop panels must look like it.

## What Changes

- The full palette of `docs/design.md` (paper, ink, secondary, lines, panel, link) in the GTK shell; the panels use it.
- Golos Text (OFL) bundled as the interface font; panels set titles of books and chapters in the book serif.
- Lucide icons (stroke 1.75) drawn from their SVG paths.
- Settings: a bottom sheet as in the design — theme circles, `A | size | A` with alignment icons, font chips in their
  own fonts, line spacing and margins as icon segments, page mode as a text segment. Keys stay: `j`/`k` move between
  controls (the current one is ringed), `h`/`l` change it; the mouse picks directly.
- Contents: header with a close button, a filter field, the book (cover, title, author and share read), a rule, then
  the entries in the book serif with the current one bold and dotted, the position on the right.
- Note: a bottom sheet with a grabber and a "NOTE 1" label above the typeset text.

Deliberate differences from Android: the desktop's four fonts (no Noto Serif), two page modes, the contents as a side
panel, positions in percent (the desktop has no book-wide page numbers), English labels until `localization`.

## Capabilities

### Modified Capabilities
- `reader-settings`: the settings panel's look and keys.
- `reader-navigation`: the look of the contents and the note.

## Impact

GTK shell only (`panels.rs`, the palette, fonts registered for the interface); one more bundled font family and its
licence; the core's note layout leaves the title to the panel's label.
