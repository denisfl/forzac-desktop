## Context

The core typesets text paragraphs into glyph runs; images and tables are skipped; every glyph comes from PT Serif.
`fb2-complete` put notes, links, images and tables into the model. Measured on a Raspberry Pi 5: a 103-page chapter
takes 88 ms to typeset, so a synchronous layout at a chapter boundary would stall the page-turn animation.

## Goals / Non-Goals

**Goals:** draw images and tables; mark note references and links and report their areas; fall back to system fonts
for missing characters; typeset neighbouring chapters in the background; keep both shells drawing the same pages.

**Non-Goals:** activating notes and links (that is `reader-navigation`, which uses the areas), formulas, hanging
punctuation, a scroll layout (`page-turning`).

## Decisions

### Page contents instead of glyph runs (core)
`ChapterLayout::render(page)` returns `PageContent { runs, images, rules, areas }`:
- `runs`: glyph runs with a font id and an ink (`Text`, `Link`, `Secondary` for the page number); the shell maps ink to
  the theme's colours, so typesetting stays theme-free (a theme change is a redraw).
- `images`: image id and rectangle; shells decode the bytes with the platform decoder (GDK texture loaders, ImageIO),
  which handles every format books use. The core only reads image dimensions (the `imagesize` crate, headers only).
- `rules`: rectangles with a strength (heavy, medium, light) for table rules; drawn in ink with an alpha per strength.
- `areas`: rectangles of note markers and links with their target.

### Fonts: ids and a fallback provider (core ↔ shell)
Fonts get ids: 0–3 the bundled PT Serif faces, then fallback fonts registered as they are needed. When a character has
no glyph in the run's face, the core asks a `FallbackProvider` (set by the shell) for a font file and face index that
cover it; the file is memory-mapped, registered under a new id, and the run is split by font before shaping.
Results are cached per character. A word is first shaped whole in the book face (a cached shape); only a word with a
`.notdef` glyph is split by character, so the common case costs nothing (checking every character cost 15% of layout
time on the Pi). The bundled faces are read without the registry's lock. Shells look a font up by id (`font_source(id)`: bundled bytes or a file path and
index, plus whether it has colour glyphs):
- GTK: fontconfig's default configuration answers the provider (`FcFontSort` for `serif` with the character in the
  charset, first font that has it);
  fallback files are added to the shell's application-only fontconfig configuration and loaded by their family name.
- macOS: Core Text answers (`CTFontCreateForString`, the URL attribute); the shell creates fonts from the same file.
Without a provider (tests, the C ABI before the Mac shell registers one) missing characters keep the `.notdef` glyph.

### Tables (core)
Ported from Android: column geometry once per table (keyed by the `Arc` pointer), sizes 1, 0.9, 0.8, the
`columnWidths`/`squeezedWidths` rules, cells as ragged paragraphs with hyphenation, header cells bold. A row is an
atomic block; the header row keeps with the next row.

### Background layout (GTK shell)
`Typesetter` becomes `Send` (`Arc` instead of `Rc` for the hyphenator; shaping caches stay per instance). The book is
shared through `Arc`. After a chapter is shown, the shell typesets the previous and next chapters on a worker thread
with its own typesetter and hands the `ChapterLayout` back to the main loop; a turn across the boundary uses the ready
layout. A changed style discards pending results. The macOS shell keeps synchronous layout until `macos-app`.

## Risks / Trade-offs

- Fallback fonts make line breaks platform-dependent for those characters → acceptable; text in the book face (all
  but 2 of 2.6 million characters in the corpus) still breaks identically.
- Large CJK collections: memory-mapped, not read, so registering one costs little.
- Image decoding on the main thread when a page is first drawn may cost a frame on the Pi → the GTK worker decodes the
  images of the current and neighbouring chapters along with their layout; only an image the worker has not reached
  (the very first page) is decoded on the main thread.
