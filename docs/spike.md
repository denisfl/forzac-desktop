# Spike: native desktop shells over a Rust core

October 2026. Goal: check the stack chosen in [ADR 0001](adr/0001-stack.md) — a Rust typesetting core, a GTK 4 shell
for Linux and an AppKit shell for macOS — against the priorities: speed, text and animation quality, battery life.

## What exists

- `core/` — FB2 / `.fb2.zip` parser (quick-xml, encoding from the XML declaration), typographic clean-up (no-break
  spaces after short Russian words and before em dashes), Liang hyphenation on hyph-utf8 patterns (`ru`, `en-us`),
  shaping with HarfRust over the bundled PT Serif, Knuth–Plass line breaking, pagination with the Android app's rules,
  pages as positioned glyph runs, a C ABI for Swift. 9 unit tests.
- `gtk/` — a window per book, pages as `GskTextNode`s cached per page, a 200 ms slide on the frame clock, keyboard
  shortcuts, six themes, text size presets, help on `?` with the single "support" line. Spike flags: `--measure`,
  `--png`, `--turns N`, `--chapter N`.
- `macos/` — the same reader in AppKit: Core Text draws the glyph runs into a bitmap at the backing scale, Core
  Animation slides the pages, keys by physical key code, standard menu (⌘Q, ⌘W, ⌃⌘F). Flags `--measure`, `--png`,
  `--chapter N`.
- `ci/` — a Fedora container (GTK 4.22, aarch64) that builds the GTK shell and runs it under a headless Weston.
- `scripts/measure.sh` — the same measurements on a real Linux desktop.
- `data/` — `.desktop` file, a MIME type for FB3, a sketch of the AUR `PKGBUILD`.

Not in the spike: EPUB and FB3, images, tables, formulas, footnotes and links, selection, search, contents, the
scroll mode, persistence of the reading position, glyph fallback for characters PT Serif lacks.

## Results

Book: Vonnegut, *Cat's Cradle*, `.fb2.zip`, 128 chapters (and three more books for the core numbers).

### Core (macOS, Apple Silicon, release build)

| | |
|---|---|
| Parse a book | 5–26 ms |
| Load Russian hyphenation patterns | 1–2 ms |
| Typeset a whole book (warm caches) | 118–348 ms for 173–461 pages, about 0.6 ms per page |
| Glyph runs of one page | 3–5 µs |

A chapter is typeset in a few milliseconds, so a re-layout after a change of size or window width needs no
background thread.

### macOS shell (Apple Silicon, Retina)

| | |
|---|---|
| Launch to the first presented page | ≈230 ms warm (≈430 ms first run); most of it is AppKit launching before `applicationDidFinishLaunching` |
| CPU while idle, 10 s | 0.03 s |
| Memory (RSS) | 26 MB |
| Binary | 6.2 MB with fonts and hyphenation patterns |

Page turns run in the window server (Core Animation): the app does no work per frame.

### GTK shell (container: Fedora, GTK 4.22, aarch64, headless Weston, software rendering)

| | |
|---|---|
| Start to the first presented page | 125–170 ms warm (1.1–1.8 s first run: shader and font caches) |
| CPU while idle, 10 s | 0 ticks |
| Page turns at 60 Hz (software GL) | p50 17 ms; missed frames are expected on llvmpipe and say nothing about hardware |
| Memory (RSS) | 250 MB with Mesa llvmpipe; to be measured with a real GPU |

The rendered pages (`--png`) were checked by eye at scale 1 and 2: justified lines, hyphens drawn, glyph positions
identical to the macOS shell (the core breaks lines the same way on both).

### Found along the way

- `PangoGlyphString::set_size` leaves new entries uninitialised; a stray `is_color` bit made GSK draw some glyphs as
  colour glyphs, i.e. invisible. The shell now sets the visual attributes explicitly (`gtk/src/page_view.rs`).
- GTK and the core must use the very same font files. The shell gives Pango a fontconfig configuration with only the
  bundled faces; a system PT Serif of another version would have other glyph ids.
- Ragged-right text (verse, headings) uses Knuth's ragged-right glue, so it gets optimal breaks too rather than the
  first-fit fallback.

## Not verified yet — needs hardware

Run `scripts/measure.sh book.fb2.zip` and send the output:

1. **Omarchy laptop** at its native refresh rate (ideally 120 Hz) and fractional scale (1.25 / 1.6): startup, turn
   frames (`missed` should be 0), idle CPU, RSS; the look of text at that scale.
2. **Raspberry Pi 5**: the same with `GSK_RENDERER=ngl` and `GSK_RENDERER=vulkan`, to choose the default.
3. **Russian keyboard layout** on Hyprland: `j`/`k` should still turn pages (GTK matches shortcuts in the first
   layout group too).
4. **macOS**: page turn smoothness on a ProMotion display; keys in a Russian layout (they use key codes).

## Next steps

1. Measurements above; pick the GTK renderer for the Pi if the default misbehaves.
2. EPUB and FB3 in the core (port from the Android app), images, footnotes, glyph fallback.
3. Reading position and recent books (SQLite in the core), contents (`o`), search (`/`), the scroll mode.
4. Packaging: AUR, a notarised macOS build, a release CI for x86_64 and aarch64.
5. Licence choice before the first public commit.
