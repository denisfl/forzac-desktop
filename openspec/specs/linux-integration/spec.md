# linux-integration Specification

## Purpose
How Forzac is packaged for Linux (AUR, Raspberry Pi OS) and how it fits the desktop: the desktop entry, MIME types,
icon, default-reader behaviour, one process per user and window sizes.

## Requirements

### Requirement: Packages
Forzac SHALL be installable on Linux from three packages: the AUR package `forzac`, built from the source of a tagged
release; the AUR package `forzac-bin`, made from the release archives for x86_64 and aarch64 (it provides and
conflicts with `forzac`); and a Debian package for Raspberry Pi OS (Debian 13, arm64) built by CI. Each SHALL install
the `forzac` binary to `/usr/bin`, the bundled fonts to `/usr/share/forzac/fonts`, the desktop entry, the FB3 MIME
type, the icons and the licence texts of the code and of every bundled asset. Each SHALL depend on GTK 4 (4.14 or
newer), fontconfig and File::MimeInfo (`perl-file-mimeinfo` on Arch, `libfile-mimeinfo-perl` on Debian), so that
`xdg-open` types books by name outside GNOME and KDE.

#### Scenario: Installing from the AUR source package
- **WHEN** `makepkg -si` is run in the `forzac` package directory on Arch x86_64
- **THEN** the package builds, installs with its dependencies, and `forzac book.fb2` opens the book with the bundled
  fonts

#### Scenario: Installing the Debian package on a Raspberry Pi 5
- **WHEN** `sudo apt install ./forzac_X.Y.Z_arm64.deb` is run on Raspberry Pi OS
- **THEN** the dependencies are installed with it, and Forzac appears in the desktop menu under Office with its icon

### Requirement: Desktop entry and MIME types
The packages SHALL install the desktop entry `app.truenrth.forzac.desktop`, which passes `desktop-file-validate`,
names Forzac in English and Russian, and declares `application/x-fictionbook+xml`,
`application/x-zip-compressed-fb2`, `application/x-fb3`, `application/epub+zip` and `text/markdown` — not `text/plain`,
which would offer a book reader for every log and configuration file. FB3, which shared-mime-info lacks,
SHALL be declared by `/usr/share/mime/packages/forzac.xml` (the `*.fb3` glob, a subclass of `application/zip`). The
Wayland app id of every Forzac window SHALL be `app.truenrth.forzac`, the desktop entry's name, so that taskbars and
docks tie the windows to the entry and its icon.

#### Scenario: A taskbar shows the icon
- **WHEN** a book is open in Forzac on the Pi 5's labwc desktop
- **THEN** the taskbar button of its window shows the Forzac icon, not a generic one

#### Scenario: An FB3 file is typed
- **WHEN** `xdg-mime query filetype book.fb3` is run after installing a package
- **THEN** it reports `application/x-fb3`

### Requirement: Icon
The application icon SHALL be the Android app's launcher icon (the marbled endpaper and the page edge) as a rounded
square, installed to the hicolor theme as `app.truenrth.forzac` in sizes 16, 22, 24, 32, 48, 64, 128, 256 and 512
pixels and as a scalable SVG, and the Android app's monochrome icon (an open book in strokes) as the symbolic icon
`app.truenrth.forzac-symbolic` in `hicolor/symbolic/apps`, which GNOME draws in the foreground colour. The icons SHALL
be generated from the Android app's vector drawables by a script in the repository, and the generated files SHALL be
committed, so that packaging needs no image tools.

#### Scenario: Regenerating the icons
- **WHEN** `scripts/icons.py` is run next to a checkout of the Android app
- **THEN** the committed hicolor icons and the macOS icon are rewritten from the vector drawables, unchanged if the
  drawables did not change

### Requirement: Default reader
After `xdg-mime default app.truenrth.forzac.desktop` for the four MIME types, `xdg-open` SHALL open `.fb2`,
`.fb2.zip`, `.fb3` and `.epub` files in Forzac on desktops other than GNOME and KDE (Hyprland, labwc, sway), typing the
files by name. Installing a package SHALL print that command once, after the first installation, and the README SHALL
give it.

#### Scenario: xdg-open on the Pi 5
- **WHEN** Forzac is the default for FB2 and the reader runs `xdg-open book.fb2` in the labwc session
- **THEN** the book opens in Forzac, not in a text editor or a browser

#### Scenario: The default is reported
- **WHEN** the hint's command has been run on Arch
- **THEN** `xdg-mime query default application/x-fictionbook+xml` reports `app.truenrth.forzac.desktop`

### Requirement: One process per user
Forzac SHALL run as one process per user session: starting `forzac` with books while it runs SHALL hand the books to
the running process, which opens each in a window of its own (or brings an open book's window forward), and the
second process exits at once. The measurement flags (`--measure`, `--png`, `--turns`, `--scrolls`) SHALL run in a
process of their own.

#### Scenario: A second book from the file manager
- **WHEN** a book is open and the reader opens another book from the file manager
- **THEN** a second window appears, and there is still one `forzac` process

### Requirement: Window size
On Hyprland, Forzac windows SHALL have no title bar drawn by the system or by the toolkit, since the compositor draws
none and arranges the windows; the library's header (`library`, `The head on Linux`) SHALL be the top row of its window
there. On floating compositors the library's header SHALL be its window's title bar with the window buttons, and a book
window keeps the toolkit's title bar. On floating compositors a book window SHALL open at 760 × 980 logical pixels and
the library at 1040 × 760, each no larger than 85% of the monitor's width and height, so that a window fits a small
screen whole. The library SHALL work in windows down to 360 × 294 logical pixels (GNOME's smallest adaptive size),
which a tiling compositor may give it.

#### Scenario: Hyprland
- **WHEN** Forzac starts in a Hyprland session
- **THEN** its windows have no title bar, fill the tile Hyprland gives them, and the library's header is its top row

#### Scenario: A small screen
- **WHEN** a book opens on the Pi 5's 1280 × 673 Raspberry Pi Connect desktop
- **THEN** the whole window, title bar included, is on the screen

#### Scenario: A floating library
- **WHEN** the library opens on GNOME
- **THEN** its title bar is the library's header — "+", search, the switcher, the main menu and the window buttons in
  one row
