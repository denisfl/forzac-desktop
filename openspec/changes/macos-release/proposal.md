## Why

The Mac app is downloaded from the project site (forzac.truenrth.app). There is no Apple Developer ID and none is
planned, and there is no App Store release (decided 2026-10-16). Without notarisation Gatekeeper blocks the first open;
since macOS 15 a Control-click "Open" no longer gets past it, and the user has to allow the app in System Settings →
Privacy & Security → "Open Anyway". The download has to make that step clear. Homebrew is dropping casks that fail
Gatekeeper, so there is no cask.

## What Changes

- An ad-hoc signed `.app` (`codesign --sign -`, which Apple Silicon requires for a bundle to run) in a DMG, built by
  a script and a CI job; a SHA-256 checksum published next to it.
- First-open instructions for the download page, with screenshots for macOS 14 and 15+.

## Capabilities

### Modified Capabilities
- `macos-app`: distribution.

## Impact

`macos/build.sh` or a new packaging script, a CI release job, text for the site.

## Non-goals

A Developer ID, notarisation, the App Store, Homebrew, automatic updates. Revisit if the first-open step turns out to
put readers off.
