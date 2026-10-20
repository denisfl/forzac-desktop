## Context

The core typesets a chapter for a `PageStyle` (window size, text size, measure and margins, `Settings::page_style`)
and renders a page with its running head and foot (`ChapterLayout::render_numbered`). Each shell keeps one current
chapter layout and a page index in it, caches pages (GTK: render nodes and their `Area`s by page index; macOS: bitmaps
by page index through `forzac_page`), and slides the current page in over the previous one. The background count of
the book's pages (`BookPages`) runs for the same style. Positions are `Locator`s; a relayout puts the page holding the
reading position on screen without moving it.

Decisions taken with the maintainer (2026-10-20): every chapter starts on a left page; spreads from a column of 28 em;
each page its own head and foot; one bookmark per spread, the ribbon over the page that holds it.

## Goals / Non-Goals

**Goals:** one rule, in the core, for when a window shows a spread and how wide its pages are; the shells draw two
cached pages side by side and change little else; a spread turn costs about two page draws, nothing while idle.

**Non-Goals:** a page-curl or other spread-specific animation; inner/outer (asymmetric) margins; pairing across
chapters; spreads in "Scroll"; a setting.

## Decisions

**1. The rule and the page style live in the core.** `Settings::spread(width, height)` is true when `turning` is
"Slide", `width > height` and `width / 2 − 2 × least margin ≥ 28 em` (`MIN_SPREAD_COLUMN`). `Settings::page_style`
returns the page for half the window when `spread` holds, else as today. Every caller then gets the spread page without
change: GTK's relayout, `forzac_paginate`, `forzac_count_new` (the count numbers half-width pages), and the note and
table styles (asked for a 100 000 px tall "window", never wider than tall). Because the style now depends on
`turning`, GTK computes it from its mode (`State::mode`, which `m` changes without touching `State::settings`); macOS
already updates `settings.turning`. `m` therefore re-typesets when the window holds a spread — a typesetting of one
chapter, as a text size change, keeping the `Locator`. Alternative: a shell-side rule — two copies to keep equal, and
the count would need its own width.

**2. Across the C ABI:** only `forzac_settings_spread(settings, width, height) -> bool`, so the macOS view knows to draw
two pages; the page width is half the view's. Every other function keeps its meaning (`forzac_page`, `forzac_marks`,
`forzac_page_words` work on one page in page coordinates).

**3. Pairs within a chapter, the shown page is the left one.** In a spread the page index the shells keep is always
even (the left page); the right page is the next one if the chapter has it. A relayout takes `page_of(locator)` and
rounds it down to even — the `Locator` itself is not moved, so leaving a spread shows the single page that holds it
(the spec's "resize back shows the page shown before"). A turn moves by two pages and sets the reading position to the
new left page's start; turning back into the previous chapter lands on its last even page; chapter edges, `go_to_page`,
jumps and the menu's slider round down the same way. Alternative: pairing by book page (Android) — the book's pages
are only known after the count, so the pairs would change under the reader when it ends.

**4. Right-page geometry is offset once, when a page is cached.** A right page's node (GTK: the page node wrapped in a
translation by the page width; macOS: drawn into the spread bitmap at that x) and its `Area`s (shifted by the page
width) are cached under its own index, so everything that reads areas gets view coordinates: clicks, `Tab` stops (the
left page's then the right page's — the concatenation of the two lists), the focus box, highlights' areas. Words and
mark rectangles for the selection and the search match (`shown_words`, `shown_rects`, the match bars) are the left
page's plus the right page's shifted, so a drag selects across the gutter with the existing code.

**5. Drawing a spread.** GTK: the screen node is a container of the two cached page nodes; the turn slides that node by
the window width exactly as a page today. macOS: the bitmap a layer shows becomes the whole spread (one image of the
view's size with both pages drawn), so the Core Animation turn, the `previous`/`current` layers and their ribbons stay
as they are. The empty right half is paper. The end page is typeset for the page width and drawn centred on the
window (translated by `(window − page) / 2`).

**6. Bookmarks.** The shown range is the left page's start to the right page's end (`page_end` of the last page shown);
`b` and the corner toggle over it with the existing `bookmarks::toggle` (setting at the left page's start). The ribbon
is placed at `style.ribbon()` plus the page width when the range's first bookmark lies on the right page; the corner
is the right page's (`ribbon().1 + page width`). On macOS each page layer's ribbon gets that x; the previous layer's
keeps the one it had.

**7. Unchanged:** the click thirds and the pill (centred on the window), the menu and the lists (book page numbers of
the left page), the reading state (the left page's start is saved), the Scroll column.

## Risks / Trade-offs

- [Twice the glyphs per spread on the Pi 5 at 1920 × 1080] → the turn already draws cached nodes only; measured with
  `scripts/pi-headless.sh --turns` before archiving (task 5).
- [A short chapter leaves half a window of paper] → accepted with the maintainer, as Apple Books does.
- [`m` re-typesets in a spread window] → one chapter, under 100 ms on the Pi 5 (`typesetting` cost), and the
  neighbours are prefetched again.
- [The gutter is twice the outer margin when the window is much wider than two columns] → each page is centred in its
  half, as on Android tablets; asymmetric margins are a later refinement if wanted.
- [The ribbon in the gutter for a bookmark on the left page] → it hangs in the left page's right margin, beside its
  column, as on a single page.
