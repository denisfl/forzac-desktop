# reader-settings Specification

## Purpose
The reader's settings: the agreed set of presets shared with the Android app, their defaults, how they are changed from
the keyboard, what each change costs, and how they are remembered.
## Requirements
### Requirement: The set
The reader SHALL offer exactly these settings, each as a few presets, and no others:
- theme: Light, Sepia, Dark, Dawn, Wave, Forest;
- font: PT Serif (default), Literata, Source Serif, PT Sans, all bundled (OFL, with their licences);
- text size: 16, 18, 20 (default), 22, 24, 27, 30 px;
- line spacing: compact (1.45), normal (1.62, default), loose (1.85) of the text size;
- alignment: justified (default) or left;
- margins: narrow, medium (default), wide — a share of the page's width (4, 7 or 13 %) and the widest text column (38,
  34 or 30 em), as on Android: a page's side margin SHALL be the larger of the share of its width and what keeps its
  line within the measure, for each page of a two-page spread;
- page mode: Slide (default) or Scroll.

#### Scenario: Nothing beyond the set
- **WHEN** the reader opens the settings
- **THEN** exactly these seven settings are listed

#### Scenario: Margins that show
- **WHEN** in a 500 px wide window the reader switches the margins from narrow to wide
- **THEN** the side margin grows from 20 to 65 px, at the same place in the text

### Requirement: The settings panel
`s` SHALL open the settings as a bottom sheet laid out as the Android app's "Aa" panel, in the theme's panel colours and
the interface font. Every row SHALL run from the sheet's left padding to its right padding and split into equal cells:
a 12 px caption above each control, 8 px from caption to control, 20 px between rows. The rows are: Theme — six equal
cards 56 px tall with 8 px between them, each the theme's paper with "Aa" in its ink, the shown one framed 2 px in the
current ink, its name after the caption ("Theme Light"); Size and Alignment, 16 px apart — two equal "A" buttons
without the size's number, dimmed at the scale's ends, and two equal icon segments; Font — chips set in their own
fonts, which stretch evenly to the row's right edge when they all fit and otherwise keep their widths and scroll
sideways; Line spacing and Margins — equal icon segments; Pages — two equal text segments. Buttons, segments and chips
SHALL be 44 px tall with 12 px corners and a 1 px frame and dividers in the theme's line colour. A selected preset
SHALL be shown by a soft fill — the theme's ink at 12 % over the panel — with its content in ink, the others' content
muted; the font chips are always in ink. `j`/`k` (or `↓`/`↑`) move through the controls in reading order with the
current one outlined, `h`/`l` (or `←`/`→`) switch its value, a click picks a value directly, and the page behind
shows the change at once; `Esc` or `s` closes the panel. While the panel is open, keys do not turn pages. The direct
keys stay: `+`/`−` text size, `0` the default size, `t`/`T` theme, `m` page mode.

#### Scenario: Trying a font
- **WHEN** the reader presses `s`, moves to the font chips and presses `l` twice
- **THEN** the page is set in the second font after the current one, at the same place in the text, and that chip has
  the soft fill

#### Scenario: Looks like the design
- **WHEN** the panel is open in any theme
- **THEN** it shows the same controls, sizes and colours as the Android app's "Aa" panel in that theme

#### Scenario: Even edges
- **WHEN** the panel is open in a 760 px window
- **THEN** the left and right edges of the theme cards, the groups, the font chips and the pages segments line up
  with the sheet's paddings, and the cells of each row are equal

### Requirement: Cost of a change
A change of theme SHALL only redraw: the page and the panels cross-fade from the old colours to the new in 200 ms,
missing no frame at 120 Hz on a Raspberry Pi 5, and then draw nothing. A change of font, size, line spacing, alignment
or margins SHALL re-typeset and keep the reading position (the first line of the page stays in the page shown). A
change of page mode keeps the top line (`page-turning`). Fonts are loaded from the bundled files on first use, so the
reader's start does not pay for fonts it does not use.

#### Scenario: Larger text
- **WHEN** the reader in the middle of a chapter presses `+`
- **THEN** the text is larger and the page shown holds the line that started the previous page

#### Scenario: Sepia for the evening
- **WHEN** the reader presses `t` from Light to Sepia
- **THEN** the page fades to Sepia in about 200 ms at the same place, and nothing is drawn afterwards

### Requirement: Light or dark by default
Until the reader chooses a theme, the theme SHALL follow the system appearance: Dark when the desktop prefers a dark
style, else Light, changing with the system while the reader is open. Choosing a theme (`t`, `T` or the panel) fixes it.

#### Scenario: A dark desktop
- **WHEN** Forzac opens for the first time on a desktop set to prefer dark
- **THEN** the book is shown in the Dark theme

### Requirement: Remembered
The settings SHALL be global, remembered through the reading state (`reading-state`) when changed, and applied to every
book opened afterwards. A value that is unknown (an older or newer version) falls back to the default.

#### Scenario: Another book
- **WHEN** the reader picks Literata and loose spacing and opens another book the next day
- **THEN** the other book is set in Literata with loose spacing

