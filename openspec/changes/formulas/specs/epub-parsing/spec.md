## ADDED Requirements

### Requirement: MathML in EPUB
An EPUB's `math` element, in the MathML namespace with or without a prefix, SHALL be read as a formula of the `math` capability and not as text. A malformed formula SHALL read as what can be read of it, and MUST NOT lose the text around it.

#### Scenario: A prefixed formula
- **WHEN** a chapter has `<m:math xmlns:m="http://www.w3.org/1998/Math/MathML">`
- **THEN** it reads as a formula

#### Scenario: A malformed formula
- **WHEN** an `mfrac` has one child
- **THEN** that child shows in the fraction's place, and the paragraph's other text is there
