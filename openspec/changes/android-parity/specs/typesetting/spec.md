## ADDED Requirements

### Requirement: The book's language
The language that chooses hyphenation SHALL come from the book's metadata; a book that names none SHALL get it from
its text, as in the Android app: among the first 2000 letters of its paragraphs and headings, more than half Cyrillic
is Russian, anything else English. Russian and English are hyphenated; a book in another language is set without
hyphenation.

#### Scenario: An English book without a language
- **WHEN** an English FB2 has no `<lang>`
- **THEN** its long words are hyphenated by the English patterns

#### Scenario: A German book
- **WHEN** a book declares `de`
- **THEN** it is set without hyphenation and justified with wider spaces

### Requirement: Code and strikethrough
Preformatted paragraphs (code) SHALL be set in the bundled monospace face (PT Mono, OFL) at 0.85 of the text size,
keeping their line breaks and spaces; inline code SHALL be set in the same face at 0.9. Text in the strikethrough
style SHALL be drawn with a line through it, in the text's ink, at about half the x-height above the baseline.

#### Scenario: A code listing
- **WHEN** an EPUB has a `pre` block with indented lines
- **THEN** the lines are set in PT Mono with their indentation, and the characters line up in columns

#### Scenario: Struck-out text
- **WHEN** a paragraph has `<strikethrough>old</strikethrough>`
- **THEN** "old" is drawn with a line through it
