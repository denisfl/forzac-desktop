# CLAUDE.md

Forzac for desktop: an offline FB2/FB3/EPUB reader with book-quality typesetting for Linux (Omarchy first, Raspberry
Pi 5) and macOS. Open source (MPL-2.0), free with one unobtrusive "support" link. The sibling of Forzac for Android;
separate code, shared principles. Everything in this repository is written in English, including commits and OpenSpec
artefacts; talk to the user in their language.

## Product principle

Few settings, great defaults; presets instead of sliders. Keyboard first (Vim keys, arrows, space). Two page modes
only: "Slide" and "Scroll". No PDF. Typography of a good printed book. Smoothness over features: no missed frames
inside an animation at 120 Hz, nothing drawn while idle.

## Architecture

[docs/adr/0001-stack.md](docs/adr/0001-stack.md). The Rust core (`core/`) parses, hyphenates, shapes (HarfRust over the
bundled fonts), breaks lines (Knuth–Plass), paginates and emits pages as positioned glyph runs. Shells only draw and
handle input: GTK 4 in Rust (`gtk/`), AppKit in Swift (`macos/`, through `core/src/ffi.rs` and `macos/forzac.h`).

Invariants:
- The reading position is a `Locator` (chapter, block, offset), never a page number.
- The core has no UI or platform code and is tested on its own.
- Shells rasterise exactly the font files the core shaped with (GTK: a fontconfig configuration with only the bundled
  files; macOS: `CGFont` from the core's bytes).
- Theme change is a redraw; font, size, spacing, alignment, margins re-typeset and keep the `Locator`.

## Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings      # needs GTK 4 dev files; on macOS: -p forzac-core
cargo test -p forzac-core
FORZAC_CORPUS=~/Books cargo test -p forzac-core --test corpus
cargo run --release -p forzac-gtk -- book.fb2.zip          # Linux
macos/build.sh                                             # macOS: macos/build/Forzac and Forzac.app (signed ad hoc)
scripts/package-macos.sh X.Y.Z                             # macOS: the DMG and its .sha256, after macos/build.sh
cargo run --release -p forzac-core --example bench -- book.fb2.zip
cargo run --release -p forzac-core --example mobi -- book.fb2.zip out.mobi   # the Kindle's MOBI; --kindle sends it
scripts/measure.sh book.fb2.zip                            # Linux desktop or Mac measurements (performance budgets)
scripts/pi-headless.sh target/release/forzac book.fb2.zip  # headless measurements on GPU (Raspberry Pi)
scripts/package-linux.sh forzac-X.Y.Z-linux-ARCH           # release archive layout
scripts/package-deb.sh X.Y.Z                               # Raspberry Pi OS .deb (in ci/debian.Dockerfile)
scripts/icons.py                                           # icons from ../forzac's vector icon (needs rsvg-convert)
PI=user@host scripts/pi-dev.sh book.fb2.zip [flags]        # build for aarch64, run on the Pi's desktop
PI=user@host scripts/pi-vnc.sh                             # watch the Pi's desktop from macOS
```

GTK shell flags for measurements: `--measure`, `--png out.png`, `--turns N`, `--chapter N`.
Containers in `ci/` build and run the GTK shell on Fedora, Arch (makepkg) and Debian/Raspberry Pi OS.
Books are never committed; test books live outside the repository.
The development Linux machine is a Raspberry Pi 5 without a monitor, watched over Raspberry Pi Connect in a browser or VNC (`scripts/pi-vnc.sh`).

## Process: OpenSpec

Every noticeable feature is a change in `openspec/changes/`: `proposal → specs → design → tasks`, implemented by its
task list, archived with `openspec archive` when done (`/opsx:propose`, `/opsx:apply`, `/opsx:archive`). Order of work:
[docs/roadmap.md](docs/roadmap.md). Small fixes go without a change.

## Working rules

- **Think before coding.** State assumptions; if something is unclear, ask; if there are several readings, show them.
- **Simplicity first.** The least code that solves the task; no speculative flexibility; no settings nobody asked for.
- **Surgical changes.** Touch only what the task needs; match the surrounding style; remove only what your change made
  unused.
- **Goal-driven.** Define how success is verified: a failing test for a bug, a measurement for performance, a
  screenshot for UI. Performance and UI are verified on hardware (the Pi 5 and the M1 MacBook Pro; there is no Omarchy machine — see
  `docs/roadmap.md`), not in containers.
