# ADR 0001: Rust core with native shells — GTK 4 on Linux, AppKit on macOS

Status: accepted for the spike (2026-10-16). Revisit after measurements on Omarchy and a Raspberry Pi 5.

## Context

Forzac is an offline FB2/EPUB reader with book-quality typesetting (hyphenation, justification, widow and orphan
control). The Android app has its own engine on `StaticLayout`. The desktop version targets:

- Linux first: Omarchy (Arch + Hyprland, Wayland), keyboard-first users; also a Raspberry Pi 5 (aarch64,
  VideoCore VII, Mesa v3d: OpenGL ES 3.1, Vulkan).
- macOS.

Priorities, in order: speed, quality of text and animation, battery life on laptops, performance. The code may differ
from Android's. The project is a separate, likely open-source repository; it is free, and good enough that people
want to support it. No App Store distribution is planned.

A first spike on Compose Multiplatform Desktop (JVM) was rejected: 0.4–0.8 s to the first page even with AppCDS,
hundreds of MB of memory, XWayland instead of native Wayland, and Skia Paragraph breaks at soft hyphens without
drawing the hyphen.

## Options

| | Linux (Wayland, Pi 5) | macOS | Main risk |
|---|---|---|---|
| Rust + GTK 4 everywhere | native Wayland, fractional scaling, GPU rendering (GSK), idle = no frames | no Metal renderer (deprecated OpenGL or software Cairo), no native menus or gestures; no notable GTK 4 app ships on macOS | macOS is second-class |
| Qt 6 (C++/QML) | good: fractional scaling on Wayland, Omarchy already ships `qt6-base`/`qt6-wayland` | good: Metal through RHI, Core Text rasterisation | drawing our own glyph runs means `QRawFont`/`drawGlyphRun` or a custom scene-graph node; C++ for parsing untrusted files |
| One Rust UI (winit + wgpu, GPUI, iced) | good | average | we build text rasterisation (young hinting in the Rust stack), ProMotion frame pacing, accessibility, menus; GPUI is not maintained outside Zed |
| **Rust core + GTK 4 shell + AppKit shell** | native | native | two thin shells to maintain |

## Decision

A Rust core (`core/`): the book model, parsers, hyphenation, shaping with HarfRust (a Rust port of HarfBuzz) over the
bundled OFL fonts, Knuth–Plass line breaking, pagination. Its output is a page as **positioned glyph runs** (face, size,
glyph ids, positions). Shells only draw them:

- Linux (`gtk/`): GTK 4 in Rust. Runs become `GskTextNode`s; each page is cached as a render node; page turns animate
  on the compositor's frame clock. Fonts are loaded through a fontconfig configuration that holds only the bundled
  files, so the glyph ids the core produced are the ones drawn.
- macOS (`macos/`): Swift + AppKit. Runs are drawn with Core Text (`CTFontDrawGlyphs`) from the same font bytes into a
  bitmap at the backing scale; page turns are Core Animation, which runs in the window server.

Because shaping happens in the core with the same font files, a page breaks identically on Linux and macOS.

## Consequences

- One typesetting engine, two small shells. The surface of a reader is small (page, contents, search, preset
  settings, help), so the cost of two shells is bounded.
- Rust makes parsing untrusted book files memory-safe.
- Text rasterisation is native on each platform: FreeType through GSK on Linux (greyscale anti-aliasing only; GTK 4
  has no subpixel AA), Core Text on macOS.
- Risks to check on hardware: GTK's renderer choice on the Pi (regressions after the switch to Vulkan by default were
  reported on the Pi 4), blur at fractional scales reported in some GTK releases, glyph fallback for characters the
  bundled faces lack (not in the spike).
- The C ABI (`core/src/ffi.rs`, `macos/forzac.h`) is hand-written for the spike; UniFFI is an option once it grows.

## Sources

- GTK renderers and fractional scaling: <https://blog.gtk.org/2024/01/28/new-renderers-for-gtk/>,
  <https://blog.gtk.org/2024/03/07/on-fractional-scales-fonts-and-hinting/>
- GTK 4 on Raspberry Pi: <https://gitlab.gnome.org/GNOME/gtk/-/issues/6726>, <https://bugzilla.redhat.com/show_bug.cgi?id=2282171>
- GPUI outside Zed: <https://github.com/zed-industries/zed/discussions/30515>
- Qt Wayland fractional scaling: <https://codebrowser.dev/qt6/qtbase/src/plugins/platforms/wayland/qwaylandfractionalscale.cpp.html>
- HarfRust: <https://github.com/harfbuzz/harfrust>
