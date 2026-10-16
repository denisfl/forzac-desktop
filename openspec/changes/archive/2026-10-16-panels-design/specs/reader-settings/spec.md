## MODIFIED Requirements

### Requirement: The settings panel
`s` SHALL open the settings as a bottom sheet laid out as in the Android design: theme circles, the text size segment
with the alignment pair, font chips set in their own fonts, line spacing and margins as icon segments, the page mode as
a text segment, in the theme's panel colours and the interface font. `j`/`k` (or `↓`/`↑`) move through the controls in
reading order with the current one outlined, `h`/`l` (or `←`/`→`) switch its value, a click picks a value directly, and
the page behind shows the change at once; `Esc` or `s` closes the panel. While the panel is open, keys do not turn
pages. The direct keys stay: `+`/`−` text size, `0` the default size, `t`/`T` theme, `m` page mode.

#### Scenario: Trying a font
- **WHEN** the reader presses `s`, moves to the font chips and presses `l` twice
- **THEN** the page is set in the second font after the current one, at the same place in the text, and that chip is filled

#### Scenario: Looks like the design
- **WHEN** the panel is open in any theme
- **THEN** it shows the same controls, sizes and colours as the design's settings panel in that theme
