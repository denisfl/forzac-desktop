## Context

`macos/build.sh` builds `macos/build/Forzac.app`: the Swift binary is only linker-signed (`flags=adhoc,linker-signed`,
identifier `Forzac`), the bundle's resources are not sealed, and there is no icon. The release workflow zips the app.
Without a Developer ID, any download of Forzac carries the quarantine attribute and Gatekeeper refuses the first open:
macOS 14 still offers Control-click › Open, macOS 15 sends the reader to System Settings › Privacy & Security. The Mac
at hand runs macOS 14.5; there is no macOS 15 machine.

## Goals / Non-Goals

**Goals:** a DMG that a reader can check, install and open on macOS 14 and 15 by following one page; the same build
locally and in CI; the app icon.

**Non-Goals:** a Developer ID, notarisation, the App Store, Homebrew, automatic updates, an Intel build, a styled DMG
window (background picture, icon positions).

## Decisions

**`build.sh` signs the bundle ad hoc.** After the bundle is assembled, `codesign --force --sign -
--identifier app.truenrth.forzac Forzac.app` seals the resources (fonts, licences, icon) and gives the signature the
bundle identifier. Every build is signed, so the app tried during development is the app shipped. No hardened runtime
or entitlements: they only matter for notarisation. The icon is `macos/Forzac.icns` (made by `scripts/icons.py`,
committed) copied to `Resources`, with `CFBundleIconFile` in `Info.plist`.

**`scripts/package-macos.sh X.Y.Z` makes the DMG and the checksum.** It sets `CFBundleShortVersionString` in the built
bundle with `PlistBuddy` (the committed `Info.plist` keeps the development version), signs again (the plist is
sealed), stages the app with an `Applications` symlink, and runs `hdiutil create -format UDZO -volname Forzac`. Then
`shasum -a 256 Forzac-X.Y.Z-macos-arm64.dmg > Forzac-X.Y.Z-macos-arm64.dmg.sha256`, the format `shasum -c` reads.
The image is not signed: a signature on the DMG adds nothing without a Developer ID.

Alternative considered: `create-dmg` or an AppleScript-styled window. Extra tools for looks; a plain window with two
icons is clear enough.

**CI.** The release workflow's macOS job runs `macos/build.sh` and `scripts/package-macos.sh "${GITHUB_REF_NAME#v}"`
and uploads the DMG and the `.sha256` instead of the zip.

**The first-open page is Markdown in the repository** (`docs/macos-first-open.md`), written for the download page of
forzac.truenrth.app, which is a separate project; publishing it there is part of the release, not of this change. It
gives the steps for macOS 14 (Control-click › Open) and for macOS 15+ (try to open, Done, System Settings › Privacy &
Security › Open Anyway, confirm with the password), the reason in one sentence, the checksum command and the Terminal
alternative. Screenshots of the macOS 14 and 15 steps are taken when the page is published (`release-1-0`): this
session may not capture the screen and there is no macOS 15 Mac; the steps are text first, so the page works without
them.

## Risks / Trade-offs

- The first-open step may put readers off → the proposal's revisit clause; the page keeps it to a few steps.
- macOS 15 steps are written from Apple's documentation, not tried here → the "fresh Mac user account" check by the
  maintainer, and a macOS 15 check before publishing.
- Apple may tighten Gatekeeper further (as it did in 15) → the page names the macOS versions it was checked on.
