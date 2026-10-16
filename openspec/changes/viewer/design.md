## Context

Images are scaled down to the column and tables squeezed into it (`typesetting`), so schematics and wide tables cannot
be read. The Android app opens both full screen with a tap (`ImageViewer.kt`, `TableViewer.kt`; parity rows V1, V2 in
[`docs/android-parity.md`](../../../docs/android-parity.md)). The desktop gets one view for both, reached from the
keyboard the way note markers and links already are (`Tab`, `Enter`).

What the Android viewers do, and what is kept:

- Image (`ImageViewer.kt`): fitted to the screen, pinch up to ×5, double tap ×2.5, pan within the edges, a tap or a
  swipe down closes, fades in 200 ms and out 150 ms. Kept: fit, ×5, pan within the edges, click closes, the fades.
  Keys replace the double tap; the swipe down has no desktop counterpart.
- Table (`TableViewer.kt`): the text size, columns by content without squeezing (at most 20 em, longer text wraps),
  page rules, pan both ways, pinch 50–300 % re-typesetting in 5 % steps, a close button. Kept, with the page's own cell
  padding and gap so the view looks like the table on the page. Android draws links in cells but does not open them;
  the desktop opens them (the proposal).
- Only images at least a third of the column wide open (`HitTest.kt` `imageAt`): ornaments do not.

## Goals / Non-Goals

**Goals:** the view in both shells, keyboard first; the core's part (areas, the table for the view) tested on its own;
nothing drawn while the view is idle.

**Non-Goals:** formulas, rotating, saving or copying an image; opening images or tables from inside the note sheet; a
setting of any kind.

## Decisions

**Images and tables are areas — core.** `Target` gains `Image(String)` (the image id) and `Table(Locator)` (the
table's first row: chapter, block, offset 0). `render_body` and `render_flow` add them to `PageContent::areas` with the
markers and links, so `Tab` order, clicks, the "Scroll" tiles and the C ABI need no second list:

- an image whose laid-out width is at least a third of the text width gets an area of its rectangle;
- a table gets one area per page (per column stretch in "Scroll"): the first of its rows drawn inserts it *before* that
  row's cell areas, later rows extend its rectangle (the union). So `Tab` reaches the table before its cells' markers;
- `Content::Row` keeps its row index, so the first row's block is `block − row` (a table's rows are consecutive blocks
  in every parser: `fb2.rs` `read_table`, `epub/xhtml.rs`).

Notes and link previews (`layout_blocks`) do not add them: their block indices are the note's, and the note sheet does
not open views. `ChapterLayout` keeps a flag for it. `Target::is_viewable()` tells image and table targets apart for
the shells.

Alternative considered: a separate list of "objects" per page. It would duplicate the area plumbing in the ABI, both
shells' `Tab` handling and the "Scroll" tiles.

**Clicks prefer markers and links.** A table's area covers its cells' links, so the shells hit-test markers and links
first and viewable areas second. A click on a viewable area opens the view instead of turning the page in the side
thirds (Android: "an image is more important than the tap zones").

**`Tab` stops for viewable areas — shells.** `stops` keeps runs of equal consecutive targets as now, and skips a
viewable area whose target an earlier stop already has (a table cut by a "Scroll" tile boundary yields two areas). The
focused image or table is outlined: a 2 px rounded rectangle in the link ink at 0.7, 4 px outside the union of its
areas; markers and links keep their soft box.

**The table for the view — core.** `layout_table(book, at, style, typesetter) -> Option<TableView>` with
`TableView { content: PageContent, width, height }`, content at the origin. It reuses `table_row` (cells, padding,
rules, bold headers, markers and links) with a different `Geometry`: `natural_geometry` measures each column as
Android's `TableViewer` does — `max(word, min(line, 20 em))` + 2 px over the cells of span 1, the page's gap of 1 em
between columns — and puts it into the typesetter's table cache before the rows are laid out. `Geometry` gains its
total `width`, which the row's rules use (the page's is still the text width). Rows stack without page breaks; the
group rule already sets them without spacing. The view's style is `Settings::table_style(zoom)`: the page style at
`text_size × zoom` with no margins. A view uses its own `Typesetter`, so the page's caches are not disturbed.

**C ABI.** `ForzacArea` gains `at` (a `ForzacLocator`); `kind` 2 is an image (`target` its id), 3 a table (`at` its
first row). New:

- `const uint8_t *forzac_image(doc, id, size_t *len)`: the image file's bytes (the same pointer the pages give, so the
  Mac's decoded-image cache hits);
