## Context

Most of the shell side is already there: `gtk/src/main.rs` sets the program name (the Wayland app id) to
`app.truenrth.forzac`, runs as a `GApplication` with `HANDLES_OPEN` (one process, one window per book), drops the
title bar on Hyprland, and shows the help with the support line on `?`. `data/` has the desktop entry, the FB3 MIME
type and an untested PKGBUILD sketch; `scripts/package-linux.sh` lays out the release archive. What is missing is the
icon, the packages themselves, the post-install hint, and checks that the whole chain works: on the Pi 5's labwc
desktop and in the Arch x86_64 container (`ci/arch.Dockerfile`).

The Pi's desktop session sets no `XDG_CURRENT_DESKTOP`, so `xdg-open` runs its generic path there, as on Hyprland:
`xdg-mime query filetype` uses `mimetype` (File::MimeInfo) when present and `file --mime-type` (content: `.fb2` is
`text/xml`) otherwise. Raspberry Pi OS registers `xarchiver` for `application/epub+zip`, so Forzac is not the EPUB
default by itself.

## Goals / Non-Goals

**Goals:** `forzac` and `forzac-bin` for the AUR and a `.deb` for Raspberry Pi OS from one staged tree; the icon from
the Android app's vector source; checks on the Pi 5 and in the Arch container.

**Non-Goals:** Flatpak, Snap, an x86_64 `.deb`, an apt repository, publishing to the AUR (a separate decision at
release time), Omarchy theme colours.

## Decisions

**The icon is generated from the Android vector drawables and committed.** `scripts/icons.py` reads
`ic_launcher_background.xml` and `ic_launcher_foreground.xml` (plain `M`/`L`/`H`/`V`/`Z` paths, fills and strokes
with alpha, two horizontal linear gradients) and writes them path for path as SVG: colours `#AARRGGBB` become a colour
and an opacity, gradients become `linearGradient` in user space. Like the store and site icons it shows the central
72 of the 108 dp an Android launcher shows. On Linux that square is clipped to a rounded square on a 128 px grid
(6 px margin, 20 px corners): the hicolor `scalable/apps/app.truenrth.forzac.svg` is that SVG, and `rsvg-convert`
renders the PNGs. The same script writes `macos/Forzac.icns` (`macos-release`). The outputs are committed (the
`.gitignore` excludes PNGs, so `data/icons/` is an exception): packages and CI need no librsvg and no checkout of the
Android app.

Alternative considered: rendering with PIL as the Android repository's `store/graphics/make.py` does. It strokes
with round joints and fills without anti-aliasing beyond supersampling, and it gives no SVG.

**One staged tree, three packages.** `scripts/package-linux.sh` stays the single layout (`bin/`, `share/forzac/`,
`share/applications/`, `share/mime/packages/`, `share/icons/hicolor/`, `share/licenses/forzac/`) and gains the icons.
`forzac-bin` copies an unpacked release archive to `/usr` (`options=(!debug)`: makepkg's debug split has nothing to
extract from a prebuilt binary, and its `-debug` package would clash with `forzac-debug`); `scripts/package-deb.sh`
stages the same tree under `/usr` and wraps it with `dpkg-deb --root-owner-group`. The source PKGBUILD installs the
same files from the source tree.

**AUR files live in `packaging/aur/<package>/`**, one directory per AUR repository (`PKGBUILD` and `forzac.install`),
replacing the `data/PKGBUILD` sketch. The source URL is `github.com/denisfl/forzac-desktop` (the repository is not
public yet). `.SRCINFO` and checksums are produced at publication (`updpkgsums`, `makepkg --printsrcinfo`), when the
tag's archives exist; the committed files carry `SKIP` until then. The container check replaces the sources with
local archives.

**The `.deb` is built in Debian 13 for arm64**, in `ci/debian.Dockerfile` (the image `scripts/pi-dev.sh` already
uses), so it links the GTK and glibc of Raspberry Pi OS 13. `Depends: libgtk-4-1 (>= 4.14), libfontconfig1,
libfile-mimeinfo-perl` written by hand: GTK pulls in GLib, Pango and Cairo, and the binary links nothing else.
The caches need no maintainer script: dpkg triggers of shared-mime-info, desktop-file-utils and the icon cache handle
`/usr/share/mime/packages`, `/usr/share/applications` and `/usr/share/icons/hicolor`. The release workflow gains a job
on `ubuntu-24.04-arm` that builds the image and the package.

**The hint, once.** pacman's `post_install` (not `post_upgrade`) and the `.deb`'s `postinst` on `configure` without a
previous version print the `xdg-mime default` command for the four types. Setting the default for the user from a
package is not possible (it is per user) and would be rude.

**The window icon by name.** `gtk::Window::set_default_icon_name("app.truenrth.forzac")`: on Wayland the compositor
takes the icon from the desktop entry by app id; under X11 GTK sets the icon from the theme by this name.

**The default size fits the monitor.** The book window's 760 × 980 does not fit the Pi Connect desktop (1280 × 673)
or a 768 px laptop screen: the bottom of the page and the status line were off the screen. The default size is
clamped to 85% of the first monitor's logical size (`GdkDisplay::monitors`): before the window is mapped GTK cannot
tell which monitor it will be on, and a single monitor is the common case. The measurement outputs (1920 × 1200) are
large enough to keep 760 × 980.

**Already in place, now specified.** One process per user (`HANDLES_OPEN`; measurement flags add `NON_UNIQUE`), no
title bar on Hyprland (`HYPRLAND_INSTANCE_SIGNATURE`), the help with the support line.

## Risks / Trade-offs

- Hyprland is not checked (no hardware; it does not run in containers) → the README keeps saying Omarchy is built and
  installed in CI only.
- A hand-written `Depends` can miss a library if the shell starts linking one → `dpkg-shlibdeps` is not used to keep
  the build a single `dpkg-deb` call; installing on the Pi with `apt install ./…deb` is the check.
- The `.deb` and `forzac-bin` repeat the archive layout → both are made from `scripts/package-linux.sh`'s output, not
  from a second list of files.
