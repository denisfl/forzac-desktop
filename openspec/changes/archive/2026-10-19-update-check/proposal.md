## Why

Forzac for desktop 0.9.0 is out on GitHub, the macOS disk image and the Linux archives with stable links from
forzac.truenrth.app/desktop/. Nothing tells a reader that a newer version exists: the disk image and the archives have
no update channel of their own. The packages (AUR, the `.deb`) do: their package managers update them.

## What Changes

- Once a day at most, when the library opens, Forzac asks GitHub for the latest release
  (`https://api.github.com/repos/denisfl/forzac-desktop/releases/latest`) in the background and compares its tag with
  its own version. The request carries nothing about the books or the reader, and not Forzac's version.
- When the release is newer, the library's head shows one quiet line: "Version 0.9.1 is out — download", the last
  word a link to https://forzac.truenrth.app/desktop/, and a "×" that hides it until a still newer version comes out.
- No download, no installer, no setting, no dialog. Offline, a failed request or an unexpected answer change nothing.
- Builds installed by a package manager do not check: the AUR packages and the `.deb` install a marker file,
  `share/forzac/packaged`, next to the fonts; a GTK build that finds it never asks. macOS always checks.
- On Linux the request goes through the system's `curl` (no new library); without `curl` there is no check.

The maintainer decided (2026-10-19): a silent daily check, no question at first start and no switch; only the
release archives check on Linux; the line in the library only; the library's privacy note and the `?` help stay as
they are (no data leaves the computer).

## Non-Goals

Downloading or installing updates; release notes in the app; checking pre-releases; a setting or a key to turn the
check off; showing the line in a book window; telling the package builds about updates.

## Capabilities

### New Capabilities
- `update-check`: when and how Forzac asks for the latest release, what it remembers, the line in the library, the
  builds that never check.

### Modified Capabilities

## Impact

Core: `update` module (the URLs, the version, when a check is due, reading the answer, comparing versions, the line's
text), three settings keys in the state file, two strings in the catalogue, C ABI for macOS. GTK: the request through
`gio::Subprocess` and `curl`, the line in the library's head, the marker check. macOS: `URLSession`, the line in the
library's head. Packaging: the marker in both PKGBUILDs and `scripts/package-deb.sh`. README: a sentence for
packagers.
