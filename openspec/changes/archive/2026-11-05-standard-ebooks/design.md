## Context

A port of the Android app's two changes of 2026-10-28 (`standard-ebooks`, `standard-ebooks-typography`); their
decisions hold here unless this design says otherwise. What the desktop did before:

- `epub::image_type` dropped `image/svg+xml`; an SVG item had no bytes in `Book::images`, its image block took no
  room, and an SVG cover was not found.
- `xhtml::kind_of` looked at the tag and classes; `epub:type` only marked notes. `read_css` read italic and bold of
  `.c` / `tag.c` selectors.
- The list marker already waits for the item's first paragraph (`flush` skips a buffer that is only the marker), and
  the first-line indent is already one box at the paragraph's start, so lines after a `br` start at the edge.
- The shells decode image bytes themselves (GTK: `gdk::Texture::from_bytes`; macOS: Image I/O) and keep the decoded
  images: GTK for the chapter and its neighbours (decoded on the prefetch worker), macOS per document (decoded on the
  main thread when a page is drawn).

Spike (2026-10-28, M1, release build): resvg 0.49 draws "Alice"'s `cover.svg` (a JPEG in a `data:` URL, a `<style>`,
a translucent box, the title as paths) correctly; at 1336 px wide the cover takes 169 ms, the heaviest Tenniel
illustration (3 MB of paths) 115–124 ms, a "Flatland" diagram under 1 ms; peak resident memory 68 MB. Android's
threshold for a raster cache was 100 ms: the cache is needed.

## Goals / Non-Goals

**Goals:** Standard Ebooks' advanced EPUBs read as their compatible ones; their markup reads as their CSS means it;
line art shows in every theme; the rules live in the core, so both shells and iOS share them.

**Non-Goals:** see the proposal.

## Decisions

1. **The core draws SVG (resvg), the shells keep decoding raster images.** One SVG renderer for GTK, AppKit and iOS,
   not librsvg on Linux and WebKit on Apple. resvg is pure Rust, small without its text feature (Standard Ebooks
   draw titles as paths). Rejected: rasterising SVG at import into PNG (the import of "Alice" would wait for 44
   drawings), and gdk-pixbuf's SVG loader (absent on some systems, none on macOS).
2. **`images` module.** `is_svg` (the root element after a BOM, the XML declaration, comments and a DOCTYPE — an XHTML
   page with SVG inside is not an image), `size` (SVG: the root's absolute `width`/`height`, else the `viewBox`, one
   side given: the other by the `viewBox`; raster: `imagesize`), `raster`, `png` (for cover thumbnails) and `is_ink`.
   Only the opening tag is read for the size: layout does not parse megabytes of paths.
3. **One raster per image, fitted to the screen.** The image view's spec says that opening an image does not decode it
   again when its page has shown it, so the page and the view share one raster. It fits the largest screen in pixels
   (the size the view fits an image to; a page is never larger), at most 4 times the SVG's own size (a small ornament
   does not take a screen's memory) and 4096 px on its long side. The screen does not change with the window or the
   settings, so neither re-typesetting nor resizing draws an SVG again.
4. **The raster cache is the core's**, in memory: rasters by a hash of the SVG's bytes and their size, least recently
   used dropped beyond 128 MB. One SVG is drawn at a time (a lock): parsing megabytes of paths takes memory, and a
   shell and a warm-up thread asking for one drawing draw it once. No disk cache: drawing a chapter's pictures again
   after reopening a book costs a fraction of a second on a worker.
5. **Off the main thread.** GTK draws them on its prefetch worker with the chapter's neighbours (the box is taken on
   the main thread, where GDK may be asked about monitors); the texture uses the raster's bytes without a copy.
   macOS draws pages on the main thread, so after laying out a chapter it calls `forzac_warm_images`: a core thread
   draws the SVG images of the chapter, the next and the one before, in that order, and the page finds them drawn.
6. **Line art: the Android thresholds, a core function.** At most a 64 × 64 grid of points; line art when at least
   10 % are transparent (`alpha < 16`) and at least 98 % of the opaque ones (`alpha ≥ 128`) are dark and colourless
   (`max(r, g, b) ≤ 0x50`, `max − min ≤ 0x20`), pixels unpremultiplied first. SVG rasters carry the flag; a shell
   tests a decoded raster image with `is_ink` (GTK: a download of a texture with alpha; macOS: the image drawn into
   64 × 64). Drawn in the theme's ink keeping the pixels' opacity: GTK a mask node (the texture's alpha over a colour),
   macOS a transparency layer with `sourceIn`, and the view a tinted copy made again when the theme changes. The ink
   is applied when drawing: a theme change stays a redraw.
7. **Parsing as on Android.** SVG items outside the spine are images (an SVG document in the spine stays a document);
   the cover is looked for among them. Rare signs are replaced as text is added, before style ranges are counted, and
   in the contents' titles. `epub:type` verse gives `Verse`, a diary no kind (as `div`). A `titlepage` section that
   holds an image keeps only its images when it closes; anchors inside move to its start. A `p` in `hgroup` takes the
   group's last heading level + 1. `hidden` skips the element. A `span` with `i1`…`i9` inside verse starts with that
   many U+2003. CSS: classes with `font-variant(-caps): small-caps`, and the `b`/`strong` rules' small capitals and
   normal weight; a frame may carry two styles (bold small capitals, an italic small-capitals class).
8. **Small capitals by the face.** `Shaper::has_small_caps` reads the face's GSUB for `smcp`; a run in such a face is
   shaped with `smcp` on (plans and the word cache keyed by it). Otherwise its lower-case letters are replaced by
   capitals in a copy of the text used for typesetting only — when the capital is one character of the same length in
   UTF-8 ("ß" stays) — and set at 0.78 of the size. Offsets, marks, search, quotes and the `Locator` read the book's
   text. Unlike Android, the test is per face, so Source Serif italic gets the reduced capitals rather than none.
9. **Verse line indents are room, not glyphs.** Of the bundled faces only Source Serif has U+2003, and a fallback
   font would differ by system or draw a box without one. The typesetter sets each U+2003 as a box exactly 1 em wide.
10. **Hanging verse lines.** Each line of a verse paragraph (up to a `br`) is broken on its own (the paragraph's forced
    breaks split it anyway), its wrapped lines `(n + 1)` em in, `n` its em spaces. The rest of the typesetter takes a
    start for every line instead of "first line, other lines".

## Risks / Trade-offs

- [A heavy SVG on the first page drawn on the main thread on macOS (~0.15 s on an M1) when the warm-up has not reached
  it] → Only when opening a book or jumping straight to an illustration; turning pages finds them drawn.
- [Memory: rasters fitted to a Retina screen are ~10 MB each] → The core keeps 128 MB; GTK's textures share the
  core's bytes; macOS copies into its images, kept per document as raster images already were.
- [resvg's parser on hostile SVG] → It reads only the book's bytes; external references are not resolved; a parse
  failure is a missing image.
- [False line art: a dark drawing whose colour matters] → Nearly black and colourless only; in light themes the ink is
  nearly black.

## Migration Plan

New core and shells; data unchanged. `Locator`s move only within chapters where replaced dashes lengthen text, list
markers joined their items (already so here), hidden subtitles left the book, or chapters were re-split (Keats): a
position past a chapter's end is clamped as after any re-typesetting. Cover thumbnails cached as "none" for advanced
EPUBs stay until the cache is cleared.
