## Why

Android 1.0.4 (`360a896`) changed two things the desktop shares rules for: the shelf's alphabetical order (`library-sort`) and holes in justified lines before a tie after a short word (`justify-gaps`). The desktop's shelf ignores leading punctuation but not English articles, and puts Latin before Cyrillic whatever the interface's language. Its Knuth–Plass breaker never breaks a tie, which leaves some lines with holes.

## What Changes

- The library's views compare without a leading "The", "A" or "An", and put titles and names in the interface's alphabet before the others, digits first, as Android does. Equal titles go by author.
- In justified text a tie after a one-letter word may break at a cost, so that it gives way to a hole and to nothing else. Measured on Android's test book: lines with spaces over three times their width halve, and about one line in 140 ends in a short word.

Not in this change, as the desktop's shelf differs by design: Android's "14 books … by author ↓" line and its reverse order (the desktop has five views as tabs), and Android's two-line titles with the share aligned in a row (the desktop shows title, authors and series under each cover). These are in `docs/android-parity.md`.

No setting is added.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `library`: the comparison of the Views requirement.
- `typesetting`: ties after short words.

## Impact

`core/src/library.rs` (`sort_key`, `arrange`), `core/src/layout/mod.rs` (`items`, `tie_in`), `core/src/typography.rs` (`SHORT_WORDS` shared). Russian books' lines change where a tie now breaks; their page counts, and English books, do not.
