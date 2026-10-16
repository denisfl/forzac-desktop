## Why

The macOS shell from the spike draws pages and turns them. It has to reach the Linux shell's features and ship as a
proper Mac app, which needs signing and notarisation to open without warnings.

## What Changes

- Feature parity with the Linux shell for `reading-state`, `reader-navigation`, `page-turning`, `reader-settings`,
  `book-search`: panels in AppKit, keys by key code (work in any layout), standard menus with the same commands.
- Trackpad: swipe to turn pages following the fingers, two-finger scroll in "Scroll" mode.
- Open from Finder and "Open With", document types for FB2/FB3/EPUB, recent documents.
- The C ABI replaced by UniFFI-generated Swift bindings if the API has grown past a handful of calls (decided in
  design).
- A signed and notarised `.app` (a Developer ID is needed), a DMG and a Homebrew cask built by CI.

## Capabilities

### New Capabilities
- `macos-app`: Mac-specific behaviour, packaging and distribution.

### Modified Capabilities
- `page-turning`, `reader-navigation`, `reader-settings`, `book-search`, `reading-state`: macOS key equivalents where
  the Mac convention differs (`⌘F` for search, `⌘W` to close).

## Impact

`macos/` grows into an Xcode-less SwiftPM package; CI gets a signing job. Requires an Apple Developer account.
