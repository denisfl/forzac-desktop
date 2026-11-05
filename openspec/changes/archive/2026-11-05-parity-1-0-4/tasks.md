## 1. The shelf's order

- [x] 1.1 `library::sort_key`: no leading "The", "A", "An"; the interface's alphabet first, digits before it; equal titles by author — check: `the_shelf_puts_the_interfaces_alphabet_first_and_skips_articles`, and the existing view tests

## 2. Ties

- [x] 2.1 Measure first, on Android's test book ("Оцифруйся или умри") with the desktop's defaults (2026-11-05, a counter outside the repository over `ChapterLayout::words`): at 760 × 980, of 8,011 justified lines 385 have a space over 2×, 23 over 3×, 5 over 4×; at 560 × 900, 2,198 / 170 / 29 of 11,096. A tie stands before 13 % of the 2× lines
- [x] 2.2 `layout::tie_in` and `TIE_COST` in justified text: the tie a penalty before a glue as wide as the no-break space. Costs tried 0–5000; chosen 5000: at 760 × 980 12 lines over 3×, 3 over 4×, 56 lines ending in a short word; at 560 × 900 118 and 19, 141 lines — check: the counter
- [x] 2.3 30 books, page hashes before and after: English books identical, Russian books' lines differ, every page count the same; typesetting a whole book within its noise (423 against 441 ms, 702 against 666 ms)

## 3. Check

- [x] 3.1 `cargo test -p forzac-core`, `clippy`, `fmt`; `openspec validate parity-1-0-4 --strict`
