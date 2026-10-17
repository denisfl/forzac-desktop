## Context

Android (`TextSelection.kt`, A spec quotes): a long press selects a word, a drag grows the selection by words, handles
move the ends, holding a handle at the page's edge turns the page; a bar "Highlight · Copy · Share" in the inverted
theme sits over the selection; a highlight is a quote `(start, end, text, chapter)`, drawn as ochre bands under the
text (`markerColor`: `#C49A52` at 32 %, 30 % on dark themes; `drawBands`: 2 px wider, 3 px corners) that appear with a
stroke (`strokeOver`: 90 ms a line, 180–600 ms); a tap on a marker offers "Delete · Copy · Share"; the contents have a
"Highlights" tab (`QuoteList`) and an export of all highlights (`QuoteExport`). Sharing one quote sends «text» and
"— Author, «Title»".

On the desktop the core already gives a page's (or a column stretch's) rectangles over a text range (`marks`, for
search), lines keep each glyph's text offset, and the reading state is one text file with bookmarks
(`bookmarks`). The maintainer chose (2026-10-17): keyboard selection as Vim's visual mode (a word cursor, `v` anchors),
mouse drags snapping to words, no "Share" on Linux, Android's English words ("Highlight", "Highlights").

## Goals / Non-Goals

**Goals:** Android's selection, bar, highlights and list, with the mouse and fully from the keyboard; copying with the
platform keys; highlights by Locator range, surviving re-layout.

**Non-Goals:** see the proposal. Also not: a text cursor for editing, selecting in the note sheet, the image view or
the table view; rectangular (column) selection.

## Decisions

**Words come from the core, motions too.** `ChapterLayout::words(i, style, book)` and `flow_words(y0, y1, style,
book)` return the words drawn on a page (or a column stretch, in the coordinates `render_flow` uses): one `Word`
per word piece on a line — its rectangle (the line's height, its glyphs' extent) and the whole word's `[start, end)`
Locators. A piece is a run of glyphs over non-white-space characters; its range is widened over the block's text to the
neighbouring white space, so a hyphenated word's two pieces carry the same range. Table cells are text with their
offsets in the row (`Table::cell_starts`). Pure functions over a word list serve both shells: `word_at(words, x, y)`
(the piece under a point, else the nearest on the nearest line), `step(words, i, ±1)` (the next distinct word, `None`
past the ends), `line_step(words, i, ±1)` (the piece on the next line whose middle is nearest), `find(words, at)`
(the piece of the word starting at a Locator) and `span(a, b)` (the range from the earlier word's start to the later
word's end). Paragraph (triple click): `Book::block_range(at)`.

**One selection state in the shell.** `Selection { cursor: (Locator, Locator), anchor: Option<(Locator, Locator)> }`:
the cursor's word and the anchor's word. The selected range is `span(anchor, cursor)` when anchored. A drag sets the
anchor at the press and moves the cursor; `v` starts the mode with the cursor and no anchor. Locators, not indices,
are kept, so a turn, a scroll, a re-layout or a theme change keeps the selection; the word list is recomputed for what is
shown. Keys of the mode are handled before the page keys; a jump, opening a panel or closing the window ends it.

**Crossing pages.** Keyboard: a step with `None` (past the page's or screen's last word) turns the page (Slide) or
moves the column by a line at once (Scroll), then the cursor goes to the first (or last) word of what is now shown —
for `j`/`k` the one nearest the old middle. Mouse in "Slide": a timer of 0.5 s starts when the pointer enters the
24 px zone at the window's left or right edge, or above or below the text, and turns the page when it fires (again
every 0.5 s while the pointer stays); in "Scroll", a tick callback moves the column by 8 × the distance outside the
window per second while the pointer is out.

