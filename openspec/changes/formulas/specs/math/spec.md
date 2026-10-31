## ADDED Requirements

### Requirement: Formulas in the model
A MathML formula SHALL be read into a tree: tokens (`mi`, `mn`, `mo`, `mtext`), rows (`mrow`, `mstyle`, an unknown container as a row), fractions (`mfrac`), scripts (`msub`, `msup`, `msubsup`), limits and accents (`munder`, `mover`, `munderover`), roots (`msqrt`, `mroot`), fences (`mfenced` with `open`, `close`, `separators`), matrices (`mtable`, `mtr`, `mtd` with `columnalign`, `columnlines`) and spaces (`mspace width`). `semantics` SHALL be its first child; `annotation` and `annotation-xml` MUST NOT show. A formula in a paragraph SHALL be one character of its text. A formula with `display="block"`, or alone in its paragraph outside a table, SHALL be a block of its own, centred.

#### Scenario: A matrix in a line
- **WHEN** a paragraph reads "Let <math><mi>A</mi><mo>=</mo><mfenced open="[" close="]"><mtable>…2×2…</mtable></mfenced></math>."
- **THEN** its text is "Let ￼." with one formula: `A`, `=`, and brackets around a 2×2 matrix

#### Scenario: A formula alone in its paragraph
- **WHEN** a paragraph holds only a chain of equalities, without `display="block"`
- **THEN** the formula is a centred block of its own

#### Scenario: Annotations
- **WHEN** a formula is `semantics` with a TeX `annotation`
- **THEN** the TeX does not show

### Requirement: Typesetting a formula
A formula SHALL be set with one-letter variables in italics, names and numbers upright; spaces around binary operators and relations, none after a unary minus; scripts smaller (0.71, at least half the text size) and moved down and up; a fraction's numerator over its denominator with a rule on the axis; ∑, ∏ and ∫ in a line with limits at the side, in a display formula above and below; an accent over a whole letter; a root sign with a rule over the radicand; fences stretched to the height of their contents; a matrix in aligned columns with its `columnlines`. It SHALL be drawn in the theme's ink, and a theme change SHALL only redraw it. It MUST NOT show a missing-glyph box for a character of the bundled math font.

#### Scenario: A subscript
- **WHEN** the text has `<msub><mi>a</mi><mn>11</mn></msub>`
- **THEN** an italic "a" and a small "11" below the baseline

#### Scenario: A sum in a line
- **WHEN** the text has `<munderover>` with `∑`, `i=1` and `n`
- **THEN** the sign, "i=1" at its lower right and "n" at its upper right, without touching the lines around

#### Scenario: A matrix
- **WHEN** a paragraph has a 3×3 matrix in brackets
- **THEN** brackets the matrix's height and three aligned columns, on a line taller than the others, the lines around it untouched

#### Scenario: Another theme
- **WHEN** the reader presses `t` on a page with formulas
- **THEN** they are drawn in the new theme's ink without typesetting again

### Requirement: A formula wider than the column
A formula wider than the column SHALL shrink to fit, to 0.55 of its size at the least (as on Android: smaller, it could not be read), without a break inside it. A click on a shrunk formula, or `Tab` to it and `Enter`, SHALL open it in the view for images and tables at the book's text size, with the view's zoom from 50 % to 300 % and panning, set again at each zoom so that it stays sharp; `Esc` closes it. A shrunk formula SHALL take a click before the turning zones; a note marker and a link inside or over it SHALL come before it. A click on a formula that fits SHALL be an ordinary click on the page.

#### Scenario: A long chain of equalities
- **WHEN** the reader clicks a shrunk chain of equalities
- **THEN** it opens at the book's size and pans sideways with the arrows and a drag

#### Scenario: A formula that fits
- **WHEN** the reader clicks a formula that fits, in the right third of the page
- **THEN** the page turns and the view does not open

### Requirement: A formula's text
Each formula SHALL have a linear text: tokens with spaces around operators, `a/b`, `x^2`, `x_i`, `√(x)`, a matrix as `[a b; c d]`. Selection SHALL take a formula as one character. Quotes, copying, sharing, search results, the screen reader's text of a note and the Kindle's MOBI SHALL show the linear text. A note's panel is typeset like the page, so a formula SHALL show there as a formula. The search SHALL find text on both sides of a formula as usual and MUST NOT match inside its linear text.

#### Scenario: A quote
- **WHEN** the reader selects "the matrix A = [1 2; 3 4] is invertible", with the formula inside, and copies it
- **THEN** the clipboard holds that text

#### Scenario: A book sent to the Kindle
- **WHEN** the reader sends a book with formulas to the Kindle
- **THEN** the MOBI has their linear text, and no replacement character

### Requirement: Speed with formulas
Typesetting a chapter with 1,000 inline formulas SHALL stay within the budget of `performance` for a chapter's layout, 50 ms on the Raspberry Pi 5 when its waiver is lifted, and within 25 ms on the M1 MacBook Pro. Turning pages full of formulas SHALL miss no more frames than a book without them, at 120 Hz on the M1 MacBook Pro, and nothing SHALL be drawn at rest.

#### Scenario: Turning a textbook
- **WHEN** the reader turns twenty pages of formulas on the M1 MacBook Pro
- **THEN** `--turns 20` reports no more missed frames than for the same run on a novel, and CPU is idle between turns
