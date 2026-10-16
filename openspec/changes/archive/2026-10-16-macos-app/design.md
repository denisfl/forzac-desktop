## Context

The Mac shell is one Swift file over a small C ABI (`core/src/ffi.rs`, `macos/forzac.h`): open, paginate, render a
page, positions and settings as strings. Pages are drawn with Core Text into bitmaps and slid by Core Animation.

## Decisions

**C ABI, extended.** About twenty more functions: settings as a struct (load, save, step, the page style follows from
them), font variations, the contents, the cover, authors and progress, notes, links, recent books, and the chapter's
continuous column for "Scroll". UniFFI would add a generator and a Swift package for no gain at this size.

**Shared logic in the core.** The contents fallback (chapters when the book has no contents) and link resolution move
into the core and are used by both shells.

**Fonts.** The bundled PT Serif stays embedded. The other families are read from the assets directory (`.app`:
`Contents/Resources`, development: the source tree) and drawn from the same files at the same variation-axis values
the core shaped with (`CTFontDescriptorCreateCopyWithVariation`). Golos Text and the book families are registered for
the process (`CTFontManagerRegisterFontsForURL`) when the first panel is built.

**Panels.** AppKit views drawn after the Android design with the same sizes and palette as the GTK panels: a bottom
sheet for settings and notes, a side panel for the contents. Layers give the rounded corners and shadows.

**Appearance.** Light or Dark follows `NSApp.effectiveAppearance` (observed) until the reader picks a theme.

**Scroll.** The chapter's continuous column (`flow`) is drawn in tiles a screen high into Core Animation layers; the
layers move, nothing is drawn per frame. Chapters before and after are laid out once the current one shows.

**Files.** `Info.plist` declares the document types; `application(_:open:)` opens each book in its own window; started
without a book, a window lists the recent books.

## Risks / Trade-offs

- Layout runs on the main thread as before (a chapter takes milliseconds on Apple Silicon); moving it off the main
  thread is left for when a measurement asks for it.
- Unsigned builds: macOS asks once on first open; signing comes with `macos-release`.
