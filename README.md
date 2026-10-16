# Forzac for desktop

An offline reader for FB2 and EPUB books with book-quality typesetting — hyphenation, justification, no lone lines
at page edges — for Linux (first of all [Omarchy](https://omarchy.org)) and macOS. Keyboard first, few settings,
free. The desktop sibling of Forzac for Android.

**Status: spike.** The stack is described in [ADR 0001](docs/adr/0001-stack.md), the results in
[docs/spike.md](docs/spike.md).

## How it is built

```
core/    Rust: model, parsers, hyphenation, shaping (HarfRust), Knuth–Plass, pagination → pages as glyph runs
gtk/     Linux shell: GTK 4 (Rust), pages drawn as GSK text nodes
macos/   macOS shell: AppKit (Swift), pages drawn with Core Text, turned with Core Animation
assets/  bundled fonts (PT Serif, OFL) and hyphenation patterns (hyph-utf8) with their licences
data/    .desktop file, MIME type for FB3, AUR PKGBUILD sketch
```

## Build and run

Linux (GTK 4.14 or newer, fontconfig):

```bash
cargo run --release -p forzac-gtk -- book.fb2.zip
```

macOS (Xcode command line tools, Rust with the `aarch64-apple-darwin` target):

```bash
macos/build.sh && macos/build/Forzac book.fb2.zip
```

Tests: `cargo test -p forzac-core`. Measurements on a Linux desktop: `scripts/measure.sh book.fb2.zip`.

## Keys

| Keys | Action |
|---|---|
| `j` `l` `→` `↓` `Space` `PgDn` `Enter` | next page |
| `k` `h` `←` `↑` `Shift+Space` `PgUp` `Backspace` | previous page |
| `]` `[` | next / previous chapter |
| `g` `G` `Home` `End` | chapter start / end |
| `+` `−` | text size |
| `t` `T` | theme |
| `f` `F11` (macOS: `⌃⌘F`) | full screen |
| `?` | help |
| `q` `Ctrl+W` (macOS: `⌘W`) | close |

Planned: `o` contents, `/` `n` `N` search, `b` bookmark, `m` page / scroll mode, `s` settings.

## Default reader on Linux

Outside GNOME and KDE (Hyprland, sway), `xdg-open` needs `perl-file-mimeinfo` to type books by name; the package
depends on it. Then:

```bash
xdg-mime default app.truenrth.forzac.desktop application/epub+zip application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3
```

## Licences

The licence of the code is to be decided before the first public release. Bundled: PT Serif — SIL Open Font
License 1.1 (`assets/fonts/pt-serif/OFL.txt`); Russian hyphenation patterns — LPPL 1.2 or later, English — free use
with the notice kept (`assets/hyph/*.lic.txt`).
