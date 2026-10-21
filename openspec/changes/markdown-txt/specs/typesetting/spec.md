## ADDED Requirements

### Requirement: List items
A list item paragraph (Markdown) SHALL be set as on Android: without a first-line indent, ragged right, hyphenated;
its wrapped lines SHALL start under the text after the marker ("• ", "1. ", "☐ ", "☑ "), not under the marker; a nested
item SHALL start 1.5 em further right per level, four levels at most. Items of one list SHALL have no spacing between
them and half a line around the list.

#### Scenario: A long item
- **WHEN** a bullet item wraps to three lines
- **THEN** the second and third lines start where the text after "• " starts
