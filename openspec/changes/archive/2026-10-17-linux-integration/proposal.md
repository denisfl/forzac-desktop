## Why

On Omarchy a reader should install with one command, open books from the file manager and from `xdg-open`, and be the
default for FB2 and EPUB. The spike found that outside GNOME and KDE `xdg-open` types files by content, so `.fb2`
opens in nvim on Omarchy unless `perl-file-mimeinfo` is present.

## What Changes

- AUR packages: `forzac` (builds from source) and `forzac-bin` (release binaries, x86_64 and aarch64); depends on
  `gtk4`, `fontconfig`, `perl-file-mimeinfo`; assets in `/usr/share/forzac`.
- `.desktop` file (`app.truenrth.forzac`), a MIME type for FB3, the matching Wayland app id; a post-install hint with
  the `xdg-mime default` command.
- One process per user: opening another book while Forzac runs opens a new window in the same process.
- Tiling compositors: no client-side title bar on Hyprland; sensible initial size. Checked on labwc (Pi 5) and headless
  sway; Hyprland itself is unverified (no hardware for it).
- Icon (from the Android app's vector icon), help on `?` with the single "support" line.
- A Debian package for Raspberry Pi OS (arm64), built by CI.

## Capabilities

### New Capabilities
- `linux-integration`: packaging, desktop integration, default-reader behaviour, single instance.

### Modified Capabilities
- `project-infrastructure`: the release artefacts gain the Debian package and the icons.

## Impact

`data/`, packaging files, CI release jobs; small changes in `gtk/src/main.rs`.

## Non-goals

Flatpak and Snap for 1.0; following Omarchy theme colours.
