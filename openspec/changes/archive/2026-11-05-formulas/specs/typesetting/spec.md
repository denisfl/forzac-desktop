## ADDED Requirements

### Requirement: A line as tall as its formula
A line of a paragraph SHALL be as tall as the paragraph's rhythm, or as its tallest formula with 0.15 em above and below when that is more. Other lines MUST NOT change height, and a paragraph without formulas MUST NOT typeset differently from before, to the pixel. A page break MAY fall between such lines as between any others, and a line taller than the page SHALL be drawn as it is, as an image taller than the page is.

#### Scenario: A book without formulas
- **WHEN** a novel is typeset before and after this change
- **THEN** every page holds the same glyphs in the same places

#### Scenario: A fraction in a line
- **WHEN** a line holds a tall fraction
- **THEN** that line is taller, and the lines above and below it keep their distance from each other's text

### Requirement: A math font
A math font SHALL be bundled, under the OFL, and used only inside formulas, for characters the book's face lacks, before the platform's fallback. Characters outside formulas MUST NOT use it. Each shell SHALL rasterise exactly that file.

#### Scenario: A subset sign on a bare system
- **WHEN** a formula has `⊆` and the book's face lacks it, on a Linux system with no math fonts
- **THEN** the sign shows from the bundled font
