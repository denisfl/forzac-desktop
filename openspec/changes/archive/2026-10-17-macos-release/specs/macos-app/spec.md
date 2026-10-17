## ADDED Requirements

### Requirement: Distribution
The Mac app SHALL be distributed as a disk image `Forzac-X.Y.Z-macos-arm64.dmg` downloaded from forzac.truenrth.app,
without a Developer ID, notarisation, the App Store or Homebrew. The image SHALL hold `Forzac.app` and a link to
`/Applications` for dragging it there, under the volume name "Forzac". The app SHALL be signed ad hoc as a whole
bundle (its resources sealed, so `codesign --verify --deep --strict` passes), with the bundle identifier
`app.truenrth.forzac`, the release's version in `CFBundleShortVersionString`, and the app icon. A file
`Forzac-X.Y.Z-macos-arm64.dmg.sha256` in `shasum -a 256` format SHALL be published next to the image.

#### Scenario: Checking a download
- **WHEN** the reader runs `shasum -a 256 -c Forzac-0.2.0-macos-arm64.dmg.sha256` next to the downloaded image
- **THEN** it reports `OK`

#### Scenario: Installing
- **WHEN** the reader opens the image and drags Forzac onto the Applications link
- **THEN** Forzac is in `/Applications` with its icon, and after the first-open step it starts and opens books

### Requirement: App icon
The app icon SHALL be the Android app's launcher icon (the marbled endpaper and the page edge) on the macOS icon grid:
a rounded square with continuous corners, 824 pixels on a 1024 pixel canvas, with a soft shadow, in every size an
`.icns` holds from 16 to 1024 pixels. It SHALL be generated from the Android app's vector drawables by the script
that makes the Linux icons.

#### Scenario: The Dock
- **WHEN** Forzac runs
- **THEN** its Dock icon has the same size and shape as the system apps' icons next to it

### Requirement: First-open instructions
The repository SHALL hold the text for the download page that explains, for macOS 14 and for macOS 15 and later, how
to open Forzac the first time despite Gatekeeper: on macOS 14 Control-click › Open › Open; on macOS 15 and later
System Settings › Privacy & Security › "Open Anyway" after the first attempt, then "Open Anyway" and the password.
It SHALL say why (the app is free and not notarised by Apple), that the step is needed once, how to check the
checksum, and the Terminal alternative (`xattr -dr com.apple.quarantine /Applications/Forzac.app`).

#### Scenario: A reader on macOS 15
- **WHEN** a reader on macOS 15 opens Forzac from the image and macOS says it was not opened
- **THEN** following the page's macOS 15 steps opens Forzac, and later opens need no step