- `ForzacTable *forzac_table(doc, at, zoom, settings)`, `ForzacPage forzac_table_page(table, float *width, float
  *height)`, `forzac_table_free(table)`: a table typeset for the view, owned by its handle, so its pointers stay valid
  while the page view keeps working on the document. The arrays a `ForzacPage` points to move into a `Published`
  struct shared by the document and the table handle.

**The view's geometry — shells.** Both shells keep the same small model: the content's size at the current zoom, its
top-left corner `pos` in the window, and padding (image 0; table 24 px, 56 px on top under the close button). `clamp`
centres the content in a direction where it (with padding) is smaller than the window, else keeps the padding as the
limit. Zooming about a point `p` keeps the content point under `p`: `pos' = p − (p − pos) × size'/size`, then clamps.
Images zoom in ×1.25 steps between 1 and 5 (the fitted size is ×1); tables between 0.5 and 3, rounded to 5 % steps,
re-typeset only when the rounded step changes. Pan keys move by an eighth of the window, `Space`/`PgUp`/`PgDn` by 90 %
of its height. Key moves and image zooms are a 150 ms ease-out cubic movement (as "Scroll"), on the frame clock (GTK)
or Core Animation (macOS); a table zoom is immediate (the old drawing cannot be morphed into the re-typeset one).
Touchpad, wheel, drag and pinch move immediately. The model is a few lines; each shell has its own rather than a C ABI
for arithmetic.

**GTK.** `gtk/src/viewer.rs`: a `Viewer` (`gtk::Overlay`) with a drawing child and the round close button (the
panels' `Icon` `X` on the sheet colour), added to the window's overlay above the page by `main.rs` when the page view
asks (`Open::Image(id)`, `Open::Table(at)`), focused, with its own `ShortcutController`, `GestureClick`, `GestureDrag`,
`GestureZoom` and `EventControllerScroll`. The image is the page's `gdk::Texture` (the page view's cache, decoded on its
worker; decoded here only when missing), drawn with `append_scaled_texture` (linear up, trilinear down). The table is
one render node made by the page view's `content_node` from `layout_table`, cached per zoom and theme, translated by
`pos`; GSK culls what lies outside the window. The fades set the widget's opacity on a tick callback. Areas in the
table call back into the page view's `open` (made public); a link to a place in the book closes the view first
(`PageView::jumps`). The note sheet remembers the widget that had the focus and gives it back on close, so `Esc` in a
note over the view returns to the view. A theme change redraws the view (the appearance watch in `main.rs`).

**macOS.** `macos/Viewer.swift`: an `NSView` over the container, first responder while open. The image is one
`CALayer` whose contents is the decoded `CGImage` (the page view's `pictures` cache, by the bytes' address) and whose
frame follows `pos` and the size, animated by Core Animation for key moves (implicit actions off for touchpad and
drag). The table is drawn as window-sized bitmap tiles in a container layer moved by `pos` (as "Scroll" tiles):
tiles are built for the window and the target of a movement before it starts, and tiles far away are dropped. Keys by
physical key code in `keyDown`; `magnify(with:)` for the pinch, `⌘`+wheel zooms (⌃+wheel is the system's zoom).
Fades with `NSAnimationContext`. `Reader.run` opens it for `Command.image(id)` / `.table(at)` and recolours it on a
theme change; `NoteSheet.dismiss` gives the focus back to the view under it.

**Help.** `?` and the READMEs gain "Tab … images and tables" and a line for the view's keys.

## Risks / Trade-offs

- [A huge table at 300 % is a big node / many tiles] → GSK culls nodes outside the clip; the Mac keeps at most a dozen
  tiles. Typesetting time is measured on the Pi against the 50 ms budget in the corpus test's largest table.
- [`block − row` assumes consecutive rows] → true for every parser; `layout_table` checks that the block is row 0 of a
  table and returns `None` otherwise.
- [A click on a table now opens it instead of turning in the side thirds] → as on Android; the middle third of a table
  also opens it, which is what a reader clicking a table wants.
- [An image smaller than the window is scaled up to fit] → as on Android; trilinear/linear filtering keeps it smooth.

## Verification

Core tests: areas for an image ≥ ⅓ and < ⅓ of the column, one table area before the cells' link area on a page, none
in notes; `layout_table` column widths (20 em cap, longest word), height = sum of rows, areas of markers in cells; ABI
tests for `forzac_table` and `forzac_image`. The corpus test typesets every table for the view and prints the slowest.
By hand on the Pi 5 (GTK, over VNC) and the M1 Mac: a schematic from `Eymishen_Elektronika-Net-nichego-proshche-` and
a wide table from the corpus, keys, mouse, touchpad, both page modes, a note in a table cell, theme change while open;
`scripts/pi-headless.sh` page-turn and idle measurements unchanged.
