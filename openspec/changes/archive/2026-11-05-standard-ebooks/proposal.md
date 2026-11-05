## Why

Standard Ebooks is the most carefully made free library of classics in English, and its readers care about typography
as much as Forzac does. The maintainer wants Forzac to be the reader one recommends for Standard Ebooks and Project
Gutenberg, in that order of platforms: Android first, then the desktop, then iOS (which takes this core by tag).

The Android app did it on 2026-10-28 in two changes (`forzac/openspec/changes/archive/2026-10-28-standard-ebooks` and
`…-standard-ebooks-typography`). The desktop reads the same books as the Android app did before them: in Standard
Ebooks' "advanced" EPUBs the cover and every picture are SVG and do not show (Tenniel's 44 in "Alice", the diagrams of
"Flatland"); the advanced title page's caption becomes the book's top heading and Keats's 133 poems collapse into a few
chapters; the two- and three-em dashes are missing from the book fonts; Dracula's diaries are narrowed as quotations;
poems are set as prose; a chapter's title under its number is a body paragraph; verse lines lose their indents; the
small capitals of `<b>` come out bold. And a black diagram on transparency vanishes in a dark theme, in any book.

## What Changes

- **SVG images.** The cover, the title page, the logo and illustrations in SVG show as raster images do: on the page,
  in the image view, in the note panel and as the library's cover. The core draws them (resvg), sized by the root's
  `width`/`height` or its `viewBox`, kept once drawn.
- **The title page.** The text of a `section epub:type="titlepage"` holding an image is a caption for screen readers;
  it is not in the book, and the title page is the image (a cover, by the existing rule).
- **Rare signs.** `⸺`, `⸻` and `℅` become `——`, `———` and `c/o` when parsed, in the text and the contents, as the
  compatible EPUBs have them.
- **Verse and diaries by `epub:type`.** `z3998:poem|verse|song|hymn` are verse; a `blockquote` with `z3998:diary` or
  `z3998:diary-entry` is the book's text, a letter inside it still a quotation.
- **Headings in `hgroup`.** A `p` in an `hgroup` is a heading one level below the group's heading ("I. Of the Nature
  of Flatland"); without one, a subtitle. An element with `hidden` is not shown.
- **Verse line indents.** A verse line with class `i1`…`i9` starts that many em in; the part of any verse line that
  does not fit goes on 1 em right of the line's start.
- **Small capitals.** A new text style, from the book's CSS: classes with `font-variant: small-caps` (Gutenberg's
  `.smcap`) and the rules of `b`/`strong` (Standard Ebooks: small capitals, `font-weight: normal` drops the bold). Set
  with the font's own small capitals (`smcp`: Literata, Source Serif roman), else as capitals at 0.78 of the size.
- **The first-line indent is the first line's.** Lines after a `br` start at the edge, as in HTML (the desktop already
  does this; now specified).
- **Line art in the theme's ink.** An image of dark, colourless lines on transparency is drawn in the text colour of
  the theme, on the page, in the view and in the note panel. Any book, any format.
- A list item's marker with the item's nested paragraph (`<li><p>…`) — the desktop already does it; now tested on the
  Standard Ebooks case.
- No new settings.

## Non-goals

- Book CSS beyond these rules: selectors on attributes or ids, `all-small-caps`, small capitals of headings.
- Plays as tables (`z3998:drama`): read as before.
- Drawing an SVG again for a deeper zoom in the view: one raster per image, fitted to the screen.
- Books added to the library before this change keep their placeholder cover (their thumbnail is cached as "none");
  their pictures show at once.
- The Kindle's MOBI (`send-to-kindle`) carries raster images only: SVG pictures are left out as before.

## Capabilities

### Modified Capabilities
- `epub-parsing`: "Text elements", "Inline styles", "Images and tables".
- `typesetting`: "Images"; new "Line art", "Small capitals", "Lines within a paragraph".

## Impact

- Core: `epub` (`mod.rs`: SVG images and cover, CSS; `xhtml.rs`: title page, rare signs, `epub:type`, `hgroup`,
  `hidden`, verse indents, small capitals), `model::Style::SmallCaps`, a new `images` module (SVG size and raster,
  line-art test), `layout` (SVG size, small capitals, verse breaks), `fonts::Shaper` (`smcp`), the C ABI
  (`forzac_svg_raster`, `forzac_is_ink`, `forzac_warm_images`; `forzac_cover` gives an SVG cover as PNG).
- A new dependency: `resvg` 0.49 (MPL-compatible, Apache-2.0 OR MIT), without text and system fonts; only `data:`
  images inside an SVG are read, nothing outside the book.
- GTK shell: image decoding (`page_view.rs`), the view, the note panel's and library's covers. macOS shell: the same
  in `Core.swift`, `PageView`, `Viewer`; SVG of nearby chapters drawn ahead on a core thread.
- iOS (`forzac-ios`) gets all of it with the core's tag; its shell needs the same three image calls.
