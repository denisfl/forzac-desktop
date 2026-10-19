# Forzac for desktop

An offline reader for FB2, FB3 and EPUB books with book-quality typesetting — hyphenation, justification, no lone lines
at page edges — for Linux (first of all [Omarchy](https://omarchy.org)) and macOS. Keyboard first, few settings,
free. The desktop sibling of Forzac for Android.

**Status: not released yet.** Reading, search, bookmarks, highlights, the library and the interface in English and
Russian work on Linux and macOS; the first release is being prepared ([docs/publishing.md](docs/publishing.md)). The stack
is described in [ADR 0001](docs/adr/0001-stack.md), the spike results in [docs/spike.md](docs/spike.md), the plan to 1.0
in [docs/roadmap.md](docs/roadmap.md).

**Try it in your browser:** [forzac.truenrth.app/read](https://forzac.truenrth.app/read/) opens your own FB2, FB3 or
EPUB with Forzac's typesetting, themes and fonts. The book stays on your computer. It is the web reader of Forzac for
Android, a separate implementation from this app, so some keys and details differ.

Work is planned with [OpenSpec](https://github.com/Fission-AI/OpenSpec): each feature is a change in
`openspec/changes/<name>` (`proposal → specs → design → tasks`), implemented from its tasks and archived when done.
In Claude Code: `/opsx:propose`, `/opsx:apply`, `/opsx:archive`.

## How it is built

```
core/    Rust: model, parsers, hyphenation, shaping (HarfRust), Knuth–Plass, pagination → pages as glyph runs
gtk/     Linux shell: GTK 4 (Rust), pages drawn as GSK text nodes
macos/   macOS shell: AppKit (Swift), pages drawn with Core Text, turned with Core Animation
assets/  bundled fonts (PT Serif, Literata, Source Serif 4, PT Sans, PT Mono for code; OFL) and hyphenation patterns (hyph-utf8) with their licences
data/    .desktop file, MIME type for FB3, icons (made by scripts/icons.py from the Android app's vector icon)
packaging/aur/  AUR packages: forzac (from source), forzac-bin (release archives)
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
the Dock icon. The build is signed ad hoc, without a Developer ID. The download for forzac.truenrth.app is a disk image
with a SHA-256 checksum (`scripts/package-macos.sh X.Y.Z`); a downloaded copy needs a first-open step in macOS,
described for the download page in [docs/macos-first-open.md](docs/macos-first-open.md).

A book reopens where it was left; the settings are remembered. Until a theme is chosen, the theme follows the
system's light or dark appearance. Without a file, Forzac shows the library: the books read, with their covers, the last one to continue first (`Enter`
opens one, `/` searches, `d` removes one from the list; the file stays). The state is kept in
`$XDG_DATA_HOME/forzac/state` on Linux and `~/Library/Application Support/Forzac/state` on macOS, cover thumbnails in
`$XDG_CACHE_HOME/forzac/covers` and `~/Library/Caches/Forzac/covers`.

Tests: `cargo test -p forzac-core`. Measurements on a Linux desktop: `scripts/measure.sh book.fb2.zip`.

## Keys

| Keys | Action |
|---|---|
| `j` `l` `→` `↓` `Space` `PgDn` | next page (Scroll mode: `j` `↓` a line, the others a screen) |
| `k` `h` `←` `↑` `Shift+Space` `PgUp` `Backspace` | previous page (Scroll mode: `k` `↑` a line, the others a screen) |
| `Ctrl+D` `Ctrl+U` | Scroll mode: half a screen down / up |
| `m` | Slide (pages) or Scroll (a continuous column), keeping the top line |
| `Enter` (or a click in the middle of the page) | the menu: back to the books, the section, the contents, a slider over the whole book (`←` `→` `h` `l` move it, `Enter` goes there), "Page N of M", the minutes left in the chapter, "Aa" for the settings; `Esc` closes |
| `]` `[` | next / previous chapter |
| `g` `G` `Home` `End` | chapter start / end |
| `o` | contents (`/` filters, `Enter` jumps, `←` `→` switch to the bookmarks and the highlights; in the highlights `d` deletes, `c` copies one, `Ctrl+C`/`⌘C` all) |
| `b` (macOS also `⌘D`) | bookmark the page (a ribbon marks it); again to remove the bookmark |
| `B` | bookmarks (`Enter` goes, `d` removes) |
| `v`, then `h` `l` `j` `k`, `v`, `o` | select text from the keyboard: a word cursor, `v` anchors, `o` moves the other end; `Enter` highlight, `c` (or `Ctrl+C`, macOS `⌘C`) copy, macOS `s` share, `Esc` back |
| `/` (macOS also `⌘F`) | search in the book: case and ё/е do not matter; `Enter` goes to a match |
| `n` `N` (macOS also `⌘G` `⇧⌘G`) | next / previous match; `Esc` clears the highlight |
| `Tab` `Shift+Tab`, then `Enter` | note markers, links, pictures and tables on the page (or click them) |
| `+` `−` `0`, arrows `h` `j` `k` `l` `Space`, `Esc` `q` | in a picture or table opened full window: zoom, fit, move, close (also the wheel, the touchpad, a drag, a pinch, `Ctrl`+wheel; macOS `⌘`+wheel) |
| `Ctrl+O` (macOS also `⌘[`) | back after a jump (the "← Back" pill at the foot of the page does the same) |
| `s` | settings: theme, font, text size, line spacing, alignment, margins, pages (`j` `k` choose, `h` `l` change; macOS also `⌘,`) |
| `+` `−` `0` | text size, the default size |
| `t` `T` | theme |
| `f` `F11` (macOS: `⌃⌘F`) | full screen |
| `?` | the keys (`Esc` closes) |
| `q` `Ctrl+W` (macOS: `⌘W`) | close |

A wheel step turns a page; one touchpad swipe turns one page; a click in the left or right third of the page turns it
(a click on a picture at least a third of the column wide or on a table opens it full window instead; a click in the
top right corner above the text sets or removes a bookmark); a drag selects text by words (in Slide a double click in
the middle third takes a word, a triple click the paragraph), a click on a highlight offers Delete and Copy;
in Scroll mode the wheel and touchpad scroll smoothly. A link to a short passage (a figure, a table, a paragraph)
opens it over the page, like a note. After the last page comes an end page that marks the book as finished.
Keys work in any keyboard layout. The interface is in English or Russian, as the system prefers (on Linux `LANGUAGE`, `LC_ALL`,
`LC_MESSAGES`, `LANG`; on macOS the preferred languages).

## Install on Linux

Nothing is published yet. Arch and Omarchy: the AUR packages `forzac` (builds from source) and `forzac-bin` (the
release build), in `packaging/aur/`; Omarchy is checked in an Arch x86_64 container only, not on hardware. Raspberry Pi
OS 13 (arm64): `sudo apt install ./forzac-linux-arm64.deb`, made by `scripts/package-deb.sh` in Debian 13
(`ci/debian.Dockerfile`; the release workflow builds it). Forzac runs as one process: books opened while it runs open
in new windows of that process.

The release archives and the macOS app look for a newer release on GitHub at most once a day, when the library
opens, and offer it in one line there. Packages leave updates to their package manager: a build whose
`share/forzac` holds a file named `packaged` (the AUR packages and the `.deb` install it) never asks.

## Default reader on Linux

Outside GNOME and KDE (Hyprland, labwc, sway), `xdg-open` needs File::MimeInfo (`perl-file-mimeinfo`,
`libfile-mimeinfo-perl`) to type books by name; the packages depend on it and print this command after installing:

```bash
xdg-mime default app.truenrth.forzac.desktop application/epub+zip application/x-fictionbook+xml application/x-zip-compressed-fb2 application/x-fb3
```

## Licences

The code is licensed under the [Mozilla Public License 2.0](LICENSE). Bundled: PT Serif, Literata, Source Serif 4 and PT Sans — SIL Open Font License 1.1 (`OFL.txt` in each `assets/fonts/` folder); Russian hyphenation patterns — LPPL 1.2 or later, English — free use
with the notice kept (`assets/hyph/*.lic.txt`).
