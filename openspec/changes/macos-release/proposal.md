## Why

An unsigned app asks for confirmation on first open and cannot be distributed through Homebrew comfortably. A Mac
release needs a signed and notarised app, a disk image and a cask.

## What Changes

- Signing with a Developer ID and notarisation; a DMG; a Homebrew cask; a CI job that builds them.

## Capabilities

### Modified Capabilities
- `macos-app`: distribution.

## Impact

Requires an Apple Developer account and its certificates as CI secrets.
