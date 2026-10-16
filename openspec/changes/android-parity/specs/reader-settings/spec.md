## MODIFIED Requirements

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