**Drawing.** The selection: rounded bands of `marks()` in the ink at 14 % over the page (as the search match), with
handles at its ends (a 2 px bar the line's height, a 5.5 px knob above the start and below the end, in the link ink),
recomputed each frame while it changes. The cursor: a soft box in the link ink as the `Tab` focus. Highlights: bands
under the text in the marker colour, baked into the cached page node (GTK) or page bitmap (macOS) and into the
"Scroll" tiles, so turning and scrolling cost what they cost before; a change drops the cached pages or tiles of that
chapter. The stroke of a new highlight: GTK draws the baked page without it, then the bands at the stroke's progress
under the page node, each frame; macOS keeps the old bitmap and lays the new one over it, masked by band-shaped layers
whose widths grow one after another (Core Animation, no app work per frame). Dark themes are those whose paper is
darker than mid grey (Dark, Wave, Forest), as for the ribbon.

**Highlights as `Tab` stops and click targets.** A new `Target::Highlight(start)` (never emitted by the core) lets the
shells add a highlight's line rectangles to a page's areas, in reading order before the first area that comes after
its first line; the existing `stops`, focus drawing and click hit-testing then cover it. A click or `Enter` on one
shows its bar.

**The bar.** Drawn by the page view like the "← Back" pill: Golos Text 15 medium in the paper colour on the ink,
14 px corners, items with 16 × 12 px padding, rules in the paper at 20 %, a soft shadow; centred over the selection's
rectangles on screen, 14 px above them, or below when there is no room. Items' rectangles are kept for clicks. It fades
in over 120 ms. Labels: selection "Highlight · Copy" (+ "Share" on macOS); highlight "Delete · Copy" (+ "Share").

**Text, clipboard, share.** `Book::text(from, to, usize::MAX)` (already used by bookmarks) gives the copied text.
`quotes::share_text(book, text)` and `quotes::export(book, highlights)` are ports of Android's `shareQuote` and
`QuoteExport.text`, tested with Android's cases. GTK puts text on the display's clipboard (`gdk::Clipboard::set_text`);
macOS on `NSPasteboard.general` and shares with `NSSharingServicePicker` at the bar's "Share" item (or the panel's
share button).

**Storage.** `quote<TAB>book-id<TAB>c<TAB>b<TAB>o<TAB>c<TAB>b<TAB>o` lines in the state file; `Store::quotes(book)`
and `Store::set_quotes(book, &[(Locator, Locator)])`, the bookmarks' rules. The text and section are not stored: they
come from the book. The list: `quotes::entries(book, highlights)` (text, section, position), the bookmarks' pattern.

**C ABI.** `forzac_page_words`, `forzac_flow_words` (arrays valid until the next call), `forzac_word_at`,
`forzac_word_step`, `forzac_word_line`, `forzac_word_find` (over a word array the shell passes back),
`forzac_block_range`, `forzac_text` (a range's text), `forzac_share_text`, `forzac_quotes_export`, the document's
highlights (`forzac_quote_add`, `forzac_quote_remove`, `forzac_quote_places`, `forzac_quotes` entries) with
`forzac_state_load_quotes`/`forzac_state_save_quotes`, and `forzac_page_marks`-style band rectangles already there
(`forzac_marks`, `forzac_flow_marks`). GTK uses the Rust functions.

**Keys.** On the page `v` (GTK `v`, macOS key code `0x09`) starts the mode; in it `h l j k`, arrows, `v`, `o`,
`Enter`, `c`, `Ctrl+C`/`⌘C`, `s` (macOS), `Esc`. In the panel's "Highlights" tab `j k Enter d c`, `Ctrl+C`/`⌘C`.
`Ctrl+C`/`⌘C` on the page without a selection does nothing. macOS: Edit → Copy `⌘C`, View → Highlights.

## Risks / Trade-offs

- [Word lists per frame while dragging in "Scroll"] → the list is for the window's stretch only and is recomputed when
  the column moves, not per pointer event; a page's list is cached with its node.
- [Baked highlights make every page node depend on them] → only the chapter of a changed highlight drops its cache;
  re-rendering a page costs what turning to an uncached page costs.
- [`Tab` order with highlights and links on one line] → highlights go before the first area after their first line;
  a link inside a highlight comes after it, as in reading order.
- [Two processes editing highlights of one book] → as bookmarks: the last write wins for that book.
