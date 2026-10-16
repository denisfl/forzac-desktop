# Forzac for desktop

An offline reader for FB2, FB3 and EPUB books with book-quality typesetting — hyphenation, justification, no lone lines
at page edges — for Linux (first of all [Omarchy](https://omarchy.org)) and macOS. Keyboard first, few settings,
free. The desktop sibling of Forzac for Android.

**Status: spike done, planning the full version.** The stack is described in [ADR 0001](docs/adr/0001-stack.md),
the spike results in [docs/spike.md](docs/spike.md), the plan to 1.0 in [docs/roadmap.md](docs/roadmap.md).

Work is planned with [OpenSpec](https://github.com/Fission-AI/OpenSpec): each feature is a change in
`openspec/changes/<name>` (`proposal → specs → design → tasks`), implemented from its tasks and archived when done.
In Claude Code: `/opsx:propose`, `/opsx:apply`, `/opsx:archive`.

## How it is built

```
core/    Rust: model, parsers, hyphenation, shaping (HarfRust), Knuth–Plass, pagination → pages as glyph runs
gtk/     Linux shell: GTK 4 (Rust), pages drawn as GSK text nodes
macos/   macOS shell: AppKit (Swift), pages drawn with Core Text, turned with Core Animation
assets/  bundled fonts (PT Serif, Literata, Source Serif 4, PT Sans; OFL) and hyphenation patterns (hyph-utf8) with their licences
data/    .desktop file, MIME type for FB3, AUR PKGBUILD sketch
```

## Build and run

Linux (GTK 4.14 or newer, fontconfig):

```bash
cargo run --release -p forzac-gtk -- book.fb2.zip
```

macOS (Xcode command line tools, Rust with the `aarch64-apple-darwin` target):

```bash
macos/build.sh && open macos/build/Forzac.app
```

`Forzac.app` opens books from Finder ("Open With", or set it as the default app for EPUB and FB2), from `⌘O` and from
the Dock icon. The build is unsigned: macOS asks once on first open (right-click › Open).

A book reopens where it was left; the settings are remembered. Until a theme is chosen, the theme follows the
system's light or dark appearance. Without a file, Forzac lists the recent books (`Enter` opens one). The state is kept in `$XDG_DATA_HOME/forzac/state` on Linux and
`~/Library/Application Support/Forzac/state` on macOS.

Tests: `cargo test -p forzac-core`. Measurements on a Linux desktop: `scripts/measure.sh book.fb2.zip`.

## Keys

| Keys | Action |
|---|---|
| `j` `l` `→` `↓` `Space` `PgDn` `Enter` | next page (Scroll mode: `j` `↓` a line, the others a screen) |
| `k` `h` `←` `↑` `Shift+Space` `PgUp` `Backspace` | previous page (Scroll mode: `k` `↑` a line, the others a screen) |
| `Ctrl+D` `Ctrl+U` | Scroll mode: half a screen down / up |
| `m` | Slide (pages) or Scroll (a continuous column), keeping the top line |
| `]` `[` | next / previous chapter |
| `g` `G` `Home` `End` | chapter start / end |
| `o` | contents (`/` filters, `Enter` jumps) |
| `Tab` `Shift+Tab`, then `Enter` | note markers and links on the page (or click them) |
| `Ctrl+O` (macOS also `⌘[`) | back after a jump |
| `s` | settings: theme, font, text size, line spacing, alignment, margins, pages (`j` `k` choose, `h` `l` change; macOS also `⌘,`) |
| `+` `−` `0` | text size, the default size |
| `t` `T` | theme |
| `f` `F11` (macOS: `⌃⌘F`) | full screen |
| `?` | help |
| `q` `Ctrl+W` (macOS: `⌘W`) | close |

A wheel step turns a page; one touchpad swipe turns one page; in Scroll mode the wheel and touchpad scroll smoothly.
Keys work in any keyboard layout. Planned: `/` `n` `N` search, `b` bookmark.

## Default reader on Linux

Outside GNOME and KDE (Hyprland, sway), `xdg-open` needs `perl-file-mimeinfo` to type books by name; the package
depends on it. Then:

```bash
xdg-mime default app.truenrth.forzac.desktop application/epub+zip application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3
```

## Licences

The code is licensed under the [Mozilla Public License 2.0](LICENSE). Bundled: PT Serif, Literata, Source Serif 4 and PT Sans — SIL Open Font License 1.1 (`OFL.txt` in each `assets/fonts/` folder); Russian hyphenation patterns — LPPL 1.2 or later, English — free use
with the notice kept (`assets/hyph/*.lic.txt`).
