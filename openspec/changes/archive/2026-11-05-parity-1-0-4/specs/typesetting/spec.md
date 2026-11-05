## ADDED Requirements

### Requirement: Ties after short words
A no-break space that ties a one-letter word to the next (`typography`: "в", "к", "с", "у", "о", "а", "и", "я") SHALL keep them on one line in justified text, unless that would set a line with holes: then the line MAY break after the short word, at the cost of a line about three times looser than it should be. Ragged-right text MUST NOT break there. The text, and so places, quotes, bookmarks and search, MUST NOT change.

#### Scenario: A tie before a hole
- **WHEN** a justified line would have spaces three times their width because "о учительнице" cannot be split
- **THEN** "о" ends the line and the line is set without the hole

#### Scenario: A tie without a hole
- **WHEN** "в качестве" stands in a line that sets well either way
- **THEN** "в" does not end the line, as before

#### Scenario: Measured
- **WHEN** "Оцифруйся или умри" is typeset in a 760 × 980 window with the default settings
- **THEN** fewer than 15 justified lines have a space over three times its width (23 before), and fewer than one line in 100 ends in a tied short word
