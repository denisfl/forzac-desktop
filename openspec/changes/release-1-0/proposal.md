## Why

1.0 is a promise of quality: fast to open, smooth to turn, quiet on battery, on every supported platform. The spike
measured these once; the release checks them against fixed budgets on the reference hardware and publishes the
packages.

## What Changes

- Performance budgets as requirements: first page after opening a book (warm) under 200 ms on the Pi 5 and under
  150 ms on an Omarchy laptop and a Mac; 0 missed frames inside page-turn animations at 120 Hz; 0 CPU ticks over 10 s
  while idle; layout of a chapter under 50 ms on the Pi 5.
- The measurement scripts (`scripts/measure.sh`, `scripts/pi-headless.sh`) produce the evidence attached to the
  release.
- Release notes, version 1.0.0, packages published (AUR, Raspberry Pi OS `.deb`, Homebrew cask).

## Capabilities

### New Capabilities
- `performance`: the budgets above and how they are measured.

### Modified Capabilities
<!-- None. -->

## Impact

No new features. Fixes found by the measurements land in this change or in their own.
