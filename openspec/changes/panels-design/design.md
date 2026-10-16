## Context

The reference is the Android app: the prototype canvas ("Форзац — макеты": `Settings.dc.html`, `Toc.dc.html`,
`Footnote.dc.html`) and its implementation (`SettingsSheet.kt`, `TocScreen.kt`, `NoteSheet.kt`, `ReaderColors.kt`).
Sizes below are the design's dp taken as logical pixels.

## Decisions

**Palette.** Each theme: paper, ink, secondary (`muted`), lines, panel (`sheet`), link — the table of `docs/design.md`.
The page uses paper, ink, link; panels use the rest.

**Fonts for the interface.** Golos Text Regular, Medium and SemiBold are bundled (`assets/fonts/golos-text/`, OFL) and,
with the book families, added to the default fontconfig configuration of the process at start, so GTK labels can use
them by name without installing anything. Book and chapter titles in panels use PT Serif (the design's book serif);
font chips use their own family.

**Icons.** Lucide paths (24×24 viewBox, stroke 1.75, round caps and joins) drawn by a small widget with GSK paths
(GTK 4.14), in the widget's CSS colour; no icon files or libraries.

**Settings sheet.** Bottom, at most 560 px wide, top corners 22 px, panel colour, a 36×4 grabber, padding 10/20/24, rows
18 px apart: theme circles (44 px, "Aa" in the theme's ink on its paper, ring in the current ink around the shown
theme, name below in 11 px secondary); a 44 px row of the size segment (`A` 14 px, the size, `A` 22 px; dimmed at the
ends) and the alignment pair (48 px icons); font chips (40 px, radius 20, name in 15 px of that font, selected filled
with ink); "Line spacing" and "Margins" segments of three icons with 12 px labels; "Pages" segment of two words. A
selected segment is filled with ink and its content drawn in the panel colour. While the theme follows the system the
shown theme is ringed like a chosen one.

Keys: `j`/`k` (`↓`/`↑`) move through the seven controls in reading order — theme, size, alignment, font, spacing,
margins, pages; the current control is outlined in ink; `h`/`l` (`←`/`→`) change its value. The buttons take no focus,
so a click changes a value without moving the keys' place.

**Contents.** The side panel stays (reading beside it), at the design's insides: title "Contents" (17 px semibold) and
a close icon; the filter field (44 px, radius 22, panel colour, search icon, always shown; `/` focuses it); the book:
a 44×66 cover (the book's cover image, else a plain cover with the title), the title (PT Serif 18 semibold, two lines),
"author · N% read" (13 px secondary); a rule; entries at least 48 px high, indented 16 px per level, in PT Serif 16 (the
current one bold with a 6 px dot), the position in 13 px secondary on the right. The keyboard selection is a light ink
wash.

**Note.** Bottom sheet like the settings (at most 560 px, 60 % of the window high, scrolling), grabber, label "NOTE"
and the note's title in 12 px medium secondary capitals, then the text typeset by the core at 17/18 of the text size
with line height 27/17. The core's note layout no longer sets the title as a subtitle: the label carries it.

## Risks / Trade-offs

- Adding fonts to the default fontconfig configuration affects only this process.
- Golos Text adds about 300 KB to the package.
