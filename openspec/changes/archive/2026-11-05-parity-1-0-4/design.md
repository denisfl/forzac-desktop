## Context

Android's breaker (Minikin) cannot split a word with a no-break space inside, and its `justify-gaps` fix is a second pass that loosens such ties where a line is set with holes. The desktop's breaker is Knuth–Plass over boxes, glue and penalties; a tie is inside a word's box, and hyphenation already splits the letters around it.

## Decisions

**A tie is a penalty, not a second pass.** In Knuth–Plass the natural form of "break here only if it saves a hole" is a penalty: the breaker takes it only when the demerits it saves are larger. The cost 5000 is about that of a line with spaces three times their width (100 · 3³ = 2700 plus the line's own); 0, 100, 300, 1000, 2000, 3000 and 5000 were measured, and above 1000 the holes no longer go down while the lines ending in a short word keep going down. The glue after the penalty is as wide as the no-break space and does not stretch, as on Android, so a tie that holds sets exactly as before.

**Justified text only.** A ragged line has no holes to save; Android loosens ties in justified text only.

**The shelf's group is part of the key.** `sort_key` prefixes the key with its group (0 digits, 1 the interface's alphabet, 2 the rest), so every view (Title, Author, Series, Shelves) orders the same way without a second comparison.

## Risks

- About one line in 140 ends in a one-letter word, where without it there would be a hole.
- Within a group, letters compare by code point after lower case: accented Latin letters come after "z". Android compares with a collator; the books in the corpus do not show the difference.
