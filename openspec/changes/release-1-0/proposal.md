## Why

1.0 is a promise of quality: fast to open, smooth to turn, quiet on battery, on every supported platform. The spike
measured these once; the release checks them against fixed budgets on the reference hardware and publishes the
packages.

## What Changes

- Performance budgets as requirements, on the test hardware in `docs/roadmap.md`: first page after opening a book
  (warm) under 200 ms on the Pi 5 and under 150 ms on the M1 Mac; 0 missed frames inside page-turn animations at
  120 Hz on the Pi 5's headless output and at 60 Hz on the Mac's display; 0 CPU ticks over 10 s while idle on both;
  layout of a chapter under 50 ms on the Pi 5.
- x86_64 Linux and Hyprland are not measured on hardware; the release notes say so and ask Omarchy users for reports.
- The measurement scripts (`scripts/measure.sh`, `scripts/pi-headless.sh`) produce the evidence attached to the
  release.
- Release notes, version 1.0.0, packages published (AUR, Raspberry Pi OS `.deb`, the macOS DMG on forzac.truenrth.app).
- Before the DMG is published (left over from `macos-release`): the maintainer opens the downloaded DMG on a fresh Mac
  user account following `docs/macos-first-open.md`, including the Gatekeeper step on a quarantined copy; the steps
  are checked on a macOS 15 Mac; screenshots of the macOS 14 and 15 steps are added to the download page.
- Publishing the AUR packages: `updpkgsums` and `.SRCINFO` from the tag's archives (`packaging/aur/`).

## Capabilities

### New Capabilities
- `performance`: the budgets above and how they are measured.

### Modified Capabilities
<!-- None. -->

## Impact

No new features. Fixes found by the measurements land in this change or in their own.
